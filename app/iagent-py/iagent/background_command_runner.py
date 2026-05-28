"""Background command execution service for non-disruptive task automation."""

from __future__ import annotations

import asyncio
import logging
import os
import time
from pathlib import Path
from uuid import uuid4

from PySide6.QtCore import QObject, Signal

logger = logging.getLogger(__name__)


class BackgroundCommandRunner(QObject):
    """Run shell commands asynchronously and emit compact completion events."""

    command_started = Signal(str, str)
    command_running = Signal(str, str)
    command_finished = Signal(str, str, int, str, str, float)
    command_failed = Signal(str, str, str)

    def __init__(
        self,
        *,
        default_cwd: Path | None = None,
        max_output_chars: int = 4000,
        parent: QObject | None = None,
    ) -> None:
        super().__init__(parent)
        self._default_cwd = default_cwd
        self._max_output_chars = max_output_chars

    def enqueue(self, command: str, *, cwd: Path | None = None) -> str:
        """Queue a shell command and return its task id immediately."""
        task_id = uuid4().hex[:8]
        command_text = command.strip()
        if not command_text:
            raise ValueError("command cannot be empty")

        self.command_started.emit(task_id, command_text)
        asyncio.ensure_future(self._run(task_id, command_text, cwd))
        return task_id

    def enqueue_exec(
        self,
        argv: list[str],
        *,
        cwd: Path | None = None,
        display_command: str | None = None,
        env: dict[str, str] | None = None,
    ) -> str:
        """Queue a non-shell command with explicit argv."""
        if not argv or not argv[0].strip():
            raise ValueError("argv cannot be empty")
        task_id = uuid4().hex[:8]
        shown = display_command or " ".join(argv)
        self.command_started.emit(task_id, shown)
        asyncio.ensure_future(self._run_exec(task_id, argv, shown, cwd, env))
        return task_id

    async def _run(self, task_id: str, command: str, cwd: Path | None) -> None:
        workdir = str(cwd or self._default_cwd or Path.home())
        started_at = time.perf_counter()
        try:
            process = await asyncio.create_subprocess_shell(
                command,
                cwd=workdir,
                stdout=asyncio.subprocess.PIPE,
                stderr=asyncio.subprocess.PIPE,
            )
            self.command_running.emit(task_id, command)
            stdout_b, stderr_b = await process.communicate()
            elapsed = time.perf_counter() - started_at

            stdout_text = self._truncate(stdout_b.decode("utf-8", errors="replace"))
            stderr_text = self._truncate(stderr_b.decode("utf-8", errors="replace"))
            return_code = process.returncode if process.returncode is not None else -1

            logger.info(
                "background command finished id=%s exit=%s cmd=%s",
                task_id,
                return_code,
                command,
            )
            self.command_finished.emit(
                task_id,
                command,
                return_code,
                stdout_text,
                stderr_text,
                elapsed,
            )
        except Exception as exc:  # noqa: BLE001
            logger.exception("background command failed id=%s cmd=%s", task_id, command)
            self.command_failed.emit(task_id, command, str(exc))

    async def _run_exec(
        self,
        task_id: str,
        argv: list[str],
        shown_command: str,
        cwd: Path | None,
        extra_env: dict[str, str] | None,
    ) -> None:
        workdir = str(cwd or self._default_cwd or Path.home())
        started_at = time.perf_counter()
        env = None
        if extra_env is not None:
            env = {**os.environ, **extra_env}
        try:
            process = await asyncio.create_subprocess_exec(
                *argv,
                cwd=workdir,
                stdout=asyncio.subprocess.PIPE,
                stderr=asyncio.subprocess.PIPE,
                env=env,
            )
            self.command_running.emit(task_id, shown_command)
            stdout_b, stderr_b = await process.communicate()
            elapsed = time.perf_counter() - started_at

            stdout_text = self._truncate(stdout_b.decode("utf-8", errors="replace"))
            stderr_text = self._truncate(stderr_b.decode("utf-8", errors="replace"))
            return_code = process.returncode if process.returncode is not None else -1

            logger.info(
                "background exec finished id=%s exit=%s argv0=%s",
                task_id,
                return_code,
                argv[0],
            )
            self.command_finished.emit(
                task_id,
                shown_command,
                return_code,
                stdout_text,
                stderr_text,
                elapsed,
            )
        except Exception as exc:  # noqa: BLE001
            logger.exception("background exec failed id=%s argv=%s", task_id, argv)
            self.command_failed.emit(task_id, shown_command, str(exc))

    def _truncate(self, text: str) -> str:
        text = text.strip()
        if len(text) <= self._max_output_chars:
            return text
        keep = self._max_output_chars // 2
        return (
            f"{text[:keep].rstrip()}\n...\n"
            f"(truncated {len(text) - 2 * keep} chars)\n...\n{text[-keep:].lstrip()}"
        )
