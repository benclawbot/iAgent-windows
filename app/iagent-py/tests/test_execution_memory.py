from pathlib import Path

from iagent.execution_memory import ExecutionMemory
from iagent.prompts import build_system_prompt


def test_execution_memory_learns_from_failed_open_a(tmp_path: Path) -> None:
    memory = ExecutionMemory(tmp_path / "execution_memory.json")
    memory.record_command_outcome(
        command="open -a Edge",
        exit_code=1,
        stdout_text="",
        stderr_text="open: command not found",
    )

    lessons = memory.lessons_for_prompt()
    assert any("do not use macos open -a" in lesson for lesson in lessons)


def test_execution_memory_learns_from_user_feedback(tmp_path: Path) -> None:
    memory = ExecutionMemory(tmp_path / "execution_memory.json")
    memory.record_user_feedback("you didn't open edge")

    lessons = memory.lessons_for_prompt()
    assert any("edge did not open" in lesson for lesson in lessons)


def test_build_system_prompt_includes_execution_lessons() -> None:
    prompt = build_system_prompt(
        kb_content=None,
        app_name=None,
        execution_lessons=["use start microsoft-edge: for edge launch"],
    )

    assert "execution memory" in prompt
    assert "use start microsoft-edge: for edge launch" in prompt


def test_execution_memory_learns_from_powerpoint_creation_feedback(tmp_path: Path) -> None:
    memory = ExecutionMemory(tmp_path / "execution_memory.json")
    memory.record_user_feedback("you did not create the powerpoint slideshow")

    lessons = memory.lessons_for_prompt()
    assert any(
        "build and save the presentation in background first" in lesson
        for lesson in lessons
    )


def test_execution_memory_learns_from_powerpoint_stop_feedback(tmp_path: Path) -> None:
    memory = ExecutionMemory(tmp_path / "execution_memory.json")
    memory.record_user_feedback("it opens the app but stops there")

    lessons = memory.lessons_for_prompt()
    assert any(
        "do the real build in background first" in lesson
        for lesson in lessons
    )


def test_execution_memory_learns_from_word_creation_feedback(tmp_path: Path) -> None:
    memory = ExecutionMemory(tmp_path / "execution_memory.json")
    memory.record_user_feedback("word opens the app but stops and did not create document")

    lessons = memory.lessons_for_prompt()
    assert any(
        "word opened without finishing document creation" in lesson
        for lesson in lessons
    )


def test_execution_memory_learns_from_excel_creation_feedback(tmp_path: Path) -> None:
    memory = ExecutionMemory(tmp_path / "execution_memory.json")
    memory.record_user_feedback("excel opens app but stops there and didn't create the workbook")

    lessons = memory.lessons_for_prompt()
    assert any(
        "excel opened without finishing workbook creation" in lesson
        for lesson in lessons
    )


def test_execution_memory_learns_from_xpi_gmail_feedback(tmp_path: Path) -> None:
    memory = ExecutionMemory(tmp_path / "execution_memory.json")
    memory.record_user_feedback("it tried to open a .xpi when i asked for a gmail draft")

    lessons = memory.lessons_for_prompt()
    assert any(".xpi extension package" in lesson for lesson in lessons)
