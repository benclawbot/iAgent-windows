# libc Audit — §1.2

**Date**: 2026-05-28
**Status**: Complete — actionable findings ready

## Summary

`libc` is used extensively across the codebase. Many usages are gated behind `#[cfg(target_os = "linux")]` or `#[cfg(target_os = "macos")]` and are safe. The actionable Windows-compatibility issues are in the file locking layer only.

---

## Usages by File

### Non-Windows-Only Uses (Safe / Pre-existing)

| File | Usage | Gate | Status |
|---|---|---|---|
| `crates/desktop-monitor/src/file_ops.rs:528` | `libc::statvfs` for disk usage | `#[cfg(target_os = "linux")]` | OK — Linux-only, correctly gated |
| `crates/jcode-storage/src/lib.rs:39` | `libc::geteuid()` | None (warns on Windows) | Appears in lib.rs — verify if storage path |
| `crates/jcode-core/src/stdin_detect.rs:142+` | Various libc ptr math | None | Signal/tty handling — appears Unix-only |
| `src/platform.rs:69+` | `getrlimit`, `setrlimit`, `kill`, `setsid`, `waitpid` | None | **Unix-only platform layer** — this file is clearly not designed for Windows yet |
| `src/process_memory.rs:3` | `libc::c_char` | None | Appears to be Unix-only FFI |
| `src/process_title.rs:80` | `libc::prctl` | None | **Unix-only** — process title via prctl |
| `src/platform_tests.rs:17` | `libc::getsid` | None | Test-only, Unix-only |

### Locking Uses — Actionable (Windows Broken)

| File | Line | Usage | Impact |
|---|---|---|---|
| `src/cli/dispatch.rs:569` | `libc::flock(fd, LOCK_EX \| LOCK_NB)` | **File lock** | Breaks on Windows — fd is a `mina` socket, not a file path |
| `src/server/socket.rs:119` | `libc::flock(fd, LOCK_EX \| LOCK_NB)` | **File lock** | Breaks on Windows |
| `src/tool/selfdev/mod.rs:556` | `libc::flock(file.as_raw_fd(), LOCK_EX \| LOCK_NB)` | **File lock** | Breaks on Windows |

### TTY/Signal Uses — Actionable if Windows TTY Support Is Needed

| File | Line | Usage | Impact |
|---|---|---|---|
| `src/cli/terminal.rs:119-161` | `libc::SIGHUP`, `SIGTERM`, `SIGINT`, `SIGQUIT` | Signal names for Unix PTY | Only used in Unix signal handler setup |
| `src/background.rs:985+` | `libc::SIGTERM`, `SIGKILL` | Process signals | Signal names only work on Unix |
| `src/tool/bash.rs:838+` | `libc::SIGKILL`, `setpgid` | Bash tool signals | Unix-only |

### Update/Display Uses — Minor

| File | Line | Usage | Impact |
|---|---|---|---|
| `src/cli/hot_exec.rs` | Signal names via `update.rs` | Only used in string messages | Cosmetic strings |
| `src/perf.rs:416+` | `getloadavg`, `sysctlbyname` | Linux/macOS perf | `#[cfg(target_os = "linux")]` or macOS only |

---

## Recommended Actions

### Priority 1 — Fix `flock` usages (breaks Windows)

**Option A: Use `fs4` crate** (cross-platform file locking, already in AWS dependencies transitively)
```toml
# Already available via aws-config deps — verify it's there
fs4 = "0.10"
```

**Option B: Gate behind `#[cfg(not(windows))]` and add Windows impl using `LockFileEx` via `windows-sys`**

All three `flock` call sites are for file-based process coordination. The Windows equivalent is `LockFileEx` / `UnlockFileEx` from `Win32_Storage_FileSystem`.

### Priority 2 — Audit `src/platform.rs` if Windows support is needed

This is the Unix platform abstraction layer. If the Windows shell ever needs process management, this needs a Windows-native implementation or removal.

### What Doesn't Need Fixing

- Signal names in strings/messages — cosmetic only
- `#[cfg(target_os = "linux")]` / `#[cfg(target_os = "macos")]` — already gated
- Test-only code — not compiled in release builds
