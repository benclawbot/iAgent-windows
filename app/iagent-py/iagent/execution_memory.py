"""Persistent execution memory for command-routing and recovery hints."""

from __future__ import annotations

import json
import logging
from pathlib import Path

logger = logging.getLogger(__name__)


class ExecutionMemory:
    """Stores lightweight lessons so the assistant avoids repeating failures."""

    def __init__(self, path: Path, *, max_lessons: int = 16) -> None:
        self._path = path
        self._max_lessons = max(1, max_lessons)
        self._lessons: list[str] = []
        self._load()

    def lessons_for_prompt(self, *, limit: int = 6) -> list[str]:
        if limit <= 0:
            return []
        return self._lessons[-limit:]

    def record_user_feedback(self, text: str) -> None:
        cleaned = text.strip().lower()
        if not cleaned:
            return

        failure_markers = (
            "didn't open",
            "didnt open",
            "did not open",
            "not opened",
            "failed to open",
        )
        if "edge" in cleaned and any(marker in cleaned for marker in failure_markers):
            self._add_lesson(
                "user reported edge did not open; for browser launch requests, "
                "prefer a direct windows command tag like [CMD:start microsoft-edge:] "
                "instead of retrying the same generic workflow."
            )
        if ".xpi" in cleaned and any(
            token in cleaned for token in ("gmail", "compose", "draft", "email")
        ):
            self._add_lesson(
                "user reported an email flow tried to open a .xpi extension package; "
                "for gmail drafting requests, use a direct gmail compose url command "
                "and avoid extension/package launch commands."
            )
        if (
            ("powerpoint" in cleaned or "slideshow" in cleaned or "presentation" in cleaned)
            and any(
                marker in cleaned
                for marker in (
                    "didn't create",
                    "didnt create",
                    "did not create",
                    "not created",
                    "opens the app but stops",
                    "opens app but stops",
                )
            )
        ):
            self._add_lesson(
                "user reported powerpoint opened without finishing creation; "
                "for slideshow requests, build and save the presentation in "
                "background first, then open the completed file."
            )
        if any(marker in cleaned for marker in ("opens the app but stops", "opens app but stops")):
            self._add_lesson(
                "for multi-step creation tasks, do the real build in background first "
                "and only open the app on the finished artifact."
            )
        if (
            ("word" in cleaned or "document" in cleaned or "docx" in cleaned)
            and any(
                marker in cleaned
                for marker in (
                    "didn't create",
                    "didnt create",
                    "did not create",
                    "not created",
                    "opens the app but stops",
                    "opens app but stops",
                )
            )
        ):
            self._add_lesson(
                "user reported word opened without finishing document creation; "
                "for word requests, build and save the document in background first, "
                "then open the completed file."
            )
        if (
            (
                "excel" in cleaned
                or "spreadsheet" in cleaned
                or "workbook" in cleaned
                or "xlsx" in cleaned
            )
            and any(
                marker in cleaned
                for marker in (
                    "didn't create",
                    "didnt create",
                    "did not create",
                    "not created",
                    "opens the app but stops",
                    "opens app but stops",
                )
            )
        ):
            self._add_lesson(
                "user reported excel opened without finishing workbook creation; "
                "for spreadsheet requests, build and save the workbook in background "
                "first, then open the completed file."
            )

    def record_command_outcome(
        self,
        *,
        command: str,
        exit_code: int,
        stdout_text: str,
        stderr_text: str,
    ) -> None:
        cmd = command.strip().lower()
        stderr_l = (stderr_text or "").lower()

        if cmd.startswith("open -a "):
            self._add_lesson(
                "this machine is windows. do not use macos open -a for launching apps; "
                "use windows start forms instead."
            )

        if cmd.startswith("start ") and exit_code == 0:
            self._add_lesson(
                "windows app/url launches succeeded with start; prefer start for "
                "local app and browser launch commands."
            )

        if exit_code != 0 and "is not recognized as an internal or external command" in stderr_l:
            self._add_lesson(
                "a previous shell command failed because the command name was not "
                "recognized; choose commands that exist on windows."
            )

        if cmd.startswith("jcode run") and exit_code == 0:
            stdout_l = (stdout_text or "").lower()
            if "unable to open" in stdout_l or "cannot open" in stdout_l:
                self._add_lesson(
                    "a previous jcode workflow reported it could not open an app; "
                    "for local app launch requests, prefer an explicit windows "
                    "[CMD:...] action."
                )

    def record_command_crash(self, *, command: str, error_text: str) -> None:
        if command.strip():
            self._add_lesson(
                "a previous background command crashed unexpectedly; when possible, "
                "use simpler windows-native launch commands."
            )
        if error_text.strip():
            logger.debug("command crash recorded: %s", error_text[:200])

    def _add_lesson(self, lesson: str) -> None:
        text = " ".join(lesson.strip().split())
        if not text:
            return
        if text in self._lessons:
            return
        self._lessons.append(text)
        if len(self._lessons) > self._max_lessons:
            self._lessons = self._lessons[-self._max_lessons :]
        self._save()

    def _load(self) -> None:
        try:
            raw = self._path.read_text(encoding="utf-8")
        except FileNotFoundError:
            return
        except OSError as exc:
            logger.warning("failed to read execution memory %s: %s", self._path, exc)
            return

        try:
            payload = json.loads(raw)
        except json.JSONDecodeError:
            logger.warning("execution memory is not valid json: %s", self._path)
            return

        values = payload.get("lessons", [])
        if not isinstance(values, list):
            return
        self._lessons = [
            " ".join(str(item).strip().split())
            for item in values
            if isinstance(item, str) and item.strip()
        ][-self._max_lessons :]

    def _save(self) -> None:
        try:
            self._path.parent.mkdir(parents=True, exist_ok=True)
            payload = {"lessons": self._lessons}
            self._path.write_text(
                json.dumps(payload, ensure_ascii=True, indent=2) + "\n",
                encoding="utf-8",
            )
        except OSError as exc:
            logger.warning("failed to save execution memory %s: %s", self._path, exc)
