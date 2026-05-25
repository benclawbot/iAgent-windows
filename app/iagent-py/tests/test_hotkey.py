from pynput import keyboard

from iagent.hotkey import _normalize_key


def test_normalize_key_accepts_generic_ctrl_alt() -> None:
    assert _normalize_key(keyboard.Key.ctrl) == "ctrl_l"
    assert _normalize_key(keyboard.Key.alt) == "alt"
