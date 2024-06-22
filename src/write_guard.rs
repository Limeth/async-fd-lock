use std::ops::{Deref, DerefMut};

use crate::sys::{AsOpenFile, AsOpenFileExt, RwLockGuard};

/// Onwed version of `RwLockWriteGuard`
///
/// # Panics
///
/// Dropping this type may panic if the lock fails to unlock.
#[must_use = "if unused the RwLock will immediately unlock"]
#[derive(Debug)]
pub struct RwLockWriteGuard<T: AsOpenFile> {
    file: T,
}

impl<T: AsOpenFile> RwLockWriteGuard<T> {
    pub(crate) fn new<F: AsOpenFile>(file: T, guard: RwLockGuard<F>) -> Self {
        guard.defuse();
        Self { file }
    }
}

impl<T: AsOpenFile> Deref for RwLockWriteGuard<T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.file
    }
}

impl<T: AsOpenFile> DerefMut for RwLockWriteGuard<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.file
    }
}

/// Release the lock.
impl<T: AsOpenFile> Drop for RwLockWriteGuard<T> {
    #[inline]
    fn drop(&mut self) {
        let _ = self.file.release_lock_blocking();
    }
}
