//! Cross-platform advisory file locking.
//! Windows: LockFileEx / UnlockFileEx
//! Unix:    flock(2)

use std::fs::File;
use std::io;

/// An RAII advisory lock on a file.
/// Dropped when the struct is dropped.
pub struct FileLock {
    file: File,
}

impl FileLock {
    /// Acquire an exclusive non-blocking lock on `file`.
    /// Returns error if already locked.
    pub fn acquire_exclusive(file: File) -> io::Result<Self> {
        lock_exclusive(&file)?;
        Ok(Self { file })
    }

    /// Release the lock and return the underlying file.
    pub fn release(self) -> File {
        let file = self.file;
        drop(self);
        file
    }
}

impl Drop for FileLock {
    fn drop(&mut self) {
        let _ = unlock(&self.file);
    }
}

// ── Windows ──────────────────────────────────────────────────────────────────

#[cfg(target_os = "windows")]
mod platform {
    use std::fs::File;
    use std::io;
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::Foundation::HANDLE;
    use windows_sys::Win32::Storage::FileSystem::{
        LockFileEx, UnlockFileEx,
        LOCKFILE_EXCLUSIVE_LOCK, LOCKFILE_FAIL_IMMEDIATELY,
    };
    use windows_sys::Win32::System::IO::OVERLAPPED;

    pub fn lock_exclusive(file: &File) -> io::Result<()> {
        let handle = file.as_raw_handle() as HANDLE;
        let mut overlapped: OVERLAPPED = unsafe { std::mem::zeroed() };
        let ok = unsafe {
            LockFileEx(
                handle,
                LOCKFILE_EXCLUSIVE_LOCK | LOCKFILE_FAIL_IMMEDIATELY,
                0,
                u32::MAX,
                u32::MAX,
                &mut overlapped,
            )
        };
        if ok == 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(())
        }
    }

    pub fn unlock(file: &File) -> io::Result<()> {
        let handle = file.as_raw_handle() as HANDLE;
        let mut overlapped: OVERLAPPED = unsafe { std::mem::zeroed() };
        let ok = unsafe {
            UnlockFileEx(handle, 0, u32::MAX, u32::MAX, &mut overlapped)
        };
        if ok == 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(())
        }
    }
}

// ── Unix ─────────────────────────────────────────────────────────────────────

#[cfg(unix)]
mod platform {
    use std::fs::File;
    use std::io;
    use std::os::unix::io::AsRawFd;

    pub fn lock_exclusive(file: &File) -> io::Result<()> {
        let ret = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
        if ret != 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(())
        }
    }

    pub fn unlock(file: &File) -> io::Result<()> {
        let ret = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_UN) };
        if ret != 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(())
        }
    }
}

fn lock_exclusive(file: &File) -> io::Result<()> {
    platform::lock_exclusive(file)
}

fn unlock(file: &File) -> io::Result<()> {
    platform::unlock(file)
}
