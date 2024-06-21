use crate::owned_read_guard::OwnedRwLockReadGuard;
use crate::owned_write_guard::OwnedRwLockWriteGuard;
use crate::read_guard::RwLockReadGuard;
use crate::sys::{self, RwLockTrait, RwLockTraitExt};
use crate::write_guard::RwLockWriteGuard;
use std::io;
use std::ops::Deref;
use tokio::task;

/// Advisory reader-writer lock for files.
///
/// This type of lock allows a number of readers or at most one writer at any point
/// in time. The write portion of this lock typically allows modification of the
/// underlying data (exclusive access) and the read portion of this lock typically
/// allows for read-only access (shared access).
#[derive(Debug)]
pub struct RwLock<T: sys::AsOpenFile> {
    pub(crate) lock: sys::RwLock<T>,
}

impl<T: sys::AsOpenFile> RwLock<T> {
    /// Create a new instance.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use fd_lock::RwLock;
    /// use std::fs::File;
    ///
    /// fn main() -> std::io::Result<()> {
    ///     let mut f = RwLock::new(File::open("foo.txt")?);
    ///     Ok(())
    /// }
    /// ```
    #[inline]
    pub fn new(inner: T) -> Self {
        Self {
            lock: sys::RwLock::new(inner),
        }
    }

    /// Locks this lock with shared read access, blocking the current thread
    /// until it can be acquired.
    ///
    /// The calling thread will be blocked until there are no more writers which
    /// hold the lock. There may be other readers currently inside the lock when
    /// this method returns. This method does not provide any guarantees with
    /// respect to the ordering of whether contentious readers or writers will
    /// acquire the lock first.
    ///
    /// Returns an RAII guard which will release this thread's shared access
    /// once it is dropped.
    ///
    /// # Errors
    ///
    /// On Unix this may return an `ErrorKind::Interrupted` if the operation was
    /// interrupted by a signal handler.
    #[inline]
    pub async fn read(&self) -> io::Result<RwLockReadGuard<'_, T>>
    where
        T: Sync,
    {
        let file = self.lock.borrow_open_file().try_clone_to_owned()?;
        task::spawn_blocking(move || sys::RwLock::<T>::acquire_lock_from_file::<false, true>(file))
            .await??;
        Ok(RwLockReadGuard::new(&self.lock))
    }

    /// Attempts to acquire this lock with shared read access.
    ///
    /// If the access could not be granted at this time, then `Err` is returned.
    /// Otherwise, an RAII guard is returned which will release the shared access
    /// when it is dropped.
    ///
    /// This function does not block.
    ///
    /// This function does not provide any guarantees with respect to the ordering
    /// of whether contentious readers or writers will acquire the lock first.
    ///
    /// # Errors
    ///
    /// If the lock is already held and `ErrorKind::WouldBlock` error is returned.
    /// On Unix this may return an `ErrorKind::Interrupted` if the operation was
    /// interrupted by a signal handler.
    #[inline]
    pub async fn try_read(&self) -> io::Result<RwLockReadGuard<'_, T>> {
        self.lock.acquire_lock::<false, false>()?;
        Ok(RwLockReadGuard::new(&self.lock))
    }

    pub async fn read_owned(self) -> Result<OwnedRwLockReadGuard<Self>, (Self, io::Error)>
    where
        T: Sync,
    {
        let file = match self.lock.borrow_open_file().try_clone_to_owned() {
            Ok(file) => file,
            Err(error) => return Err((self, error)),
        };
        let result = task::spawn_blocking(move || {
            sys::RwLock::<T>::acquire_lock_from_file::<false, true>(file)
        })
        .await;
        if let Err(error) = result.map_err(Into::into).and_then(std::convert::identity) {
            return Err((self, error));
        }
        Ok(OwnedRwLockReadGuard::new(self))
    }

    pub async fn try_read_owned(self) -> Result<OwnedRwLockReadGuard<Self>, (Self, io::Error)> {
        if let Err(err) = self.lock.acquire_lock::<false, false>() {
            return Err((self, err));
        }
        Ok(OwnedRwLockReadGuard::new(self))
    }

    /// Locks this lock with exclusive write access, blocking the current thread
    /// until it can be acquired.
    ///
    /// This function will not return while other writers or other readers
    /// currently have access to the lock.
    ///
    /// Returns an RAII guard which will drop the write access of this rwlock
    /// when dropped.
    ///
    /// # Errors
    ///
    /// On Unix this may return an `ErrorKind::Interrupted` if the operation was
    /// interrupted by a signal handler.
    #[inline]
    pub async fn write(&mut self) -> io::Result<RwLockWriteGuard<'_, T>>
    where
        T: Sync,
    {
        let file = self.lock.borrow_open_file().try_clone_to_owned()?;
        task::spawn_blocking(move || sys::RwLock::<T>::acquire_lock_from_file::<true, true>(file))
            .await??;
        Ok(RwLockWriteGuard::new(&mut self.lock))
    }

    /// Attempts to lock this lock with exclusive write access.
    ///
    /// If the lock could not be acquired at this time, then `Err` is returned.
    /// Otherwise, an RAII guard is returned which will release the lock when
    /// it is dropped.
    ///
    /// # Errors
    ///
    /// If the lock is already held and `ErrorKind::WouldBlock` error is returned.
    /// On Unix this may return an `ErrorKind::Interrupted` if the operation was
    /// interrupted by a signal handler.
    #[inline]
    pub async fn try_write(&mut self) -> io::Result<RwLockWriteGuard<'_, T>> {
        self.lock.acquire_lock::<true, false>()?;
        Ok(RwLockWriteGuard::new(&mut self.lock))
    }

    pub async fn write_owned(self) -> Result<OwnedRwLockWriteGuard<Self>, (Self, io::Error)>
    where
        T: Sync,
    {
        let file = match self.lock.borrow_open_file().try_clone_to_owned() {
            Ok(file) => file,
            Err(error) => return Err((self, error)),
        };
        let result = task::spawn_blocking(move || {
            sys::RwLock::<T>::acquire_lock_from_file::<true, true>(file)
        })
        .await;
        if let Err(error) = result.map_err(Into::into).and_then(std::convert::identity) {
            return Err((self, error));
        }
        Ok(OwnedRwLockWriteGuard::new(self))
    }

    pub async fn try_write_owned(self) -> Result<OwnedRwLockWriteGuard<Self>, (Self, io::Error)> {
        if let Err(err) = self.lock.acquire_lock::<true, false>() {
            return Err((self, err));
        }
        Ok(OwnedRwLockWriteGuard::new(self))
    }

    /// Consumes this `RwLock`, returning the underlying data.
    #[inline]
    pub fn into_inner(self) -> T
    where
        T: Sized,
    {
        self.lock.into_inner()
    }
}

impl<T: sys::AsOpenFile> Deref for RwLock<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.lock
    }
}

impl<T: sys::AsOpenFile> crate::rw_lock::RwLockTrait for RwLock<T> {
    type AsOpenFile = T;

    fn into_inner(self) -> Self::AsOpenFile
    where
        Self::AsOpenFile: Sized,
    {
        self.lock.into_inner()
    }

    fn release_lock(&self) -> io::Result<()> {
        self.lock.release_lock()
    }
}
