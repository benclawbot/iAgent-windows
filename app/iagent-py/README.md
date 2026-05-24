# iAgent Runtime (`iagent-py`)

`iagent-py` is the Windows desktop runtime for iAgent: tray icon, dock, voice loop, context capture, and background task UX.

It is designed for ambient help:
- stay in your current app
- get short actionable guidance in a compact dock
- offload heavier tasks to backend `jcode` without blocking your workflow

## Operating modes

1. Voice companion mode
- hold hotkey, speak, release
- transcribe + capture on-screen context
- return concise guidance in the bottom-right dock

2. Background task mode
- queue commands or delegated goals
- keep working while tasks execute
- review output/status in the task inbox

3. Proposal popup mode
- AI-suggested `TYPE`, `CMD`, and `JCODE` actions appear as system-wide floating cards
- choose Validate to execute through the existing safe action path
- choose Refuse to dismiss without running the proposed action

## Relationship with `jcode`

iAgent is the frontend orchestrator. `jcode` is the backend executor.

Delegated goals (`[JCODE:...]`) run as:

```bash
jcode run --json --quiet "<goal>"
```

`jcode` resolution order inside iAgent:
1. `IAGENT_JCODE_BIN` environment variable
2. `jcode_path` in `%APPDATA%\\iAgent\\config.toml`
3. `jcode` on system `PATH`
4. bundled repo build output: `backend/jcode/target/(release|debug)/jcode(.exe)`

When ambient mode is started from the tray, backend overlay IPC can be tuned in
the backend ambient config via `[ambient.desktop_overlay]`.

## Runtime architecture

Primary modules:
- `iagent/app.py`: bootstrap + service wiring + backend delegation
- `iagent/companion_manager.py`: voice/state/action routing
- `iagent/background_command_runner.py`: async command execution
- `iagent/proposals.py`: proposal records for user-approved actions
- `iagent/ui/proposal_popup.py`: topmost Validate/Refuse popup cards
- `iagent/ui/task_inbox.py`: badge + inbox details
- `iagent/response_actions.py`: action parser (`POINT`, `TYPE`, `CMD`, `JCODE`)
- `iagent/config.py`: config schema/validation

Execution pipeline:
1. Input capture (voice/hotkey or text)
2. Context capture (screen + app state)
3. Immediate concise response
4. Proposal popup for mutating actions (`TYPE`, `CMD`, `JCODE`)
5. Optional delegated background execution through `jcode`
6. Inbox/badge update on completion or failure

## Requirements

Required:
- Windows 10/11 (64-bit)
- Python 3.12+
- `uv`
- MiniMax key (`minimax_api_key`)

Recommended:
- Rust toolchain (`rustup`, `cargo`) to build bundled backend
- AssemblyAI key if you are not using `worker_url`

## Installation

### 1) Clone with submodules

```bash
git clone https://github.com/benclawbot/iAgent-windows.git
cd iAgent-windows
```

Optional automated setup from repo root:

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\setup-windows.ps1
```

### 2) Build backend once

```bash
cargo build --release
```

### 3) Install Python deps

```bash
cd app/iagent-py
uv sync
```

### 4) Configure

First run creates `%APPDATA%\\iAgent\\config.toml`.

Minimum:

```toml
minimax_api_key = "your-key"
```

Optional explicit backend path:

```toml
jcode_path = "C:\\Users\\YourName\\iAgent-windows\\target\\release\\iagent.exe"
```

### 5) Run

```bash
uv run iagent
```

## Feature summary

- push-to-talk hotkey (`ctrl+alt` or `right_ctrl`)
- live transcription while recording
- multi-monitor screenshot capture
- concise dock response output
- async task inbox with unread badge and command output
- system-wide proposal popups for AI-suggested mutating actions
- guarded command execution (`rm` requires approval)
- foreground typing disabled by default (`allow_foreground_typing=false`)
- tray controls to start/stop backend ambient mode (`jcode ambient desktop --headless`)

## Action tags

- `[POINT:x,y:label]`: move companion cursor
- `[POINT:none]`: suppress pointer move
- `[TYPE:...]`: proposal to draft (or active typing when enabled)
- `[ENTER]`: submit typed content
- `[CMD:...]`: propose a background shell command
- `[JCODE:...]`: propose a delegated backend goal

## Config reference

Edit `%APPDATA%\\iAgent\\config.toml`.

| Field | Required | Default | Description |
|---|---|---|---|
| `minimax_api_key` | yes | none | Primary LLM key |
| `worker_url` | no | empty | Worker proxy URL (`http(s)://...`) |
| `assemblyai_api_key` | no | empty | Direct transcription key |
| `hotkey` | no | `ctrl+alt` | `ctrl+alt` or `right_ctrl` |
| `tts_provider` | no | `piper` | `piper` or `elevenlabs` |
| `eleven_labs_api_key` | no | empty | ElevenLabs key |
| `eleven_labs_voice_id` | no | empty | ElevenLabs voice id |
| `log_level` | no | `INFO` | `DEBUG`, `INFO`, `WARNING`, `ERROR` |
| `knowledge_dir` | no | `%APPDATA%\\iAgent\\knowledge\\` if present | Knowledge root |
| `allow_foreground_typing` | no | `false` | Enables real foreground typing |
| `jcode_path` | no | empty | Explicit backend executable path |

## Build and test

```bash
uv run pyinstaller iagent.spec
uv run pytest
uv run ruff check .
```

## License

MIT (see `LICENSE`).
