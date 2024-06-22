use async_trait::async_trait;
use fd_lock::{AsOpenFile, LockRead, LockWrite, RwLockReadGuard, RwLockWriteGuard};
use std::io;
use std::io::ErrorKind;
use std::path::Path;
use std::time::Duration;
use tokio::fs::File;
use tokio::time;
use tokio::time::error::Elapsed;

pub mod blocking {
    pub use fd_lock::blocking::*;
    pub use std::fs::File;
    use std::path::Path;

    pub async fn file_create(path: impl AsRef<Path>) -> std::io::Result<File> {
        File::create(path)
    }

    pub async fn file_open(path: impl AsRef<Path>) -> std::io::Result<File> {
        File::open(path)
    }
}

pub async fn file_create(path: impl AsRef<Path>) -> std::io::Result<File> {
    File::create(path).await
}

pub async fn file_open(path: impl AsRef<Path>) -> std::io::Result<File> {
    File::open(path).await
}

#[async_trait]
pub trait LockReadExt: AsOpenFile {
    async fn try_lock_read_async(self) -> Result<RwLockReadGuard<Self>, (Self, io::Error)>
    where
        Self: Sized + Send + Sync + 'static;
}

#[async_trait]
pub trait LockWriteExt: AsOpenFile {
    async fn try_lock_write_async(self) -> Result<RwLockWriteGuard<Self>, (Self, io::Error)>
    where
        Self: Sized + Send + Sync + 'static;
}

#[async_trait]
impl LockReadExt for std::fs::File {
    async fn try_lock_read_async(self) -> Result<RwLockReadGuard<Self>, (Self, io::Error)>
    where
        Self: Sized + Send + Sync + 'static,
    {
        blocking::LockRead::try_lock_read(self)
    }
}

#[async_trait]
impl LockWriteExt for std::fs::File {
    async fn try_lock_write_async(self) -> Result<RwLockWriteGuard<Self>, (Self, io::Error)>
    where
        Self: Sized + Send + Sync + 'static,
    {
        blocking::LockWrite::try_lock_write(self)
    }
}

#[async_trait]
impl LockReadExt for tokio::fs::File {
    async fn try_lock_read_async(self) -> Result<RwLockReadGuard<Self>, (Self, io::Error)>
    where
        Self: Sized + Send + Sync + 'static,
    {
        LockRead::try_lock_read(self).await
    }
}

#[async_trait]
impl LockWriteExt for tokio::fs::File {
    async fn try_lock_write_async(self) -> Result<RwLockWriteGuard<Self>, (Self, io::Error)>
    where
        Self: Sized + Send + Sync + 'static,
    {
        LockWrite::try_lock_write(self).await
    }
}

#[async_trait]
pub trait AsyncTryClone {
    async fn async_try_clone(&self) -> std::io::Result<Self>
    where
        Self: Sized;
}

#[async_trait]
impl AsyncTryClone for std::fs::File {
    async fn async_try_clone(&self) -> std::io::Result<Self>
    where
        Self: Sized,
    {
        self.try_clone()
    }
}

#[async_trait]
impl AsyncTryClone for tokio::fs::File {
    async fn async_try_clone(&self) -> std::io::Result<Self>
    where
        Self: Sized,
    {
        self.try_clone().await
    }
}

#[async_trait]
pub trait AsOpenFileExt: AsOpenFile {
    async fn timeout_lock_read_async(self) -> Result<RwLockReadGuard<Self>, (Self, io::Error)>
    where
        Self: Sized + Send + Sync + 'static;
    async fn timeout_lock_write_async(self) -> Result<RwLockWriteGuard<Self>, (Self, io::Error)>
    where
        Self: Sized + Send + Sync + 'static;
}

const TIMEOUT: Duration = Duration::from_millis(1000);

#[async_trait]
impl<T> AsOpenFileExt for T
where
    T: AsOpenFile + LockRead + LockWrite + AsyncTryClone,
{
    async fn timeout_lock_read_async(self) -> Result<RwLockReadGuard<T>, (Self, io::Error)>
    where
        T: Sized + Send + Sync + 'static,
    {
        let clone = match self.async_try_clone().await {
            Ok(clone) => clone,
            Err(error) => return Err((self, error)),
        };
        time::timeout(TIMEOUT, self.lock_read())
            .await
            .map_err(move |_: Elapsed| (clone, ErrorKind::WouldBlock.into()))
            .and_then(std::convert::identity)
    }

    async fn timeout_lock_write_async(self) -> Result<RwLockWriteGuard<T>, (Self, io::Error)>
    where
        T: Sized + Send + Sync + 'static,
    {
        let clone = match self.async_try_clone().await {
            Ok(clone) => clone,
            Err(error) => return Err((self, error)),
        };
        time::timeout(TIMEOUT, self.lock_write())
            .await
            .map_err(move |_: Elapsed| (clone, ErrorKind::WouldBlock.into()))
            .and_then(std::convert::identity)
    }
}
