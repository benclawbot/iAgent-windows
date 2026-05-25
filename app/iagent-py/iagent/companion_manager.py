"""CompanionManager — orchestration state machine for iAgent.

Owns the hotkey → mic → transcription → screen-capture → LLM pipeline and
emits high-level Qt signals that the UI layer (panel, tray) can bind to
without knowing the plumbing details.

Replaces the ad-hoc closure wiring in ``app.py``.  In Task 4.9 the
application entry point will be refactored to instantiate a
``CompanionManager`` and delegate to it.
"""

from __future__ import annotations

import asyncio
import base64
import logging
from collections import deque
from collections.abc import AsyncGenerator, Callable
from typing import Any, Protocol

from PySide6.QtCore import QByteArray, QObject, Signal

from iagent.active_window import get_foreground_window_title
from iagent.clients.llm_client import LLMClient
from iagent.clients.transcription_client import TranscriptionClient
from iagent.clients.tts_client import TTSClient
from iagent.config import Config
from iagent.conversation_history import ConversationHistory
from iagent.execution_memory import ExecutionMemory
from iagent.hotkey import HotkeyMonitor
from iagent.knowledge_base import load_kb_from_disk, match_app, select_content
from iagent.mic_capture import MicCapture
from iagent.point_mapper import map_point_to_screen
from iagent.prompts import build_system_prompt
from iagent.proposals import ActionProposal, proposals_from_actions
from iagent.response_actions import parse_response_actions
from iagent.screen_capture import ScreenshotImage
from iagent.state import VoiceState

logger = logging.getLogger(__name__)


class CaptureVisibilityController(Protocol):
    """Protocol for hiding/restoring UI during screen capture."""

    def hide_for_capture(self) -> None:
        """Temporarily hide during screen capture."""
        ...

    def restore_after_capture(self) -> None:
        """Restore after screen capture."""
        ...

    def fly_to(self, x: int, y: int) -> None:
        """Animate companion to target screen position."""
        ...


