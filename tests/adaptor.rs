use async_trait::async_trait;
use fd_lock::{
    blocking, AsOpenFile, OwnedRwLockReadGuard, OwnedRwLockWriteGuard, RwLock, RwLockReadGuard,
    RwLockTrait, RwLockWriteGuard,
};
use polonius_the_crab::prelude::*;
use std::io;
use std::io::ErrorKind;
use std::time::Duration;
use tokio::time;
use tokio::time::error::Elapsed;

/// An adaptor for [`RwLock::try_read`] and [`RwLock::try_write`] with the same signature as
/// [`RwLock::try_read_owned`] and [`RwLock::try_write_owned`], respectively.
///
/// This enables a shared implementation of tests.
#[async_trait]
pub trait TryRwLock<T: AsOpenFile>: Sized + RwLockTrait {
    async fn try_read_own(self) -> Result<OwnedRwLockReadGuard<Self>, (Self, io::Error)>
    where
        T: Send + Sync + 'static;
    async fn try_write_own(self) -> Result<OwnedRwLockWriteGuard<Self>, (Self, io::Error)>
    where
        T: Send + Sync + 'static;
    async fn try_read_ref(&self) -> Result<RwLockReadGuard<'_, T>, (&Self, io::Error)>
    where
        T: Send + Sync + 'static;
    async fn try_write_ref(&mut self) -> Result<RwLockWriteGuard<'_, T>, (&mut Self, io::Error)>
    where
        T: Send + Sync + 'static;
}

#[async_trait]
impl<T: AsOpenFile> TryRwLock<T> for blocking::RwLock<T> {
    async fn try_read_own(self) -> Result<OwnedRwLockReadGuard<Self>, (Self, io::Error)>
    where
        T: Send + Sync + 'static,
    {
        self.try_read_owned()
    }

    async fn try_write_own(self) -> Result<OwnedRwLockWriteGuard<Self>, (Self, io::Error)>
    where
        T: Send + Sync + 'static,
    {
        self.try_write_owned()
    }

    async fn try_read_ref(&self) -> Result<RwLockReadGuard<'_, T>, (&Self, io::Error)>
    where
        T: Send + Sync + 'static,
    {
        self.try_read().map_err(move |err| (self, err))
    }

    async fn try_write_ref(&mut self) -> Result<RwLockWriteGuard<'_, T>, (&mut Self, io::Error)>
    where
        T: Send + Sync + 'static,
    {
        let mut this = self;
        let err = polonius!(|this| -> Result<
            RwLockWriteGuard<'polonius, T>,
            (&'polonius mut blocking::RwLock<T>, io::Error),
        > {
            match this.try_write() {
                Ok(ok) => polonius_return!(Ok(ok)),
                Err(err) => err,
            }
        });
        Err((this, err))
    }
}

#[async_trait]
impl<T: AsOpenFile> TryRwLock<T> for RwLock<T> {
    async fn try_read_own(self) -> Result<OwnedRwLockReadGuard<Self>, (Self, io::Error)>
    where
        T: Send + Sync + 'static,
    {
        self.try_read_owned().await
    }

    async fn try_write_own(self) -> Result<OwnedRwLockWriteGuard<Self>, (Self, io::Error)>
    where
        T: Send + Sync + 'static,
    {
        self.try_write_owned().await
    }

    async fn try_read_ref(&self) -> Result<RwLockReadGuard<'_, T>, (&Self, io::Error)>
    where
        T: Send + Sync + 'static,
    {
        self.try_read().await.map_err(move |err| (self, err))
    }

    async fn try_write_ref(&mut self) -> Result<RwLockWriteGuard<'_, T>, (&mut Self, io::Error)>
    where
        T: Send + Sync + 'static,
    {
        // Safety:
        // `this` is only used when returning an `Err(_)` variant, while `&mut self` is no longer
        // being used. This is a work-around for the so-called [Problem Case #3](problem-case-3)
        // of the current borrow checker. The crate `polonius-the-crab` cannot be used here because
        // of async.
        //
        // [problem-case-3]: https://github.com/rust-lang/rfcs/blob/master/text/2094-nll.md#problem-case-3-conditional-control-flow-across-functions
        let this = unsafe { &mut *(self as *mut _) };
        let err = match self.try_write().await {
            Ok(ok) => return Ok(ok),
            Err(err) => err,
        };
        Err((this, err))
    }
}

#[async_trait]
pub trait TimeoutRwLock<T: AsOpenFile>: Sized + RwLockTrait {
    async fn timeout_read_ref(&self) -> Result<RwLockReadGuard<'_, T>, (&Self, io::Error)>
    where
        T: Send + Sync + 'static;
    async fn timeout_write_ref(
        &mut self,
    ) -> Result<RwLockWriteGuard<'_, T>, (&mut Self, io::Error)>
    where
        T: Send + Sync + 'static;
}

const TIMEOUT: Duration = Duration::from_millis(1000);

#[async_trait]
impl<T: AsOpenFile> TimeoutRwLock<T> for RwLock<T> {
    async fn timeout_read_ref(&self) -> Result<RwLockReadGuard<'_, T>, (&Self, io::Error)>
    where
        T: Send + Sync + 'static,
    {
        time::timeout(TIMEOUT, self.read())
            .await
            .map_err(|_: Elapsed| ErrorKind::WouldBlock.into())
            .and_then(std::convert::identity)
            .map_err(|err| (self, err))
    }

    async fn timeout_write_ref(&mut self) -> Result<RwLockWriteGuard<'_, T>, (&mut Self, io::Error)>
    where
        T: Send + Sync + 'static,
    {
        // Safety:
        // `this` is only used when returning an `Err(_)` variant, while `&mut self` is no longer
        // being used. This is a work-around for the so-called [Problem Case #3](problem-case-3)
        // of the current borrow checker. The crate `polonius-the-crab` cannot be used here because
        // of async.
        //
        // [problem-case-3]: https://github.com/rust-lang/rfcs/blob/master/text/2094-nll.md#problem-case-3-conditional-control-flow-across-functions
        let this = unsafe { &mut *(self as *mut _) };
        time::timeout(TIMEOUT, self.write())
            .await
            .map_err(|_: Elapsed| ErrorKind::WouldBlock.into())
            .and_then(std::convert::identity)
            .map_err(|err| (this, err))
    }
}
