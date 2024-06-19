use std::io::{self, Error, ErrorKind};
use std::os::windows::io::{AsHandle, AsRawHandle};

use windows_sys::Win32::Foundation::ERROR_LOCK_VIOLATION;
use windows_sys::Win32::Foundation::HANDLE;
use windows_sys::Win32::Storage::FileSystem::{
    LockFileEx, UnlockFile, LOCKFILE_EXCLUSIVE_LOCK, LOCKFILE_FAIL_IMMEDIATELY,
};

use super::utils::{syscall, Overlapped};
use super::{RwLockReadGuard, RwLockWriteGuard};

#[derive(Debug)]
pub struct RwLock<T: AsHandle> {
    pub(crate) inner: T,
}

impl<T: AsHandle> RwLock<T> {
    #[inline]
    pub fn new(inner: T) -> Self {
        RwLock { inner }
    }

    #[inline]
    pub fn read(&self) -> io::Result<RwLockReadGuard<'_, T>> {
        self.acquire_lock::<false, true>()?;
        Ok(RwLockReadGuard { lock: self })
    }

    #[inline]
    pub fn try_read(&self) -> io::Result<RwLockReadGuard<'_, T>> {
        self.acquire_lock::<false, false>().map_err(|error| {
            match error.raw_os_error().map(|error_code| error_code as u32) {
                Some(ERROR_LOCK_VIOLATION) => Error::from(ErrorKind::WouldBlock),
                _ => error,
            }
        })?;
        Ok(RwLockReadGuard { lock: self })
    }

    #[inline]
    pub fn write(&mut self) -> io::Result<RwLockWriteGuard<'_, T>> {
        self.acquire_lock::<true, true>()?;
        Ok(RwLockWriteGuard { lock: self })
    }

    #[inline]
    pub fn try_write(&mut self) -> io::Result<RwLockWriteGuard<'_, T>> {
        self.acquire_lock::<true, false>().map_err(|error| {
            match error.raw_os_error().map(|error_code| error_code as u32) {
                Some(ERROR_LOCK_VIOLATION) => Error::from(ErrorKind::WouldBlock),
                _ => error,
            }
        })?;
        Ok(RwLockWriteGuard { lock: self })
    }

    #[inline]
    pub fn into_inner(self) -> T
    where
        T: Sized,
    {
        self.inner
    }

    pub(crate) fn acquire_lock<const WRITE: bool, const BLOCK: bool>(&self) -> io::Result<()> {
        // See: https://stackoverflow.com/a/9186532, https://docs.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-lockfileex
        let handle = self.inner.as_handle().as_raw_handle() as HANDLE;
        let overlapped = Overlapped::zero();
        let flags = if WRITE { LOCKFILE_EXCLUSIVE_LOCK } else { 0 }
            | if BLOCK { 0 } else { LOCKFILE_FAIL_IMMEDIATELY };
        syscall(unsafe { LockFileEx(handle, flags, 0, 1, 0, overlapped.raw()) })?;
        Ok(())
    }

    pub(crate) fn release_lock(&self) -> io::Result<()> {
        let handle = self.inner.as_handle().as_raw_handle() as HANDLE;
        syscall(unsafe { UnlockFile(handle, 0, 0, 1, 0) })?;
        Ok(())
    }
}
