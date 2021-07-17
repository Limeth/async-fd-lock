use crate::read_guard::FileLockReadGuard;
use crate::sys;
use crate::write_guard::FileLockWriteGuard;
use std::io;

/// Advisory reader-writer lock for files.
///
/// This type of lock allows a number of readers or at most one writer at any point
/// in time. The write portion of this lock typically allows modification of the
/// underlying data (exclusive access) and the read portion of this lock typically
/// allows for read-only access (shared access).
#[derive(Debug)]
pub struct FileLock<T: sys::AsRaw> {
    lock: sys::FileLock<T>,
}

impl<T: sys::AsRaw> FileLock<T> {
    /// Create a new instance.
    #[inline]
    pub fn new(inner: T) -> Self {
        Self {
            lock: sys::FileLock::new(inner),
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
    pub fn read(&self) -> io::Result<FileLockReadGuard<'_, T>> {
        let guard = self.lock.read()?;
        Ok(FileLockReadGuard::new(guard))
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
    pub fn try_read(&self) -> io::Result<FileLockReadGuard<'_, T>> {
        let guard = self.lock.try_read()?;
        Ok(FileLockReadGuard::new(guard))
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
    pub fn write(&mut self) -> io::Result<FileLockWriteGuard<'_, T>> {
        let guard = self.lock.write()?;
        Ok(FileLockWriteGuard::new(guard))
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
    pub fn try_write(&mut self) -> io::Result<FileLockWriteGuard<'_, T>> {
        let guard = self.lock.try_write()?;
        Ok(FileLockWriteGuard::new(guard))
    }

    /// Consumes this `FileLock`, returning the underlying data.
    #[inline]
    pub fn into_inner(self) -> T
    where
        T: Sized,
    {
        self.lock.into_inner()
    }
}
