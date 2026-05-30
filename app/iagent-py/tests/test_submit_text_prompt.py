import asyncio

from PySide6.QtCore import QByteArray, QCoreApplication, QObject, Signal

from iagent.companion_manager import CompanionManager
from iagent.config import Config


class _FakeMic(QObject):
    pcm_chunk = Signal(QByteArray)
    audio_level = Signal(float)
    error = Signal(str)


class _FakeHotkey(QObject):
    pressed = Signal()
    released = Signal()
    cancelled = Signal()


class _FakeTranscription(QObject):
    interim_transcript = Signal(str)
    final_transcript = Signal(str)
    error = Signal(str)

    def __init__(self) -> None:
        super().__init__()
        self.start_calls = 0
        self.stop_calls = 0

    async def start_stream(self, _pcm_iter) -> None:
        self.start_calls += 1

    async def stop_stream(self) -> None:
        self.stop_calls += 1


class _FakeLLM(QObject):
    delta = Signal(str)
    error = Signal(str)

    def __init__(self, response: str = "here you go [POINT:none]") -> None:
        super().__init__()
        self.calls = 0
        self.response = response

    async def send(self, messages, system: str, model: str):  # noqa: ANN001
        self.calls += 1
        assert isinstance(messages, list)
        assert isinstance(system, str)
        assert isinstance(model, str)
        return self.response


class _FakeTTS(QObject):
    error = Signal(str)

    def stop(self) -> None:
        return


class _FakePanel:
    def hide_for_capture(self) -> None:
        return

    def restore_after_capture(self) -> None:
        return

    def fly_to(self, _x: int, _y: int) -> None:
        return


def _fake_capture():
    return []


def _make_config() -> Config:
    return Config(
        minimax_api_key="test-key",
        worker_url=None,
        assemblyai_api_key=None,
        hotkey="ctrl+alt",
        tts_provider="piper",
        eleven_labs_api_key=None,
        eleven_labs_voice_id=None,
        log_level="INFO",
        knowledge_dir=None,
        allow_foreground_typing=False,
        iagent_path=None,
    )


def test_submit_text_prompt_bypasses_transcription() -> None:
    _app = QCoreApplication.instance() or QCoreApplication([])

    mic = _FakeMic()
    hotkey = _FakeHotkey()
    transcription = _FakeTranscription()
    llm = _FakeLLM()
    tts = _FakeTTS()

    manager = CompanionManager(
        config=_make_config(),
        mic=mic,
        hotkey=hotkey,
        transcription=transcription,
        llm=llm,
        tts=tts,
        screen_capture_fn=_fake_capture,
        panel_visibility_controller=_FakePanel(),
    )

    emitted: list[str] = []
    manager.response_complete.connect(emitted.append)

    loop = asyncio.new_event_loop()
    try:
        asyncio.set_event_loop(loop)
        manager.submit_text_prompt("go to youtube and search for most recent ai related videos")
        assert manager._current_task is not None
        loop.run_until_complete(manager._current_task)
    finally:
        loop.close()
        asyncio.set_event_loop(None)

    # Typed prompts should run directly through _run_turn/LLM without starting ASR.
    assert transcription.start_calls == 0
    assert transcription.stop_calls == 0
    assert llm.calls == 1
    assert emitted and emitted[0]


def test_submit_text_prompt_turns_command_action_into_proposal() -> None:
    _app = QCoreApplication.instance() or QCoreApplication([])

    manager = CompanionManager(
        config=_make_config(),
        mic=_FakeMic(),
        hotkey=_FakeHotkey(),
        transcription=_FakeTranscription(),
        llm=_FakeLLM("I can run that. [CMD:python -m pytest -q]"),
        tts=_FakeTTS(),
        screen_capture_fn=_fake_capture,
        panel_visibility_controller=_FakePanel(),
    )

    proposals: list[object] = []
    queued_commands: list[str] = []
    manager.proposal_requested.connect(proposals.append)
    manager.background_command_requested.connect(queued_commands.append)

    loop = asyncio.new_event_loop()
    try:
        asyncio.set_event_loop(loop)
        manager.submit_text_prompt("run tests")
        assert manager._current_task is not None
        loop.run_until_complete(manager._current_task)
    finally:
        loop.close()
        asyncio.set_event_loop(None)

    assert queued_commands == []
    assert len(proposals) == 1
    assert proposals[0].kind == "command"
    assert proposals[0].payload == "python -m pytest -q"


def test_accepting_command_proposal_routes_to_existing_command_signal() -> None:
    _app = QCoreApplication.instance() or QCoreApplication([])

    manager = CompanionManager(
        config=_make_config(),
        mic=_FakeMic(),
        hotkey=_FakeHotkey(),
        transcription=_FakeTranscription(),
        llm=_FakeLLM(),
        tts=_FakeTTS(),
        screen_capture_fn=_fake_capture,
        panel_visibility_controller=_FakePanel(),
    )

    from iagent.proposals import ActionProposal

    queued_commands: list[str] = []
    decisions: list[tuple[str, bool]] = []
    manager.background_command_requested.connect(queued_commands.append)
    manager.proposal_decided.connect(
        lambda proposal, accepted: decisions.append((proposal.kind, accepted))
    )

    proposal = ActionProposal(
        proposal_id="command:test",
        kind="command",
        title="Run Command",
        body="git status",
        payload="git status",
    )
    manager.accept_proposal(proposal)

    assert queued_commands == ["git status"]
    assert decisions == [("command", True)]
