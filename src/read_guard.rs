use std::ops::{Deref, DerefMut};

use crate::sys::{AsOpenFile, AsOpenFileExt, RwLockGuard};

/// Onwed version of `RwLockReadGuard`
///
/// # Panics
///
/// Dropping this type may panic if the lock fails to unlock.
#[must_use = "if unused the RwLock will immediately unlock"]
#[derive(Debug)]
pub struct RwLockReadGuard<T: AsOpenFile> {
    file: T,
}

impl<T: AsOpenFile> RwLockReadGuard<T> {
    pub(crate) fn new<F: AsOpenFile>(file: T, guard: RwLockGuard<F>) -> Self {
        guard.defuse();
        Self { file }
    }
}

impl<T: AsOpenFile> Deref for RwLockReadGuard<T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.file
    }
}

impl<T: AsOpenFile> DerefMut for RwLockReadGuard<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.file
    }
}

/// Release the lock.
impl<T: AsOpenFile> Drop for RwLockReadGuard<T> {
    #[inline]
    fn drop(&mut self) {
        let _ = self.file.release_lock_blocking();
    }
}
