//! Advisory reader-writer locks for files.
//!
//! # Notes on Advisory Locks
//!
//! "advisory locks" are locks which programs must opt-in to adhere to. This
//! means that they can be used to coordinate file access, but not prevent
//! access. Use this to coordinate file access between multiple instances of the
//! same program. But do not use this to prevent actors from accessing or
//! modifying files.
//!
//! # Example
//!
//! ```ignore
//! use tokio::fs::File;
//! use tokio::io::{AsyncReadExt, AsyncWriteExt};
//! use fd_lock::RwLock;
//!
//! # tokio_test::block_on(async {
//! // Create an async advisory file lock.
//! let mut f = RwLock::new(File::open("foo.txt").await?);
//!
//! // Lock it for reading.
//! {
//!     let mut read_guard_1 = f.read().await?;
//!     let mut read_guard_2 = f.read().await?;
//!     let byte_1 = (&*read_guard_1).read_u8().await?;
//!     let byte_2 = read_guard_2.read_u8().await?;
//! }
//!
//! // Lock it for writing.
//! {
//!     let mut write_guard = f.write().await?;
//!     write_guard.write(b"chashu cat").await?;
//! }
//! # std::io::Result::Ok(())
//! # }).unwrap();
//! ```
#![forbid(future_incompatible)]
#![deny(missing_debug_implementations, nonstandard_style)]
#![cfg_attr(doc, warn(missing_docs, rustdoc::missing_doc_code_examples))]

use std::io;
use sys::AsOpenFileExt;

mod read_guard;
mod write_guard;

pub(crate) mod sys;

pub use nonblocking::*;
pub use read_guard::RwLockReadGuard;
// pub use rw_lock::*;
pub use sys::AsOpenFile;
pub use write_guard::RwLockWriteGuard;

pub type LockReadResult<T> = Result<RwLockReadGuard<T>, (T, io::Error)>;
pub type LockWriteResult<T> = Result<RwLockWriteGuard<T>, (T, io::Error)>;

pub mod blocking {
    use super::*;

    pub trait LockRead: AsOpenFile + std::io::Read {
        fn lock_read(self) -> LockReadResult<Self>
        where
            Self: Sized;

        fn try_lock_read(self) -> LockReadResult<Self>
        where
            Self: Sized;
    }

    pub trait LockWrite: AsOpenFile + std::io::Write {
        fn lock_write(self) -> LockWriteResult<Self>
        where
            Self: Sized;

        fn try_lock_write(self) -> LockWriteResult<Self>
        where
            Self: Sized;
    }

    impl<T> LockRead for T
    where
        T: AsOpenFile + std::io::Read,
    {
        fn lock_read(self) -> LockReadResult<Self> {
            if let Err(err) = self.acquire_lock_blocking::<false, true>() {
                return Err((self, err));
            }
            Ok(RwLockReadGuard::new(self))
        }

        fn try_lock_read(self) -> LockReadResult<Self> {
            if let Err(err) = self.acquire_lock_blocking::<false, false>() {
                return Err((self, err));
            }
            Ok(RwLockReadGuard::new(self))
        }
    }

    impl<T> LockWrite for T
    where
        T: AsOpenFile + std::io::Write,
    {
        fn lock_write(self) -> LockWriteResult<Self> {
            if let Err(err) = self.acquire_lock_blocking::<true, true>() {
                return Err((self, err));
            }
            Ok(RwLockWriteGuard::new(self))
        }

        fn try_lock_write(self) -> LockWriteResult<Self> {
            if let Err(err) = self.acquire_lock_blocking::<true, false>() {
                return Err((self, err));
            }
            Ok(RwLockWriteGuard::new(self))
        }
    }
}

pub mod nonblocking {
    use super::*;
    use async_trait::async_trait;
    use sys::{AsOpenFileExt, LockGuard};

    async fn lock<const WRITE: bool, const BLOCK: bool, T>(
        file: &T,
    ) -> Result<LockGuard<T>, io::Error>
    where
        T: AsOpenFile + Sync + 'static,
    {
        let handle = file.borrow_open_file().try_clone_to_owned()?;
        let (sync_send, async_recv) = tokio::sync::oneshot::channel();
        tokio::task::spawn_blocking(move || {
            let guard = handle
                .acquire_lock_blocking::<WRITE, BLOCK>()
                .map(|()| LockGuard::<T>::new(handle));
            let result = sync_send.send(guard);
            drop(result); // If the guard cannot be sent to the async task, release the lock immediately.
        });
        async_recv
            .await
            .expect("the blocking task is not cancelable")
    }

    #[async_trait]
    pub trait LockRead: AsOpenFile + tokio::io::AsyncRead {
        async fn lock_read(self) -> LockReadResult<Self>
        where
            Self: Sized;

        async fn try_lock_read(self) -> LockReadResult<Self>
        where
            Self: Sized;
    }

    #[async_trait]
    pub trait LockWrite: AsOpenFile + tokio::io::AsyncWrite {
        async fn lock_write(self) -> LockWriteResult<Self>
        where
            Self: Sized;

        async fn try_lock_write(self) -> LockWriteResult<Self>
        where
            Self: Sized;
    }

    #[async_trait]
    impl<T> LockRead for T
    where
        T: AsOpenFile + tokio::io::AsyncRead + Send + Sync + 'static,
    {
        async fn lock_read(self) -> LockReadResult<Self> {
            let guard = match lock::<false, true, _>(&self).await {
                Ok(guard) => guard,
                Err(error) => return Err((self, error)),
            };
            let guard = guard.defuse_with(|_| RwLockReadGuard::new(self));
            Ok(guard)
        }

        async fn try_lock_read(self) -> LockReadResult<Self> {
            let guard = match lock::<false, false, _>(&self).await {
                Ok(guard) => guard,
                Err(error) => return Err((self, error)),
            };
            let guard = guard.defuse_with(|_| RwLockReadGuard::new(self));
            Ok(guard)
        }
    }

    #[async_trait]
    impl<T> LockWrite for T
    where
        T: AsOpenFile + tokio::io::AsyncWrite + Send + Sync + 'static,
    {
        async fn lock_write(self) -> LockWriteResult<Self> {
            let guard = match lock::<true, true, _>(&self).await {
                Ok(guard) => guard,
                Err(error) => return Err((self, error)),
            };
            let guard = guard.defuse_with(|_| RwLockWriteGuard::new(self));
            Ok(guard)
        }

        async fn try_lock_write(self) -> LockWriteResult<Self> {
            let guard = match lock::<true, false, _>(&self).await {
                Ok(guard) => guard,
                Err(error) => return Err((self, error)),
            };
            let guard = guard.defuse_with(|_| RwLockWriteGuard::new(self));
            Ok(guard)
        }
    }
}
