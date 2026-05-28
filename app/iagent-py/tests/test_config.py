from pathlib import Path

import pytest

from iagent.config import Config, ConfigError


def _write_config(path: Path, body: str) -> None:
    path.write_text(body, encoding="utf-8")


def test_config_defaults_to_safe_foreground_typing(tmp_path: Path) -> None:
    cfg_path = tmp_path / "config.toml"
    _write_config(
        cfg_path,
        'minimax_api_key = "x"\n'
        'hotkey = "ctrl+alt"\n'
        'tts_provider = "piper"\n',
    )

    cfg = Config.from_path(cfg_path)
    assert cfg.allow_foreground_typing is False


def test_config_accepts_allow_foreground_typing_boolean(tmp_path: Path) -> None:
    cfg_path = tmp_path / "config.toml"
    _write_config(
        cfg_path,
        'minimax_api_key = "x"\n'
        'hotkey = "ctrl+alt"\n'
        'tts_provider = "piper"\n'
        "allow_foreground_typing = true\n",
    )

    cfg = Config.from_path(cfg_path)
    assert cfg.allow_foreground_typing is True


def test_config_rejects_non_boolean_allow_foreground_typing(tmp_path: Path) -> None:
    cfg_path = tmp_path / "config.toml"
    _write_config(
        cfg_path,
        'minimax_api_key = "x"\n'
        'hotkey = "ctrl+alt"\n'
        'tts_provider = "piper"\n'
        'allow_foreground_typing = "yes"\n',
    )

    with pytest.raises(ConfigError, match="allow_foreground_typing must be true or false"):
        Config.from_path(cfg_path)


def test_config_rejects_invalid_toml(tmp_path: Path) -> None:
    cfg_path = tmp_path / "config.toml"
    cfg_path.write_text("this is not valid toml {{{", encoding="utf-8")
    with pytest.raises(ConfigError, match="parse TOML"):
        Config.from_path(cfg_path)


def test_config_rejects_missing_minimax_api_key(tmp_path: Path) -> None:
    cfg_path = tmp_path / "config.toml"
    _write_config(
        cfg_path,
        'hotkey = "ctrl+alt"\n'
        'tts_provider = "piper"\n',
    )
    with pytest.raises(ConfigError, match="minimax_api_key is required"):
        Config.from_path(cfg_path)


def test_config_rejects_invalid_hotkey(tmp_path: Path) -> None:
    cfg_path = tmp_path / "config.toml"
    _write_config(
        cfg_path,
        'minimax_api_key = "x"\n'
        'hotkey = "banana"\n'
        'tts_provider = "piper"\n',
    )
    with pytest.raises(ConfigError, match="hotkey must be one of"):
        Config.from_path(cfg_path)


def test_config_accepts_iagent_path_file(tmp_path: Path) -> None:
    cfg_path = tmp_path / "config.toml"
    iagent_bin = tmp_path / "iagent.exe"
    iagent_bin.write_text("", encoding="utf-8")
    _write_config(
        cfg_path,
        'minimax_api_key = "x"\n'
        f'iagent_path = "{iagent_bin.as_posix()}"\n',
    )

    cfg = Config.from_path(cfg_path)
    assert cfg.iagent_path == iagent_bin.resolve()


def test_config_rejects_invalid_iagent_path(tmp_path: Path) -> None:
    cfg_path = tmp_path / "config.toml"
    missing = (tmp_path / "missing-iagent.exe").as_posix()
    _write_config(
        cfg_path,
        'minimax_api_key = "x"\n'
        f'iagent_path = "{missing}"\n',
    )

    with pytest.raises(ConfigError, match="iagent_path must point to an iagent executable file"):
        Config.from_path(cfg_path)


def test_ensure_exists_creates_from_example(tmp_path: Path) -> None:
    example_path = tmp_path / "config.example.toml"
    example_path.write_text('minimax_api_key = "example"\n', encoding="utf-8")
    target_path = tmp_path / "nested" / "config.toml"

    created = Config.ensure_exists(target_path, example_path)

    assert created is True
    assert target_path.exists()
    assert target_path.read_text(encoding="utf-8") == example_path.read_text(encoding="utf-8")


def test_ensure_exists_noop_when_already_present(tmp_path: Path) -> None:
    example_path = tmp_path / "config.example.toml"
    example_path.write_text('minimax_api_key = "example"\n', encoding="utf-8")
    target_path = tmp_path / "config.toml"
    target_path.write_text('minimax_api_key = "real"\n', encoding="utf-8")

    created = Config.ensure_exists(target_path, example_path)

    assert created is False
    assert target_path.read_text(encoding="utf-8") == 'minimax_api_key = "real"\n'
