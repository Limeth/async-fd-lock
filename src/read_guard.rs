use std::ops;

use crate::sys::{self, RwLockTrait};

/// RAII structure used to release the shared read access of a lock when
/// dropped.
///
/// This structure is created by the [`read`] and [`try_read`] methods on
/// [`RwLock`].
///
/// [`read`]: crate::RwLock::read
/// [`try_read`]: crate::RwLock::try_read
/// [`RwLock`]: crate::RwLock
#[must_use = "if unused the RwLock will immediately unlock"]
#[derive(Debug)]
pub struct RwLockReadGuard<'lock, T: sys::AsOpenFile> {
    lock: &'lock sys::RwLock<T>,
}

impl<'lock, T: sys::AsOpenFile> RwLockReadGuard<'lock, T> {
    pub(crate) fn new(lock: &'lock sys::RwLock<T>) -> Self {
        Self { lock }
    }
}

impl<T: sys::AsOpenFile> ops::Deref for RwLockReadGuard<'_, T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.lock.inner
    }
}

/// Release the lock.
impl<T: sys::AsOpenFile> Drop for RwLockReadGuard<'_, T> {
    #[inline]
    fn drop(&mut self) {
        let _ = self.lock.release_lock();
    }
}
