"""ConPTY bridge for Windows — drop-in replacement for pty_bridge.py.

Uses ``pywinpty`` (Windows Pseudo Console API, available since Windows 10
build 17763) to provide the same streaming byte I/O interface that
``PtyBridge`` exposes on POSIX via ``ptyprocess``/``fcntl``/``termios``.

The public surface is intentionally identical:

    from hermes_cli.conpty_bridge import PtyBridge, PtyUnavailableError

So the web_server import can fall through from pty_bridge → conpty_bridge
with zero call-site changes.
"""

from __future__ import annotations

import os
import sys
import subprocess
import time
import threading
from typing import Optional, Sequence

_IS_WINDOWS = sys.platform == "win32"

try:
    if not _IS_WINDOWS:
        raise ImportError("conpty_bridge is Windows-only")
    from winpty import PtyProcess as WinPtyProcess  # type: ignore
    _CONPTY_AVAILABLE = True
except ImportError:
    WinPtyProcess = None  # type: ignore
    _CONPTY_AVAILABLE = False

__all__ = ["PtyBridge", "PtyUnavailableError"]


class PtyUnavailableError(RuntimeError):
    """Raised when a ConPTY cannot be created on this platform."""


class PtyBridge:
    """Windows ConPTY wrapper matching the POSIX PtyBridge API."""

    def __init__(self, proc: "WinPtyProcess"):  # type: ignore[name-defined]
        self._proc = proc
        self._closed = False
        self._read_lock = threading.Lock()

    @classmethod
    def is_available(cls) -> bool:
        return bool(_CONPTY_AVAILABLE)

    @classmethod
    def spawn(
        cls,
        argv: Sequence[str],
        *,
        cwd: Optional[str] = None,
        env: Optional[dict] = None,
        cols: int = 80,
        rows: int = 24,
    ) -> "PtyBridge":
        if not _CONPTY_AVAILABLE:
            if WinPtyProcess is None:
                raise PtyUnavailableError(
                    "The `pywinpty` package is missing. "
                    "Install with: pip install pywinpty "
                    "(or pip install -e '.[pty]')."
                )
            raise PtyUnavailableError("ConPTY is unavailable on this platform.")

        spawn_env = (os.environ.copy() if env is None else env.copy())
        proc = WinPtyProcess.spawn(
            list(argv),
            cwd=cwd,
            env=spawn_env,
            dimensions=(rows, cols),
        )
        return cls(proc)

    @property
    def pid(self) -> int:
        return int(self._proc.pid)

    def is_alive(self) -> bool:
        if self._closed:
            return False
        try:
            return bool(self._proc.isalive())
        except Exception:
            return False

    def read(self, timeout: float = 0.2) -> Optional[bytes]:
        """Read from ConPTY. Returns bytes, empty bytes (no data), or None (EOF)."""
        if self._closed:
            return None
        deadline = time.monotonic() + timeout
        with self._read_lock:
            while time.monotonic() < deadline:
                if not self._proc.isalive():
                    try:
                        data = self._proc.read(65536)
                        if data:
                            return data.encode("utf-8", errors="replace") if isinstance(data, str) else data
                    except (EOFError, OSError):
                        pass
                    return None
                try:
                    data = self._proc.read(65536)
                    if data:
                        return data.encode("utf-8", errors="replace") if isinstance(data, str) else data
                except EOFError:
                    return None
                except OSError:
                    time.sleep(0.01)
        return b""

    def write(self, data: bytes) -> None:
        if self._closed or not data:
            return
        try:
            text = data.decode("utf-8", errors="replace") if isinstance(data, bytes) else data
            self._proc.write(text)
        except (OSError, EOFError):
            pass

    def resize(self, cols: int, rows: int) -> None:
        if self._closed:
            return
        try:
            self._proc.setwinsize(max(1, rows), max(1, cols))
        except (OSError, AttributeError):
            pass

    def close(self) -> None:
        if self._closed:
            return
        self._closed = True

        if self._proc.isalive():
            try:
                subprocess.run(
                    ["taskkill", "/PID", str(self.pid), "/T", "/F"],
                    capture_output=True,
                    timeout=5,
                )
            except (FileNotFoundError, subprocess.TimeoutExpired, OSError):
                try:
                    os.kill(self.pid, 9)
                except (ProcessLookupError, PermissionError, OSError):
                    pass

        deadline = time.monotonic() + 1.0
        while self._proc.isalive() and time.monotonic() < deadline:
            time.sleep(0.02)

    def __enter__(self) -> "PtyBridge":
        return self

    def __exit__(self, *_exc) -> None:
        self.close()
