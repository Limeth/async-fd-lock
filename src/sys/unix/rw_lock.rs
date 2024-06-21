use rustix::fd::{AsFd, BorrowedFd, OwnedFd};
use rustix::fs::FlockOperation;
use std::io::{self, Error, ErrorKind};
use std::ops::Deref;

use crate::sys::{AsOpenFile, RwLockTrait};

use super::compatible_unix_lock;

#[derive(Debug)]
pub struct RwLock<T: AsFd> {
    pub(crate) inner: T,
}

impl<T: AsFd> Deref for RwLock<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T: AsFd> RwLockTrait for RwLock<T> {
    type AsOpenFile = T;
    type BorrowedOpenFile<'a> = BorrowedFd<'a> where Self: 'a;
    type OwnedOpenFile = OwnedFd;

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
        let operation = match (WRITE, BLOCK) {
            (false, false) => FlockOperation::NonBlockingLockShared,
            (false, true) => FlockOperation::LockShared,
            (true, false) => FlockOperation::NonBlockingLockExclusive,
            (true, true) => FlockOperation::LockExclusive,
        };
        let fd = handle.as_fd();
        let result = compatible_unix_lock(fd, operation);
        if BLOCK {
            result?;
        } else {
            result.map_err(|err| match err.kind() {
                ErrorKind::AlreadyExists => ErrorKind::WouldBlock.into(),
                _ => Error::from(err),
            })?;
        }
        Ok(())
    }

    fn borrow_open_file(&self) -> Self::BorrowedOpenFile<'_> {
        self.inner.as_fd()
    }

    fn release_lock_from_file(handle: &impl AsOpenFile) -> io::Result<()> {
        let fd = handle.as_fd();
        compatible_unix_lock(fd, FlockOperation::Unlock)?;
        Ok(())
    }
}
