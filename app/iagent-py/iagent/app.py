"""iAgent QApplication bootstrap.

Resolves the config file path via platformdirs, ensures the file exists
(creating from config.example.toml on first run), loads it, and holds
the resulting Config for downstream components to read.
"""

from __future__ import annotations

import asyncio
import logging
import os
import re
import shutil
import signal
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path

import qasync
from platformdirs import user_config_dir, user_log_dir
from PySide6.QtCore import QTimer
from PySide6.QtWidgets import QApplication, QInputDialog, QLineEdit, QMessageBox

from iagent.background_command_runner import BackgroundCommandRunner
from iagent.clients.llm_client import LLMClient
from iagent.clients.transcription_client import TranscriptionClient
from iagent.clients.tts_client import TTSClient
from iagent.command_policy import (
    GMAIL_COMPOSE_COMMAND,
    normalize_ai_command,
    requires_manual_approval,
)
from iagent.companion_manager import CompanionManager
from iagent.config import Config, ConfigError
from iagent.execution_memory import ExecutionMemory
from iagent.hotkey import HotkeyMonitor
from iagent.logging_config import configure_logging
from iagent.mic_capture import MicCapture
from iagent.screen_capture import capture_all
from iagent.state import VoiceState
from iagent.ui.history_window import HistoryWindow
from iagent.ui.proposal_popup import ProposalPopupController
from iagent.ui.task_inbox import TaskInboxController
from iagent.ui.tray_icon import TrayIcon

APP_NAME = "iAgent"
APP_AUTHOR = "iAgent"

logger = logging.getLogger(__name__)


@dataclass
class BootstrapResult:
    app: QApplication
    config: Config | None
    config_error: ConfigError | None
    was_first_run: bool
    config_path: Path
    log_dir: Path


class _NoOpPanelVisibilityController:
    """Disables floating cursor visuals while keeping capture hooks stable."""

    def hide_for_capture(self) -> None:
        return

    def restore_after_capture(self) -> None:
        return

    def fly_to(self, _x: int, _y: int) -> None:
        return


def _example_config_path() -> Path:
    # config.example.toml sits next to the iagent package directory
    # (i.e. inside iagent-py/, alongside iagent/).
    return Path(__file__).resolve().parent.parent / "config.example.toml"


def _project_root() -> Path:
    return Path(__file__).resolve().parents[2]


def _candidate_submodule_iagent_paths() -> list[Path]:
    backend_dir = _project_root() / "backend" / "iagent"
    return [
        backend_dir / "target" / "release" / "iagent.exe",
        backend_dir / "target" / "release" / "iagent",
        backend_dir / "target" / "debug" / "iagent.exe",
        backend_dir / "target" / "debug" / "iagent",
    ]


def _resolve_iagent_executable(config: Config | None) -> Path | None:
    env_override = os.environ.get("IAGENT_BIN", "").strip()
    if env_override:
        env_path = Path(env_override).expanduser()
        if env_path.is_file():
            return env_path.resolve()

    if config is not None and config.iagent_path is not None and config.iagent_path.is_file():
        return config.iagent_path

    in_path = shutil.which("iagent")
    if in_path:
        return Path(in_path).resolve()

    for candidate in _candidate_submodule_iagent_paths():
        if candidate.is_file():
            return candidate.resolve()
    return None


def bootstrap(argv: list[str] | None = None) -> BootstrapResult:
    # Qt 6 sets PROCESS_PER_MONITOR_DPI_AWARE_V2 internally during
    # QApplication init. Calling SetProcessDpiAwareness ourselves is
    # redundant and raises "Access is denied" if Qt gets there first.
    # mss captures at raw physical pixels regardless, so no explicit call
    # is needed.

    argv = argv if argv is not None else sys.argv
    app = QApplication(argv)
    app.setApplicationName(APP_NAME)
    app.setOrganizationName(APP_AUTHOR)
    app.setQuitOnLastWindowClosed(False)  # tray app: closing panel must not quit

    # Pass appauthor=False so platformdirs does NOT nest a redundant
    # second "iAgent" folder inside the first (which happens when
    # appname == appauthor on Windows). We want %APPDATA%\iAgent\config.toml,
    # not %APPDATA%\iAgent\iAgent\config.toml.
    config_dir = Path(user_config_dir(APP_NAME, appauthor=False, roaming=True))
    config_path = config_dir / "config.toml"
    log_dir = Path(user_log_dir(APP_NAME, appauthor=False))

    was_first_run = Config.ensure_exists(config_path, _example_config_path())

    try:
        config = Config.from_path(config_path)
        config_error = None
    except ConfigError as exc:
        config = None
        config_error = exc

    return BootstrapResult(
        app=app,
        config=config,
        config_error=config_error,
        was_first_run=was_first_run,
        config_path=config_path,
        log_dir=log_dir,
    )


