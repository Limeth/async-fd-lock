use std::ops;

use crate::rw_lock::RwLockTrait;

/// Onwed version of `RwLockWriteGuard`
///
/// # Panics
///
/// Dropping this type may panic if the lock fails to unlock.
#[must_use = "if unused the RwLock will immediately unlock"]
#[derive(Debug)]
pub struct OwnedRwLockWriteGuard<L: RwLockTrait> {
    lock: L,
}

impl<L: RwLockTrait> OwnedRwLockWriteGuard<L> {
    pub(crate) fn new(lock: L) -> Self {
        Self { lock }
    }
}

impl<L: RwLockTrait> ops::Deref for OwnedRwLockWriteGuard<L> {
    type Target = L::AsOpenFile;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.lock
    }
}

/// Release the lock.
impl<L: RwLockTrait> Drop for OwnedRwLockWriteGuard<L> {
    #[inline]
    fn drop(&mut self) {
        let _ = self.lock.release_lock();
    }
}
