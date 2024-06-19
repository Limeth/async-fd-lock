use rustix::fd::{AsFd, AsRawFd};
use rustix::fs::FlockOperation;
use std::io::{self, Error, ErrorKind};

use super::{compatible_unix_lock, RwLockReadGuard, RwLockWriteGuard};

#[derive(Debug)]
pub struct RwLock<T: AsFd> {
    pub(crate) inner: T,
}

impl<T: AsFd> RwLock<T> {
    #[inline]
    pub fn new(inner: T) -> Self {
        RwLock { inner }
    }

    #[inline]
    pub fn write(&mut self) -> io::Result<RwLockWriteGuard<'_, T>> {
        self.acquire_lock::<true, true>()?;
        Ok(RwLockWriteGuard::new(self))
    }

    #[inline]
    pub fn try_write(&mut self) -> Result<RwLockWriteGuard<'_, T>, Error> {
        self.acquire_lock::<true, false>()
            .map_err(|err| match err.kind() {
                ErrorKind::AlreadyExists => ErrorKind::WouldBlock.into(),
                _ => err,
            })?;
        Ok(RwLockWriteGuard::new(self))
    }

    #[inline]
    pub fn read(&self) -> io::Result<RwLockReadGuard<'_, T>> {
        self.acquire_lock::<false, true>()?;
        Ok(RwLockReadGuard::new(self))
    }

    #[inline]
    pub fn try_read(&self) -> Result<RwLockReadGuard<'_, T>, Error> {
        self.acquire_lock::<false, false>()
            .map_err(|err| match err.kind() {
                ErrorKind::AlreadyExists => ErrorKind::WouldBlock.into(),
                _ => err,
            })?;
        Ok(RwLockReadGuard::new(self))
    }

    #[inline]
    pub fn into_inner(self) -> T
    where
        T: Sized,
    {
        self.inner
    }

    pub(crate) fn acquire_lock<const WRITE: bool, const BLOCK: bool>(&self) -> io::Result<()> {
        let fd = self.inner.as_fd();
        let operation = match (WRITE, BLOCK) {
            (false, false) => FlockOperation::NonBlockingLockShared,
            (false, true) => FlockOperation::LockShared,
            (true, false) => FlockOperation::NonBlockingLockExclusive,
            (true, true) => FlockOperation::LockExclusive,
        };
        compatible_unix_lock(fd, operation)?;
        Ok(())
    }

    pub(crate) fn release_lock(&self) -> io::Result<()> {
        let fd = self.inner.as_fd();
        compatible_unix_lock(fd, FlockOperation::Unlock)?;
        Ok(())
    }
}
