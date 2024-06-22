mod utils;

use rustix::fd::{BorrowedFd, OwnedFd};
use rustix::fs::FlockOperation;
use std::io::{self, Error, ErrorKind};
use utils::*;

use crate::sys::{AsOpenFile, AsOpenFileExt};

use super::RwLockGuard;

impl<T> AsOpenFileExt for T
where
    T: AsOpenFile,
{
    type BorrowedOpenFile<'a> = BorrowedFd<'a>
    where
        Self: 'a;
    type OwnedOpenFile = OwnedFd;

    fn borrow_open_file(&self) -> Self::BorrowedOpenFile<'_> {
        self.as_fd()
    }

    fn acquire_lock_blocking<const WRITE: bool, const BLOCK: bool>(
        &self,
    ) -> io::Result<RwLockGuard<Self::OwnedOpenFile>> {
        let handle_clone = self.as_fd().try_clone_to_owned()?;
        let operation = match (WRITE, BLOCK) {
            (false, false) => FlockOperation::NonBlockingLockShared,
            (false, true) => FlockOperation::LockShared,
            (true, false) => FlockOperation::NonBlockingLockExclusive,
            (true, true) => FlockOperation::LockExclusive,
        };
        let fd = self.as_fd();
        let result = compatible_unix_lock(fd, operation);
        if BLOCK {
            result?;
        } else {
            result.map_err(|err| match err.kind() {
                ErrorKind::AlreadyExists => ErrorKind::WouldBlock.into(),
                _ => Error::from(err),
            })?;
        }
        Ok(RwLockGuard::new(handle_clone))
    }

    fn release_lock_blocking(&self) -> io::Result<()> {
        let fd = self.as_fd();
        compatible_unix_lock(fd, FlockOperation::Unlock)?;
        Ok(())
    }
}
