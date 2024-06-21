use std::io::{self, Error, ErrorKind};
use std::ops::Deref;
use std::os::windows::io::{AsHandle, AsRawHandle, BorrowedHandle, OwnedHandle};

use windows_sys::Win32::Foundation::ERROR_LOCK_VIOLATION;
use windows_sys::Win32::Foundation::HANDLE;
use windows_sys::Win32::Storage::FileSystem::{
    LockFileEx, UnlockFile, LOCKFILE_EXCLUSIVE_LOCK, LOCKFILE_FAIL_IMMEDIATELY,
};

use crate::sys::{AsOpenFile, RwLockTrait};

use super::utils::{syscall, Overlapped};

#[derive(Debug)]
pub struct RwLock<T: AsHandle> {
    pub inner: T,
}

impl<T: AsHandle> Deref for RwLock<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T: AsHandle> RwLockTrait for RwLock<T> {
    type AsOpenFile = T;
    type BorrowedOpenFile<'a> = BorrowedHandle<'a> where Self: 'a;
    type OwnedOpenFile = OwnedHandle;

    #[inline]
    fn new(inner: T) -> Self {
        RwLock { inner }
    }

    #[inline]
    fn into_inner(self) -> T
    where
        T: Sized,
    {
        self.inner
    }

    fn acquire_lock_from_file<const WRITE: bool, const BLOCK: bool>(
        handle: &impl AsOpenFile,
    ) -> io::Result<()> {
        // See: https://stackoverflow.com/a/9186532, https://docs.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-lockfileex
        let handle = handle.as_handle().as_raw_handle() as HANDLE;
        let overlapped = Overlapped::zero();
        let flags = if WRITE { LOCKFILE_EXCLUSIVE_LOCK } else { 0 }
            | if BLOCK { 0 } else { LOCKFILE_FAIL_IMMEDIATELY };
        let result = syscall(unsafe { LockFileEx(handle, flags, 0, 1, 0, overlapped.raw()) });
        if BLOCK {
            result?;
        } else {
            result.map_err(|error| {
                match error.raw_os_error().map(|error_code| error_code as u32) {
                    Some(ERROR_LOCK_VIOLATION) => Error::from(ErrorKind::WouldBlock),
                    _ => error,
                }
            })?;
        }
        Ok(())
    }

    fn borrow_open_file(&self) -> Self::BorrowedOpenFile<'_> {
        self.inner.as_handle()
    }

    fn release_lock_from_file(handle: &impl AsOpenFile) -> io::Result<()> {
        let handle = handle.as_handle().as_raw_handle() as HANDLE;
        syscall(unsafe { UnlockFile(handle, 0, 0, 1, 0) })?;
        Ok(())
    }
}
