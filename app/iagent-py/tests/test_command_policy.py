from iagent.command_policy import (
    GMAIL_COMPOSE_COMMAND,
    normalize_ai_command,
    requires_manual_approval,
)


def test_requires_manual_approval_flags_rm() -> None:
    assert requires_manual_approval("rm -rf /tmp/cache")
    assert requires_manual_approval("echo hi && rm notes.txt")
    assert requires_manual_approval('powershell -Command "rm file.txt"')


def test_requires_manual_approval_ignores_non_rm_commands() -> None:
    assert not requires_manual_approval("git status")
    assert not requires_manual_approval("rmdir temp")
    assert not requires_manual_approval("python -m pytest -q")


def test_normalize_ai_command_rewrites_xpi_gmail_command() -> None:
    normalized, note = normalize_ai_command("start /b C:\\tmp\\gmail-helper.xpi")

    assert normalized == GMAIL_COMPOSE_COMMAND
    assert note is not None and "rewrote" in note


def test_normalize_ai_command_blocks_non_gmail_xpi() -> None:
    normalized, note = normalize_ai_command("start C:\\tmp\\unknown-addon.xpi")

    assert normalized is None
    assert note is not None and "blocked" in note