def run() -> int:
    """Start the iAgent tray app and run the Qt event loop.

    Wires together the tray icon, floating panel, global hotkey
    monitor, and CompanionManager. Blocks until the user quits
    via the tray menu.
    """
    result = bootstrap()

    log_level = result.config.log_level if result.config else "INFO"
    configure_logging(result.log_dir, log_level)

    if result.was_first_run:
        logger.info("first run: created config at %s", result.config_path)

    if result.config_error is not None:
        logger.warning("config error: %s", result.config_error)

    tray_icon = TrayIcon()
    history = HistoryWindow()

    if result.config_error is not None:
        tray_icon.notify(
            "iAgent Config Error",
            f"{result.config_error}. Open tray menu > Settings to edit config.toml.",
        )

    mic = MicCapture()
    panel_visibility = _NoOpPanelVisibilityController()

    mic.error.connect(lambda msg: logger.error("mic error: %s", msg))

    # Tray menu -> settings / history
    tray_icon.show_settings_requested.connect(
        lambda: os.startfile(result.config_path)
    )
    tray_icon.show_history_requested.connect(history.show)
    tray_icon.show_history_requested.connect(history.raise_)

    command_runner = BackgroundCommandRunner(default_cwd=Path.home())
    task_inbox = TaskInboxController()
    proposal_popups = ProposalPopupController()
    execution_memory = ExecutionMemory(result.config_path.parent / "execution_memory.json")
    tray_icon.show_prompt_dock_requested.connect(task_inbox.show_prompt_dock)
    ambient_process: subprocess.Popen | None = None
    _close_in_progress = False

    def _ambient_is_running() -> bool:
        nonlocal ambient_process
        if ambient_process is None:
            return False
        if ambient_process.poll() is not None:
            ambient_process = None
            return False
        return True

    def _stop_ambient_mode(notify: bool = True) -> None:
        nonlocal ambient_process
        if not _ambient_is_running():
            return
        assert ambient_process is not None  # narrowed by _ambient_is_running
        ambient_process.terminate()
        try:
            ambient_process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            ambient_process.kill()
        ambient_process = None
        if notify:
            tray_icon.notify("iAgent Ambient", "Ambient mode stopped.")

    def _start_ambient_mode() -> None:
        nonlocal ambient_process
        if _ambient_is_running():
            tray_icon.notify("iAgent Ambient", "Ambient mode is already running.")
            return
        iagent_bin = _resolve_iagent_executable(result.config)
        if iagent_bin is None:
            tray_icon.notify(
                "iAgent Ambient",
                "iAgent not found. Build backend/iagent or set iagent_path.",
            )
            return
        try:
            ambient_process = subprocess.Popen(
                [str(iagent_bin), "ambient", "desktop", "--headless"],
                cwd=Path.home(),
                creationflags=getattr(subprocess, "CREATE_NO_WINDOW", 0),
            )
        except OSError as exc:
            tray_icon.notify("iAgent Ambient Error", str(exc))
            return
        tray_icon.notify("iAgent Ambient", "Ambient mode started.")

    def _stop_related_processes() -> None:
        """Best-effort stop of sibling iAgent/worker processes started externally."""
        import subprocess
        worker_dir = str(Path(__file__).resolve().parent.parent.parent / "worker")
        script = (
            f"$workerDir='{worker_dir}'; "
            f"$selfPid={os.getpid()}; "
            "Get-CimInstance Win32_Process | "
            "Where-Object { "
            "($_.Name -eq 'node.exe' -and $_.CommandLine -like '*wrangler dev*' "
            "-and $_.CommandLine -like \"*$workerDir*\") "
            "-or "
            "($_.Name -eq 'uv.exe' "
            "-and $_.CommandLine -match 'run\\s+python(\\.exe)?\\s+-m\\s+iagent') "
            "-or "
            "(($_.Name -eq 'python.exe' -or $_.Name -eq 'pythonw.exe') "
            "-and ($_.CommandLine -match 'uv(\\.exe)?\\s+run\\s+python(\\.exe)?\\s+-m\\s+iagent' "
            "-or $_.CommandLine -match 'python(\\.exe)?\\s+-m\\s+iagent') "
            "-and $_.ProcessId -ne $selfPid) "
            "} | "
            "ForEach-Object { Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue }"
        )
        # Launch async without waiting - Fire-and-forget
        try:
            subprocess.Popen(
                ["powershell.exe", "-NoProfile", "-Command", script],
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
                creationflags=subprocess.CREATE_NO_WINDOW,
            )
        except Exception:
            pass

    def _close_application_requested() -> None:
        nonlocal _close_in_progress
        if _close_in_progress:
            return
        _close_in_progress = True
        # Stop ambient mode in a non-blocking way
        if _ambient_is_running():
            assert ambient_process is not None
            try:
                ambient_process.terminate()
            except Exception:
                pass
            ambient_process = None
        _stop_related_processes()
        # Exit immediately without waiting
        QTimer.singleShot(50, lambda: os._exit(0))
    task_inbox.close_all_requested.connect(_close_application_requested)
    tray_icon.quit_requested.connect(_close_application_requested)

    def _is_command_approved(command: str, *, source_label: str) -> bool:
        if not requires_manual_approval(command):
            return True

        answer = QMessageBox.question(
            None,
            "Approve rm Command",
            (
                f"{source_label} requested a command containing 'rm':\n\n"
                f"{command}\n\n"
                "Allow this command?"
            ),
            QMessageBox.StandardButton.Yes | QMessageBox.StandardButton.No,
            QMessageBox.StandardButton.No,
        )
        return answer == QMessageBox.StandardButton.Yes

    def _queue_background_command(command: str, *, source_label: str) -> str | None:
        cleaned = command.strip()
        if not cleaned:
            return None
        command_to_run = cleaned
        if source_label == "AI":
            normalized, note = normalize_ai_command(cleaned)
            if normalized is None:
                logger.warning("blocked ai command: %s (%s)", cleaned, note or "no reason")
                tray_icon.notify(
                    "iAgent",
                    "Blocked an invalid AI browser command. Please retry the request.",
                )
                return None
            if normalized != cleaned:
                logger.info("rewrote ai command: %s -> %s (%s)", cleaned, normalized, note)
                tray_icon.notify("iAgent", "Opening Gmail compose in your default browser.")
            command_to_run = normalized

        if not _is_command_approved(command_to_run, source_label=source_label):
            tray_icon.notify("iAgent", f"Blocked {source_label} rm command.")
            return None
        try:
            task_id = command_runner.enqueue(command_to_run)
        except ValueError as exc:
            tray_icon.notify("iAgent Task Error", str(exc))
            return None
        tray_icon.notify("iAgent", f"Queued task {task_id}: {command_to_run[:120]}")
        return task_id

    def _office_builder_script_for_goal(goal: str) -> tuple[str, Path] | None:
        goal_l = goal.lower()
        creation_tokens = ("create", "build", "make", "draft", "generate", "prepare", "write")
        if not any(token in goal_l for token in creation_tokens):
            return None

        script_dir = Path(__file__).resolve().parent
        office_builders: tuple[tuple[str, tuple[str, ...], str], ...] = (
            (
                "PowerPoint",
                ("powerpoint", "ppt", "pptx", "slideshow", "presentation", "slides"),
                "create_powerpoint_from_goal.ps1",
            ),
            (
                "Word",
                ("microsoft word", "word doc", "word document", "docx", ".docx"),
                "create_document_from_goal.py",
            ),
            (
                "Excel",
                ("excel", "xlsx", "spreadsheet", "workbook", "worksheet"),
                "create_excel_from_goal.ps1",
            ),
        )
        for app_name, keywords, script_name in office_builders:
            if any(keyword in goal_l for keyword in keywords):
                return app_name, script_dir / script_name
        return None

    def _queue_office_goal(goal: str, *, source_label: str) -> str | None:
        goal_clean = goal.strip()
        if not goal_clean:
            return None
        builder = _office_builder_script_for_goal(goal_clean)
        if builder is None:
            return None
        goal_l = goal_clean.lower()
        explicit_open = any(
            token in goal_l
            for token in (
                "open when done",
                "open it",
                "open the",
                "open in word",
                "open in excel",
                "open in powerpoint",
                "and open",
                "then open",
            )
        )
        explicit_no_open = any(
            token in goal_l
            for token in ("don't open", "do not open", "without opening", "no open")
        )
        # Default behavior for Office artifact requests is to open the result
        # immediately so the user can review it.
        open_when_done = explicit_open or not explicit_no_open
        app_name, script_path = builder
        if not script_path.is_file():
            tray_icon.notify(
                "iAgent Task Error",
                f"{app_name} builder script missing: {script_path}",
            )
            return None
        title_hint_match = re.search(r'titled\\s+"([^"]+)"', goal_clean, re.IGNORECASE)
        title_hint = title_hint_match.group(1) if title_hint_match else "Office Artifact"
        if script_path.suffix.lower() == ".py":
            display = f"python -m iagent.{script_path.stem} <goal: {title_hint[:60]}>"
            args = [
                sys.executable,
                "-m",
                f"iagent.{script_path.stem}",
                "--goal",
                goal_clean,
            ]
            if open_when_done:
                args.append("--open-when-done")
        else:
            display = f"powershell -File {script_path.name} <goal: {title_hint[:60]}>"
            args = [
                "powershell.exe",
                "-NoProfile",
                "-ExecutionPolicy",
                "Bypass",
                "-File",
                str(script_path),
                "-Goal",
                goal_clean,
            ]
            if open_when_done:
                args.append("-OpenWhenDone")
        try:
            task_id = command_runner.enqueue_exec(
                args,
                cwd=Path.home(),
                display_command=display,
            )
        except ValueError as exc:
            tray_icon.notify("iAgent Task Error", str(exc))
            return None
        tray_icon.notify("iAgent", f"Queued {source_label} {app_name} task {task_id}.")
        return task_id

    def _queue_jcode_goal(goal: str, *, source_label: str) -> str | None:
        goal_clean = goal.strip()
        if not goal_clean:
            return None
        goal_l = goal_clean.lower()
        if (
            "gmail" in goal_l
            and any(token in goal_l for token in ("compose", "draft", "write", "email"))
        ):
            return _queue_background_command(
                GMAIL_COMPOSE_COMMAND,
                source_label=source_label,
            )
        if "Attachments:\n-" in goal_clean:
            office_task_id = None
        else:
            office_task_id = _queue_office_goal(goal_clean, source_label=source_label)
        if office_task_id is not None:
            return office_task_id
        iagent_bin = _resolve_iagent_executable(result.config)
        if iagent_bin is None:
            tray_icon.notify(
                "iAgent Task Error",
                "iAgent not found. Build backend/iagent or set iagent_path.",
            )
            logger.warning(
                "iAgent executable not found via PATH, IAGENT_BIN, "
                "config.iagent_path, or backend/iagent/target"
            )
            return None
        display = f"iagent run --json <goal: {goal_clean[:80]}>"
        iagent_env = {"OPENAI_API_KEY": result.config.minimax_api_key}
        try:
            task_id = command_runner.enqueue_exec(
                [
                    str(iagent_bin),
                    "run",
                    "--json",
                    "--quiet",
                    goal_clean,
                ],
                cwd=Path.home(),
                display_command=display,
                env=iagent_env,
            )
        except ValueError as exc:
            tray_icon.notify("iAgent Task Error", str(exc))
            return None
        tray_icon.notify("iAgent", f"Queued {source_label} iagent task {task_id}.")
        return task_id

    def _on_run_command_requested() -> None:
        command, ok = QInputDialog.getText(
            None,
            "Run Background Command",
            "Shell command to run in background:",
            QLineEdit.EchoMode.Normal,
            "",
        )
        if not ok or not command.strip():
            return
        _queue_background_command(command, source_label="Manual")

    tray_icon.run_command_requested.connect(_on_run_command_requested)
    tray_icon.ambient_start_requested.connect(_start_ambient_mode)
    tray_icon.ambient_stop_requested.connect(_stop_ambient_mode)

    def _on_command_started(task_id: str, command: str) -> None:
        logger.info("background task started %s cmd=%s", task_id, command)

    def _on_command_running(task_id: str, command: str) -> None:
        logger.info("background task running %s cmd=%s", task_id, command)

    def _on_command_finished(
        task_id: str,
        command: str,
        exit_code: int,
        stdout_text: str,
        stderr_text: str,
        elapsed: float,
    ) -> None:
        logger.info(
            "background task %s exit=%s cmd=%s stdout_len=%d stderr_len=%d",
            task_id,
            exit_code,
            command,
            len(stdout_text),
            len(stderr_text),
        )
        if exit_code == 0:
            created_pattern = re.search(
                r"created_(presentation|document|workbook)=(.+)",
                stdout_text,
            )
            if created_pattern:
                artifact_type = created_pattern.group(1).strip()
                artifact_path = created_pattern.group(2).strip()
                label = {
                    "presentation": "Presentation",
                    "document": "Document",
                    "workbook": "Workbook",
                }.get(artifact_type, "File")
                tray_icon.notify("iAgent", f"{label} ready: {artifact_path}")
        execution_memory.record_command_outcome(
            command=command,
            exit_code=exit_code,
            stdout_text=stdout_text,
            stderr_text=stderr_text,
        )
        task_inbox.on_task_finished(
            task_id,
            command,
            exit_code,
            stdout_text,
            stderr_text,
            elapsed,
        )

    def _on_command_failed(task_id: str, command: str, error_text: str) -> None:
        logger.error("background task %s crashed cmd=%s err=%s", task_id, command, error_text)
        execution_memory.record_command_crash(command=command, error_text=error_text)
        task_inbox.on_task_failed(task_id, command, error_text)

    command_runner.command_started.connect(_on_command_started)
    command_runner.command_started.connect(task_inbox.on_task_started)
    command_runner.command_running.connect(_on_command_running)
    command_runner.command_running.connect(task_inbox.on_task_running)
    command_runner.command_finished.connect(_on_command_finished)
    command_runner.command_failed.connect(_on_command_failed)

    def _on_task_feedback(task_id: str, feedback_text: str) -> None:
        task = task_inbox.get_task_record(task_id)
        if task is None:
            tray_icon.notify("iAgent", f"Task {task_id} not found.")
            return

        goal = (
            f"Improve or update work for task {task_id}.\\n"
            f"Original command: {task.command}\\n"
            f"Original status: {task.status}\\n"
            f"Original exit code: {task.exit_code}\\n"
            f"Original stdout:\\n{task.stdout_text}\\n"
            f"Original stderr:\\n{task.stderr_text}\\n"
            f"User feedback:\\n{feedback_text}\\n"
            "Apply the requested changes and return a concise update summary."
        )
        followup_run_id = _queue_jcode_goal(goal, source_label="Task review")
        if followup_run_id is not None:
            task_inbox.bind_followup_run(followup_run_id, task.task_id, feedback_text)

    task_inbox.task_feedback_requested.connect(_on_task_feedback)

    # ------------------------------------------------------------------
    # Hotkey monitor
    # ------------------------------------------------------------------
    hotkey_binding = result.config.hotkey if result.config is not None else "ctrl+alt"
    hotkey_monitor = HotkeyMonitor(binding=hotkey_binding)

    # ------------------------------------------------------------------
    # CompanionManager (only when config loaded successfully)
    # ------------------------------------------------------------------
    if result.config is not None:
        # MiniMax API key goes directly to LLMClient (no worker proxy needed)
        llm = LLMClient(api_key=result.config.minimax_api_key)
        # AssemblyAI token fetched via Worker endpoint configured in config.toml.
        worker_url = result.config.worker_url or ""
        transcription = TranscriptionClient(
            worker_url=worker_url,
            assemblyai_api_key=result.config.assemblyai_api_key,
        )
        tts = TTSClient(
            worker_url=worker_url,
            tts_provider=result.config.tts_provider,
            eleven_labs_api_key=result.config.eleven_labs_api_key,
            eleven_labs_voice_id=result.config.eleven_labs_voice_id,
        )

        manager = CompanionManager(
            config=result.config,
            mic=mic,
            hotkey=hotkey_monitor,
            transcription=transcription,
            llm=llm,
            tts=tts,
            screen_capture_fn=capture_all,
            panel_visibility_controller=panel_visibility,
            execution_memory=execution_memory,
        )

        # Transcription -> log
        manager.final_transcript.connect(
            lambda text: logger.info("final transcript: %s", text)
        )

        # LLM response -> log
        manager.response_complete.connect(
            lambda text: logger.info("response complete: %s", text[:120])
        )
        manager.response_complete.connect(task_inbox.on_assistant_feedback)

        # Errors -> log + companion
        manager.error.connect(
            lambda msg: logger.error("error: %s", msg)
        )
        manager.error.connect(lambda msg: tray_icon.notify("iAgent Error", msg[:180]))
        manager.error.connect(task_inbox.on_assistant_error)
        manager.state_changed.connect(task_inbox.on_voice_state_changed)
        manager.state_changed.connect(
            lambda s: tray_icon.set_hold_to_talk_active(s == VoiceState.LISTENING)
        )

        # Transcription -> history window
        manager.interim_transcript.connect(history.append_interim)
        manager.final_transcript.connect(history.set_final)

        # LLM response -> history window
        manager.response_delta.connect(history.append_delta)
        manager.response_complete.connect(history.commit_turn)

        # Errors -> history window
        manager.error.connect(history.show_error)

        # Non-disruptive action handling.
        manager.background_command_requested.connect(
            lambda cmd: _queue_background_command(cmd, source_label="AI")
        )
        manager.iagent_goal_requested.connect(
            lambda goal: _queue_jcode_goal(goal, source_label="AI")
        )
        manager.typing_action_blocked.connect(
            lambda text, press_enter: tray_icon.notify(
                "iAgent Draft Ready",
                (
                    "Typing was not executed (safe mode). "
                    f"Draft: {text[:120]}"
                    + (" [ENTER]" if press_enter else "")
                ),
            )
        )
        manager.proposal_requested.connect(proposal_popups.show_proposal)
        manager.proposal_requested.connect(
            lambda proposal: tray_icon.notify("iAgent Proposal", proposal.title)
        )
        proposal_popups.proposal_accepted.connect(manager.accept_proposal)
        proposal_popups.proposal_rejected.connect(manager.reject_proposal)
        manager.proposal_decided.connect(
            lambda proposal, accepted: tray_icon.notify(
                "iAgent Proposal",
                f"{'Validated' if accepted else 'Refused'}: {proposal.title}",
            )
        )
        manager.success_turn_completed.connect(
            lambda: tray_icon.notify("iAgent", "Task complete")
        )
        task_inbox.prompt_submitted.connect(manager.submit_text_prompt)
        task_inbox.voice_start_requested.connect(manager.start_voice_prompt)
        task_inbox.voice_stop_requested.connect(manager.stop_voice_prompt)
        tray_icon.voice_start_requested.connect(manager.start_voice_prompt)
        tray_icon.voice_stop_requested.connect(manager.stop_voice_prompt)

    hotkey_monitor.start()
    tray_icon.show()

    # Ensure the pynput listener thread is stopped before the app exits,
    # otherwise Python may hang on interpreter shutdown waiting for it
    # (pynput's helper thread is not always daemonic on Windows).
    result.app.aboutToQuit.connect(hotkey_monitor.stop)
    result.app.aboutToQuit.connect(task_inbox.shutdown)

    # Use qasync to bridge the Qt event loop with asyncio so that
    # asyncio.create_task / ensure_future work inside Qt signal handlers.
    loop = qasync.QEventLoop(result.app)
    asyncio.set_event_loop(loop)

    # Allow Ctrl+C to quit cleanly. qasync swallows SIGINT by default.
    signal.signal(signal.SIGINT, lambda *_: result.app.quit())

    with loop:
        loop.run_forever()
    return 0
