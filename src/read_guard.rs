use std::{
    io::{self, Read},
    pin::Pin,
};

use cfg_if::cfg_if;
use pin_project::{pin_project, pinned_drop};

use crate::sys::{AsOpenFile, AsOpenFileExt, RwLockGuard};

/// Onwed version of `RwLockReadGuard`
///
/// # Panics
///
/// Dropping this type may panic if the lock fails to unlock.
#[must_use = "if unused the RwLock will immediately unlock"]
#[derive(Debug)]
#[pin_project(PinnedDrop)]
pub struct RwLockReadGuard<T: AsOpenFile> {
    #[pin]
    file: Option<T>,
}

impl<T: AsOpenFile> RwLockReadGuard<T> {
    pub(crate) fn new<F: AsOpenFile>(file: T, guard: RwLockGuard<F>) -> Self {
        guard.defuse();
        Self { file: Some(file) }
    }

    pub fn inner(&self) -> &T {
        self.file
            .as_ref()
            .expect("file only removed during release")
    }

    pub fn inner_mut(&mut self) -> &mut T {
        self.file
            .as_mut()
            .expect("file only removed during release")
    }

    pub fn inner_pin_mut(self: Pin<&mut Self>) -> Pin<&mut T> {
        self.project()
            .file
            .as_pin_mut()
            .expect("file only removed during release")
    }

    /// Releases the lock, returning the inner file.
    pub fn release(mut self) -> io::Result<T> {
        let file = self.file.take().expect("file only removed during release");
        file.release_lock_blocking()?;
        Ok(file)
    }
}

/// Delegate [`Read`] to the inner file.
impl<T: AsOpenFile + Read> Read for RwLockReadGuard<T> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.inner_mut().read(buf)
    }

    fn read_vectored(&mut self, bufs: &mut [io::IoSliceMut<'_>]) -> io::Result<usize> {
        self.inner_mut().read_vectored(bufs)
    }

    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> io::Result<usize> {
        self.inner_mut().read_to_end(buf)
    }

    fn read_to_string(&mut self, buf: &mut String) -> io::Result<usize> {
        self.inner_mut().read_to_string(buf)
    }

    fn read_exact(&mut self, buf: &mut [u8]) -> io::Result<()> {
        self.inner_mut().read_exact(buf)
    }

    fn by_ref(&mut self) -> &mut Self
    where
        Self: Sized,
    {
        self
    }
}

cfg_if! {
    if #[cfg(feature = "async")] {
        use tokio::io::AsyncRead;

        /// Delegate [`AsyncRead`] to the inner file.
        impl<T: AsOpenFile + AsyncRead> AsyncRead for RwLockReadGuard<T> {
            fn poll_read(
                self: Pin<&mut Self>,
                cx: &mut std::task::Context<'_>,
                buf: &mut tokio::io::ReadBuf<'_>,
            ) -> std::task::Poll<io::Result<()>> {
                self.inner_pin_mut().poll_read(cx, buf)
            }
        }
    }
}

/// Release the lock if it was not already released, as indicated by a `None`.
#[pinned_drop]
impl<T: AsOpenFile> PinnedDrop for RwLockReadGuard<T> {
    #[inline]
    fn drop(self: Pin<&mut Self>) {
        if let Some(file) = self.project().file.as_pin_mut() {
            let _ = file.release_lock_blocking();
        }
    }
}
