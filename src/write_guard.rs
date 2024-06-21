use std::ops;

use crate::sys::{self, RwLockTraitExt};

/// RAII structure used to release the exclusive write access of a lock when
/// dropped.
///
/// This structure is created by the [`write`] and [`try_write`] methods
/// on [`RwLock`].
///
/// [`write`]: crate::RwLock::write
/// [`try_write`]: crate::RwLock::try_write
/// [`RwLock`]: crate::RwLock
#[must_use = "if unused the RwLock will immediately unlock"]
#[derive(Debug)]
pub struct RwLockWriteGuard<'lock, T: sys::AsOpenFile> {
    lock: &'lock mut sys::RwLock<T>,
}

impl<'lock, T: sys::AsOpenFile> RwLockWriteGuard<'lock, T> {
    pub(crate) fn new(lock: &'lock mut sys::RwLock<T>) -> Self {
        Self { lock }
    }
}

impl<T: sys::AsOpenFile> ops::Deref for RwLockWriteGuard<'_, T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.lock.inner
    }
}

impl<T: sys::AsOpenFile> ops::DerefMut for RwLockWriteGuard<'_, T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.lock.inner
    }
}

/// Release the lock.
impl<T: sys::AsOpenFile> Drop for RwLockWriteGuard<'_, T> {
    #[inline]
    fn drop(&mut self) {
        let _ = self.lock.release_lock();
    }
}