class CompanionManager(QObject):
    """Orchestration state machine for the voice companion pipeline.

    Coordinates hotkey detection, microphone capture, transcription,
    screen capture, and LLM streaming into a single coherent lifecycle
    with cancellation support.
    """

    # ---- Qt signals ----
    state_changed = Signal(VoiceState)
    audio_level = Signal(float)
    interim_transcript = Signal(str)
    final_transcript = Signal(str)
    response_delta = Signal(str)
    response_complete = Signal(str)
    success_turn_completed = Signal()
    proposal_requested = Signal(object)
    proposal_decided = Signal(object, bool)
    background_command_requested = Signal(str)
    jcode_goal_requested = Signal(str)
    typing_action_blocked = Signal(str, bool)
    error = Signal(str)

    def __init__(
        self,
        config: Config,
        mic: MicCapture,
        hotkey: HotkeyMonitor,
        transcription: TranscriptionClient,
        llm: LLMClient,
        tts: TTSClient,
        screen_capture_fn: Callable[[], list[ScreenshotImage]],
        panel_visibility_controller: CaptureVisibilityController,
        execution_memory: ExecutionMemory | None = None,
        parent: QObject | None = None,
    ) -> None:
        super().__init__(parent)

        self._config = config
        self._mic = mic
        self._hotkey = hotkey
        self._transcription = transcription
        self._llm = llm
        self._tts = tts
        self._screen_capture_fn = screen_capture_fn
        self._panel_visibility_controller = panel_visibility_controller
        self._execution_memory = execution_memory

        # Internal state
        self._state: VoiceState = VoiceState.IDLE
        self._current_task: asyncio.Task[None] | None = None
        self._history = ConversationHistory()
        self._knowledge_dir = config.knowledge_dir  # Path | None
        # MiniMax M2.7 is hardcoded — no model picker in this version
        self._current_model: str = "MiniMax-M2.7"
        self._cancel_flag: bool = False
        self._current_screenshots: list[ScreenshotImage] = []
        self._speak_task: asyncio.Task[None] | None = None
        self._allow_foreground_typing = config.allow_foreground_typing
        self._llm_delta_in_think_block = False

        # PCM deque bridge — same pattern as app.py.  Replaced on every
        # hotkey-press cycle so a stale generator cannot leak chunks.
        self._pcm: dict[str, Any] = {
            "deque": deque(),
            "event": asyncio.Event(),
            "done": False,
        }

        # ---- Signal wiring ----
        hotkey.pressed.connect(self._on_hotkey_pressed)
        hotkey.released.connect(self._on_hotkey_released)
        hotkey.cancelled.connect(self._on_hotkey_cancelled)

        mic.audio_level.connect(self.audio_level)
        mic.pcm_chunk.connect(self._on_pcm_chunk)

        transcription.interim_transcript.connect(self.interim_transcript)
        transcription.final_transcript.connect(self._on_final_transcript)
        transcription.error.connect(self._on_error)

        llm.delta.connect(self._on_llm_delta)
        llm.error.connect(self._on_error)

        tts.error.connect(self._on_error)

    # ------------------------------------------------------------------
    # Public API
    # ------------------------------------------------------------------

    def set_model(self, model_id: str) -> None:
        """Update the model used for LLM requests."""
        self._current_model = model_id

    def accept_proposal(self, proposal: ActionProposal) -> None:
        """Execute a user-validated proposal through existing action paths."""
        if proposal.kind == "command":
            self.background_command_requested.emit(proposal.payload)
        elif proposal.kind == "jcode":
            self.jcode_goal_requested.emit(proposal.payload)
        elif proposal.kind == "type":
            if self._allow_foreground_typing:
                asyncio.ensure_future(
                    self._type_into_active_app(proposal.payload, proposal.press_enter)
                )
            else:
                self.typing_action_blocked.emit(proposal.payload, proposal.press_enter)
        self.proposal_decided.emit(proposal, True)

    def reject_proposal(self, proposal: ActionProposal) -> None:
        """Record a user refusal without executing the proposal."""
        logger.info("proposal refused: %s %s", proposal.kind, proposal.proposal_id)
        self.proposal_decided.emit(proposal, False)

    def submit_text_prompt(self, text: str) -> None:
        """Submit a typed prompt without using microphone capture."""
        cleaned = text.strip()
        if not cleaned:
            return

        if self._current_task is not None and not self._current_task.done():
            self._cancel_flag = True
            self._current_task.cancel()

        self._cancel_flag = False
        self._set_state(VoiceState.PROCESSING)
        self._current_task = asyncio.ensure_future(self._run_turn(cleaned))

    def start_voice_prompt(self) -> None:
        """Start voice capture programmatically (same flow as hotkey press)."""
        self._on_hotkey_pressed()

    def stop_voice_prompt(self) -> None:
        """Stop voice capture programmatically (same flow as hotkey release)."""
        if self._state == VoiceState.LISTENING:
            self._on_hotkey_released()

    # ------------------------------------------------------------------
    # PCM deque bridge
    # ------------------------------------------------------------------

    def _on_pcm_chunk(self, chunk: QByteArray) -> None:
        self._pcm["deque"].append(chunk)
        self._pcm["event"].set()

    async def _pcm_async_generator(self) -> AsyncGenerator[QByteArray, None]:
        """Async generator that yields QByteArray chunks from the deque.

        Snapshots the deque and event refs at first iteration.  The ``done``
        flag is read from the live dict — stale generators from a previous
        session are terminated via ``task.cancel()`` in ``_on_hotkey_pressed``,
        not via the done flag.
        """
        dq: deque[QByteArray] = self._pcm["deque"]
        ev: asyncio.Event = self._pcm["event"]
        while True:
            await ev.wait()
            ev.clear()
            while dq:
                yield dq.popleft()
            if self._pcm["done"]:
                return

    def _reset_pcm_bridge(self) -> None:
        """Replace the PCM bridge state for a fresh session."""
        self._pcm["deque"] = deque()
        self._pcm["event"] = asyncio.Event()
        self._pcm["done"] = False

    def _stop_pcm_bridge(self) -> None:
        """Signal the PCM generator to terminate."""
        self._pcm["done"] = True
        self._pcm["event"].set()

    # ------------------------------------------------------------------
    # State transitions
    # ------------------------------------------------------------------

    def _set_state(self, new_state: VoiceState) -> None:
        self._state = new_state
        self.state_changed.emit(new_state)

    # ------------------------------------------------------------------
    # Hotkey handlers
    # ------------------------------------------------------------------

    def _on_hotkey_pressed(self) -> None:
        logger.debug("hotkey pressed (state=%s)", self._state)

        # Stop TTS playback immediately.
        self._tts.stop()
        if self._speak_task is not None and not self._speak_task.done():
            self._speak_task.cancel()

        # Interrupt any in-flight turn OR lingering stream task.
        if self._state != VoiceState.IDLE:
            self._cancel_flag = True
        if self._current_task is not None and not self._current_task.done():
            self._current_task.cancel()

        # Reset PCM bridge for a fresh session.
        self._reset_pcm_bridge()

        # Transition to LISTENING, start mic, start transcription.
        self._set_state(VoiceState.LISTENING)
        self._mic.start()

        self._current_task = asyncio.ensure_future(
            self._transcription.start_stream(self._pcm_async_generator())
        )

    def _on_hotkey_released(self) -> None:
        logger.debug("hotkey released")
        self._set_state(VoiceState.PROCESSING)
        self._mic.stop()
        self._stop_pcm_bridge()
        asyncio.ensure_future(self._transcription.stop_stream())

    def _on_hotkey_cancelled(self) -> None:
        logger.debug("hotkey cancelled")
        self._mic.stop()
        self._stop_pcm_bridge()
        asyncio.ensure_future(self._transcription.stop_stream())
        # Transition to IDLE directly — the transcription client may not
        # emit a final_transcript if the session never fully started.
        self._set_state(VoiceState.IDLE)

    # ------------------------------------------------------------------
    # Transcription handler
    # ------------------------------------------------------------------

    def _on_final_transcript(self, text: str) -> None:
        if not text:
            # User pressed and released without speaking enough.
            self._set_state(VoiceState.IDLE)
            return

        # Reset cancel flag here (not in _run_turn) so a rapid
        # press between task assignment and coroutine start cannot
        # have its cancellation silently undone.
        self._cancel_flag = False
        self._current_task = asyncio.ensure_future(self._run_turn(text))

    # ------------------------------------------------------------------
    # LLM delta relay (only when not cancelled)
    # ------------------------------------------------------------------

    def _on_llm_delta(self, text: str) -> None:
        if not self._cancel_flag:
            visible = self._strip_reasoning_from_delta(text)
            if visible:
                self.response_delta.emit(visible)

    def _strip_reasoning_from_delta(self, delta: str) -> str:
        """Filter out streamed <think> blocks from user-visible text."""
        out: list[str] = []
        i = 0
        while i < len(delta):
            if self._llm_delta_in_think_block:
                end = delta.find("</think>", i)
                if end == -1:
                    return "".join(out)
                self._llm_delta_in_think_block = False
                i = end + len("</think>")
                continue

            start = delta.find("<think>", i)
            if start == -1:
                out.append(delta[i:])
                break

            if start > i:
                out.append(delta[i:start])
            i = start + len("<think>")
            self._llm_delta_in_think_block = True

        return "".join(out)

    # ------------------------------------------------------------------
    # Error handler
    # ------------------------------------------------------------------

    def _on_error(self, msg: str) -> None:
        logger.error("companion error: %s", msg)
        self.error.emit(msg)
        self._set_state(VoiceState.IDLE)

    async def _type_into_active_app(self, text: str, press_enter: bool) -> None:
        """Type text into the currently focused app, optionally pressing Enter."""
        # Imported lazily to avoid global keyboard hook setup unless this
        # explicitly enabled behavior is used.
        from pynput.keyboard import Controller, Key

        def _do_type() -> None:
            keyboard = Controller()
            keyboard.type(text)
            if press_enter:
                keyboard.press(Key.enter)
                keyboard.release(Key.enter)

        await asyncio.to_thread(_do_type)

    # ------------------------------------------------------------------
    # Turn pipeline (async)
    # ------------------------------------------------------------------

    async def _run_turn(self, text: str) -> None:
        """Execute the full turn: screen capture → LLM request → history.

        This coroutine becomes ``_current_task`` and supports cancellation
        via ``_cancel_flag`` (cooperative) and ``task.cancel()`` (hard).
        """
        try:
            # Yield control so stop_stream (which is still draining after
            # the recv loop emitted final_transcript synchronously) can
            # finish before we do any work that pumps the Qt event loop
            # (hide_for_capture calls processEvents, which would re-enter
            # the stop_stream task and trigger a RuntimeError).
            await asyncio.sleep(0)

            # Emit the final transcript so the UI can display it.
            self.final_transcript.emit(text)
            if self._execution_memory is not None:
                self._execution_memory.record_user_feedback(text)

            # Hide the panel so it doesn't appear in the screenshot.
            # The async sleep lets qasync process the Qt opacity change
            # AND lets pending asyncio tasks (stop_stream cleanup) settle
            # — avoids re-entrancy that processEvents() would cause.
            self._panel_visibility_controller.hide_for_capture()
            await asyncio.sleep(0.05)
            try:
                screenshots = await asyncio.to_thread(self._screen_capture_fn)
                self._current_screenshots = screenshots
            finally:
                self._panel_visibility_controller.restore_after_capture()

            # Build image content blocks.
            image_blocks: list[dict[str, Any]] = []
            for screenshot in screenshots:
                b64 = base64.b64encode(screenshot.jpeg_bytes).decode("ascii")
                image_blocks.append({"type": "text", "text": screenshot.label})
                image_blocks.append(
                    {
                        "type": "image",
                        "source": {
                            "type": "base64",
                            "media_type": "image/jpeg",
                            "data": b64,
                        },
                    }
                )

            # Build messages from conversation history.
            messages = self._history.messages_for_request(
                current_user_text=text,
                current_images=image_blocks,
            )

            # Detect active app and load KB
            window_title = get_foreground_window_title()
            kb_content = None
            app_name = None
            if self._knowledge_dir is not None:
                apps = load_kb_from_disk(self._knowledge_dir)
                matched = match_app(window_title, apps)
                if matched is not None:
                    app_name = matched.name
                    kb_content = select_content(matched, text)
                    logger.info("KB loaded: %s (%d chars)", app_name, len(kb_content))
                else:
                    logger.debug("no KB match for window: %s", window_title)

            execution_lessons = None
            if self._execution_memory is not None:
                execution_lessons = self._execution_memory.lessons_for_prompt()
            system_prompt = build_system_prompt(
                kb_content,
                app_name,
                execution_lessons=execution_lessons,
            )

            # Transition to RESPONDING.
            self._set_state(VoiceState.RESPONDING)

            # Send to LLM.
            self._llm_delta_in_think_block = False
            full_text = await self._llm.send(
                messages,
                system=system_prompt,
                model=self._current_model,
            )

            # Only commit to history and emit completion if not cancelled.
            if not self._cancel_flag:
                actions = parse_response_actions(full_text)
                visible_text = actions.spoken_text.strip()

                if not visible_text:
                    action_notes: list[str] = []
                    if actions.cli_command:
                        action_notes.append("queued a background command")
                    if actions.jcode_goal:
                        action_notes.append("queued a jcode workflow")
                    if actions.type_text:
                        if self._allow_foreground_typing:
                            action_notes.append("typed into the active app")
                        else:
                            action_notes.append("prepared a typing draft for manual review")
                    if actions.point_tag is not None:
                        action_notes.append("prepared an on-screen pointer hint")
                    if actions.press_enter and actions.type_text is None:
                        action_notes.append("scheduled an Enter key press")

                    if action_notes:
                        visible_text = "I handled your request and " + ", ".join(action_notes) + "."
                    else:
                        visible_text = "I could not produce a visible reply. Please try again."

                self._history.append(text, visible_text)
                self.response_complete.emit(visible_text)
                self.success_turn_completed.emit()

                if actions.point_tag is not None:
                    coords = map_point_to_screen(actions.point_tag, self._current_screenshots)
                    if coords is not None:
                        self._panel_visibility_controller.fly_to(coords[0], coords[1])
                        logger.info(
                            "POINT: (%d, %d) label=%s",
                            coords[0], coords[1], actions.point_tag.label,
                        )

                for proposal in proposals_from_actions(actions):
                    self.proposal_requested.emit(proposal)
                    logger.info(
                        "proposal requested: %s %s",
                        proposal.kind,
                        proposal.proposal_id,
                    )

                if self._state == VoiceState.RESPONDING:
                    self._set_state(VoiceState.IDLE)

        except asyncio.CancelledError:
            logger.debug("turn cancelled")
            self._set_state(VoiceState.IDLE)

        except Exception as exc:  # noqa: BLE001
            logger.error("turn pipeline error: %s", exc)
            self.error.emit(str(exc))
            self._set_state(VoiceState.IDLE)

        # On success, stay in RESPONDING so the response text remains
        # visible until the next hotkey press resets to LISTENING.
