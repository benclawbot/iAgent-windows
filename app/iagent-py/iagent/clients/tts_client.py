"""TTS client supporting multiple backends.

Piper (local, free, default) â€” no API key needed.
ElevenLabs (cloud, premium) â€” via Cloudflare Worker proxy.
Fallback: pyttsx3 (Windows SAPI5, always available).
"""

from __future__ import annotations

import asyncio
import logging
import os
import shutil
import tempfile
from pathlib import Path

import httpx
from PySide6.QtCore import QObject, Signal
from PySide6.QtMultimedia import QAudioOutput, QMediaPlayer

logger = logging.getLogger(__name__)


class TTSClient(QObject):
    """Multi-backend TTS with fallback chain: Piper â†’ pyttsx3 â†’ text-only.

    Signals:
        playback_started: Emitted when audio playback begins.
        playback_finished: Emitted when audio playback completes normally.
        error(str): Emitted on HTTP or playback errors.
    """

    playback_started = Signal()
    playback_finished = Signal()
    error = Signal(str)

    def __init__(
        self,
        *,
        tts_provider: str = "piper",
        eleven_labs_api_key: str | None = None,
        eleven_labs_voice_id: str | None = None,
        worker_url: str = "",
        parent: QObject | None = None,
    ) -> None:
        super().__init__(parent)
        self._tts_provider = tts_provider
        self._eleven_labs_api_key = eleven_labs_api_key
        self._eleven_labs_voice_id = eleven_labs_voice_id
        self._worker_url = worker_url.rstrip("/")

        self._player = QMediaPlayer(self)
        self._audio_output = QAudioOutput(self)
        self._player.setAudioOutput(self._audio_output)

        self._player.mediaStatusChanged.connect(self._on_media_status)

        self._playback_future: asyncio.Future[bool] | None = None
        self._piper_executable: Path | None = None
        self._piper_voice: Path | None = None
        self._pyttsx3_lock = asyncio.Lock()

        self._setup_piper()

    def _setup_piper(self) -> None:
        """Locate or warn about piper installation."""
        # Check if piper is in PATH
        piper_path = shutil.which("piper")
        if piper_path:
            self._piper_executable = Path(piper_path)
            # Try to find a downloaded voice
            voice_dir = Path.home() / ".iagent" / "piper"
            if voice_dir.exists():
                voices = list(voice_dir.glob("*.onnx"))
                if voices:
                    self._piper_voice = voices[0]
            logger.info("Piper found at %s", self._piper_executable)
        else:
            logger.warning(
                "piper not found in PATH. Install with: pip install piper-tts"
            )

    async def speak(self, text: str) -> None:
        """Fetch/generate TTS audio and play it, awaiting until playback finishes.

        Tries providers in order: configured provider â†’ pyttsx3 fallback â†’ text-only.
        """
        audio_bytes: bytes | None = None

        # Try configured provider first
        if self._tts_provider == "elevenlabs" and self._eleven_labs_api_key:
            audio_bytes = await self._speak_elevenlabs(text)

        if audio_bytes is None and self._piper_executable and self._piper_voice:
            audio_bytes = await self._speak_piper(text)

        if audio_bytes is None:
            pyttsx3_spoke = await self._speak_pyttsx3(text)
            if pyttsx3_spoke:
                return

        if audio_bytes is None:
            logger.warning("All TTS backends failed â€” skipping audio playback")
            return

        await self._play_audio(audio_bytes)

    async def _speak_elevenlabs(self, text: str) -> bytes | None:
        """Generate audio via ElevenLabs API (requires worker proxy for CORS)."""
        if not self._worker_url or not self._eleven_labs_api_key:
            return None

        try:
            async with httpx.AsyncClient(timeout=60.0) as client:
                response = await client.post(
                    f"{self._worker_url}/tts",
                    json={
                        "text": text,
                        "api_key": self._eleven_labs_api_key,
                        "voice_id": self._eleven_labs_voice_id or "",
                    },
                )
                response.raise_for_status()
                return response.content
        except Exception as exc:
            logger.warning("ElevenLabs TTS failed: %s", exc)
            return None

    async def _speak_piper(self, text: str) -> bytes | None:
        """Generate audio via local Piper TTS engine."""
        if not self._piper_executable or not self._piper_voice:
            return None

        try:
            # Write text to temp file (piper reads from stdin or file)
            with tempfile.NamedTemporaryFile(
                mode="w", suffix=".txt", delete=False, encoding="utf-8"
            ) as f:
                f.write(text)
                text_file = f.name

            # Run piper, capture stdout (wav data)
            process = await asyncio.create_subprocess_exec(
                str(self._piper_executable),
                "-m", str(self._piper_voice),
                "--output-raw",
                stdin=asyncio.subprocess.DEVNULL,
                stdout=asyncio.subprocess.PIPE,
                stderr=asyncio.subprocess.PIPE,
            )
            stdout, stderr = await process.communicate()

            os.unlink(text_file)

            if process.returncode != 0:
                logger.warning("Piper exited with code %d: %s", process.returncode, stderr.decode())
                return None

            return stdout

        except Exception as exc:
            logger.warning("Piper TTS failed: %s", exc)
            return None

    async def _speak_pyttsx3(self, text: str) -> bool:
        """Fallback: use Windows SAPI5 via pyttsx3. Returns True when playback was started."""
        try:
            async with self._pyttsx3_lock:
                import pyttsx3

                engine = pyttsx3.init()
                engine.setProperty("rate", 150)

                # pyttsx3 plays synchronously; run in thread to avoid blocking event loop.
                def run_and_wait() -> None:
                    engine.say(text)
                    engine.runAndWait()

                await asyncio.to_thread(run_and_wait)
            return True

        except Exception as exc:
            logger.warning("pyttsx3 TTS failed: %s", exc)
            return False

    async def _play_audio(self, audio_bytes: bytes) -> None:
        """Play raw audio bytes (WAV or MP3 depending on source) via QMediaPlayer."""
        try:
            from PySide6.QtCore import QBuffer, QByteArray, QIODevice

            byte_array = QByteArray(audio_bytes)
            buffer = QBuffer(byte_array, parent=self)
            buffer.open(QIODevice.OpenModeFlag.ReadOnly)

            self._player.setSourceDevice(buffer)
            self._player.play()
            self.playback_started.emit()

            loop = asyncio.get_running_loop()
            self._playback_future = loop.create_future()
            await self._playback_future
        except Exception as exc:
            self.error.emit(f"TTS playback failed: {exc}")

    def stop(self) -> None:
        """Stop playback and resolve the pending future."""
        self._player.stop()
        if self._playback_future and not self._playback_future.done():
            self._playback_future.set_result(False)

    def _on_media_status(self, status) -> None:
        from PySide6.QtMultimedia import QMediaPlayer

        if status == QMediaPlayer.MediaStatus.EndOfMedia:
            if self._playback_future and not self._playback_future.done():
                self._playback_future.set_result(True)
            self.playback_finished.emit()
        elif status == QMediaPlayer.MediaStatus.InvalidMedia:
            err = self._player.errorString() or "invalid media"
            if self._playback_future and not self._playback_future.done():
                self._playback_future.set_result(False)
            self.error.emit(err)
