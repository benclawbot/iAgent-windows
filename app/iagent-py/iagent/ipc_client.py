"""
IPC client for connecting the Python dock to the Rust iAgent backend.
Uses 'iagent run --json' for synchronous requests.
"""
import asyncio
import json
import os
import subprocess
import threading
from pathlib import Path
from typing import Any, AsyncIterator, Optional

import websockets

# Path to the iagent binary - prefer the local build over system PATH
def find_iagent_binary() -> Optional[Path]:
    local = Path(__file__).parent.parent.parent / "backend" / "iagent" / "target" / "release" / "iagent.exe"
    if local.exists():
        return local
    # Try PATH
    import shutil
    path = shutil.which("iagent")
    if path:
        return Path(path)
    return None

def get_runtime_dir() -> Path:
    if os.name == "nt":
        base = Path(os.environ.get("LOCALAPPDATA", Path.home() / "AppData" / "Local"))
        return base / "iAgent"
    return Path.home() / ".config" / "iAgent"

def get_socket_path() -> Path:
    return get_runtime_dir() / "iagent.sock"

def path_to_pipe_name(path: Path) -> str:
    """Convert Unix socket path to Windows named pipe name."""
    import hashlib
    h = hashlib.sha256(str(path).encode()).hexdigest()[:16]
    return f"\\\\.\\pipe\\iagent-{h}"

class IagentClient:
    """Client for talking to the iAgent backend via IPC."""

    def __init__(self, binary_path: Optional[Path] = None):
        self.binary_path = binary_path or find_iagent_binary()
        self._proc: Optional[subprocess.Popen] = None

    def _read_env_api_key(self) -> Optional[str]:
        """Read API key from the settings file."""
        settings_path = get_runtime_dir() / "settings.toml"
        if settings_path.exists():
            try:
                content = settings_path.read_text()
                for line in content.splitlines():
                    if line.startswith("api_key") and "=" in line:
                        return line.split("=", 1)[1].strip().strip('"')
            except Exception:
                pass
        return None

    def _get_env_for_subprocess(self) -> dict:
        """Build env dict for iagent subprocess."""
        env = os.environ.copy()
        api_key = self._read_env_api_key()
        if api_key:
            env["OPENAI_API_KEY"] = api_key
        return env

    def run_json(self, message: str, timeout: int = 60) -> dict:
        """Run a message synchronously via 'iagent run --json'."""
        if not self.binary_path:
            raise RuntimeError("iagent binary not found")
        result = subprocess.run(
            [str(self.binary_path), "run", "--json", message],
            capture_output=True,
            text=True,
            timeout=timeout,
            env=self._get_env_for_subprocess(),
        )
        if result.returncode != 0:
            stderr = result.stderr.strip()
            if "invalid api key" in stderr.lower() or "401" in stderr:
                return {"error": "invalid_api_key", "details": stderr}
            return {"error": f"exit_{result.returncode}", "details": stderr}
        try:
            return json.loads(result.stdout)
        except json.JSONDecodeError as e:
            return {"error": "json_decode_error", "details": str(e), "raw": result.stdout[:500]}

    async def connect_socket(self) -> AsyncIterator[dict]:
        """Connect to the running iagent server via Unix socket (Linux) or named pipe (Windows).
        
        Yields ServerEvent dicts as they arrive.
        """
        import hashlib
        
        if os.name == "nt":
            pipe_name = path_to_pipe_name(get_socket_path())
            # Windows named pipe - use asyncio
            while True:
                try:
                    reader, writer = await asyncio.open_connection(pipe_name, pipe_name)
                    break
                except ConnectionRefusedError:
                    await asyncio.sleep(0.1)
                    continue
            
            async for line in self._read_events_async(reader):
                yield line
        else:
            socket_path = get_socket_path()
            async for line in self._read_events_async(socket_path):
                yield line

    async def _read_events_async(self, path) -> AsyncIterator[dict]:
        """Read newline-delimited JSON events from a socket/pipe."""
        import asyncio
        buffer = ""
        try:
            async for data in self._socket_reader(path):
                buffer += data
                while "\\n" in buffer:
                    line, buffer = buffer.split("\\n", 1)
                    try:
                        yield json.loads(line)
                    except json.JSONDecodeError:
                        pass
        except Exception as e:
            yield {"error": str(e)}

    async def _socket_reader(self, path):
        """Read from a Unix socket or pipe."""
        if os.name == "nt":
            reader, _ = path  # tuple from open_connection
            while True:
                data = await reader.read(4096)
                if not data:
                    break
                yield data.decode()
        else:
            import asyncio
            reader = asyncio.StreamReader()
            protocol = asyncio.StreamReaderProtocol(reader)
            _, writer = await asyncio.get_event_loop().create_connection(
                lambda: protocol, path=path
            )
            while True:
                line = await reader.readline()
                if not line:
                    break
                yield line.decode()

    def send_message(self, content: str, images: list = None, timeout: int = 60) -> dict:
        """Send a message and return the parsed JSON response."""
        return self.run_json(content, timeout)

    def is_server_running(self) -> bool:
        """Check if iagent server is running."""
        return get_socket_path().exists()
