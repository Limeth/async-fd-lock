use rustix::fd::AsFd;
use rustix::fs::FlockOperation;
use std::io::{self, Error, ErrorKind};

use crate::sys::RwLockTrait;

use super::compatible_unix_lock;

#[derive(Debug)]
pub struct RwLock<T: AsFd> {
    pub(crate) inner: T,
}

impl<T: AsFd> RwLockTrait<T> for RwLock<T> {
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

    fn acquire_lock<const WRITE: bool, const BLOCK: bool>(&self) -> io::Result<()> {
        let fd = self.inner.as_fd();
        let operation = match (WRITE, BLOCK) {
            (false, false) => FlockOperation::NonBlockingLockShared,
            (false, true) => FlockOperation::LockShared,
            (true, false) => FlockOperation::NonBlockingLockExclusive,
            (true, true) => FlockOperation::LockExclusive,
        };
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

    fn release_lock(&self) -> io::Result<()> {
        let fd = self.inner.as_fd();
        compatible_unix_lock(fd, FlockOperation::Unlock)?;
        Ok(())
    }
}
