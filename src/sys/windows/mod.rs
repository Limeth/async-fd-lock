mod utils;

use std::io::{self, Error, ErrorKind};
use std::os::windows::io::{AsRawHandle, BorrowedHandle, OwnedHandle};
use utils::{syscall, Overlapped};
use windows_sys::Win32::Foundation::ERROR_LOCK_VIOLATION;
use windows_sys::Win32::Foundation::HANDLE;
use windows_sys::Win32::Storage::FileSystem::{
    LockFileEx, UnlockFile, LOCKFILE_EXCLUSIVE_LOCK, LOCKFILE_FAIL_IMMEDIATELY,
};

use crate::sys::{AsOpenFile, AsOpenFileExt};

impl<T> AsOpenFileExt for T
where
    T: AsOpenFile,
{
    type BorrowedOpenFile<'a> = BorrowedHandle<'a>
    where
        Self: 'a;
    type OwnedOpenFile = OwnedHandle;

    fn borrow_open_file(&self) -> Self::BorrowedOpenFile<'_> {
        self.as_handle()
    }

    fn acquire_lock_blocking<const WRITE: bool, const BLOCK: bool>(&self) -> io::Result<()> {
        // See: https://stackoverflow.com/a/9186532, https://docs.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-lockfileex
        let handle = self.as_handle().as_raw_handle() as HANDLE;
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

    fn release_lock_blocking(&self) -> io::Result<()> {
        let handle = self.as_handle().as_raw_handle() as HANDLE;
        syscall(unsafe { UnlockFile(handle, 0, 0, 1, 0) })?;
        Ok(())
    }
}
