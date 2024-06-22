use async_trait::async_trait;
use fd_lock::{
    blocking, AsOpenFile, OwnedRwLockReadGuard, OwnedRwLockWriteGuard, RwLock, RwLockReadGuard,
    RwLockTrait, RwLockWriteGuard,
};
use paste::paste;
use polonius_the_crab::prelude::*;
use std::fs::File;
use std::io;
use std::io::ErrorKind;
use std::time::Duration;
use tempfile::tempdir;
use tokio::time;
use tokio::time::error::Elapsed;

/// An adaptor for [`RwLock::try_read`] and [`RwLock::try_write`] with the same signature as
/// [`RwLock::try_read_owned`] and [`RwLock::try_write_owned`], respectively.
///
/// This enables a shared implementation of tests.
#[async_trait]
trait TryRwLock<T: AsOpenFile>: Sized + RwLockTrait {
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
trait TimeoutRwLock<T: AsOpenFile>: Sized + RwLockTrait {
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

macro_rules! generate_tests {
    ($($blocking_first:ident)?, $prefix_first:ident, $suffix_first:ident; $($blocking_second:ident)?, $prefix_second:ident, $suffix_second:ident) => {
        paste! {
            #[tokio::test]
            async fn [<$($blocking_first _)? $prefix_first _read_ $suffix_first _ $($blocking_second _)? $prefix_second _read_ $suffix_second _lock>]() {
                let dir = tempdir().unwrap();
                let path = dir.path().join("lockfile");

                let l0 = $($blocking_first::)? RwLock::new(File::create(&path).unwrap());
                let l1 = $($blocking_second::)? RwLock::new(File::open(path).unwrap());

                let _g0 = l0.[<$prefix_first _read_ $suffix_first>]().await.unwrap();
                let _g1 = l1.[<$prefix_second _read_ $suffix_second>]().await.unwrap();
            }

            #[tokio::test]
            async fn [<$($blocking_first _)? $prefix_first _write_ $suffix_first _ $($blocking_second _)? $prefix_second _write_ $suffix_second _lock>]() {
                let dir = tempdir().unwrap();
                let path = dir.path().join("lockfile");

                #[allow(unused_mut)]
                let mut l0 = $($blocking_first::)? RwLock::new(File::create(&path).unwrap());
                #[allow(unused_mut)]
                let mut l1 = $($blocking_second::)? RwLock::new(File::open(path).unwrap());

                let g0 = l0.[<$prefix_first _write_ $suffix_first>]().await.unwrap();
                let (l1, err) = l1.[<$prefix_second _write_ $suffix_second>]().await.unwrap_err();

                assert!(matches!(err.kind(), ErrorKind::WouldBlock));
                drop(g0);

                let _g1 = l1.[<$prefix_second _write_ $suffix_second>]().await.unwrap();
            }

            #[tokio::test]
            async fn [<$($blocking_first _)? $prefix_first _read_ $suffix_first _ $($blocking_second _)? $prefix_second _write_ $suffix_second _lock>]() {
                let dir = tempdir().unwrap();
                let path = dir.path().join("lockfile");

                let l0 = $($blocking_first::)? RwLock::new(File::create(&path).unwrap());
                #[allow(unused_mut)]
                let mut l1 = $($blocking_second::)? RwLock::new(File::open(path).unwrap());

                let g0 = l0.[<$prefix_first _read_ $suffix_first>]().await.unwrap();
                let (l1, err) = l1.[<$prefix_second _write_ $suffix_second>]().await.unwrap_err();

                assert!(matches!(err.kind(), ErrorKind::WouldBlock));
                drop(g0);

                let _g1 = l1.[<$prefix_second _write_ $suffix_second>]().await.unwrap();
            }

            #[tokio::test]
            async fn [<$($blocking_first _)? $prefix_first _write_ $suffix_first _ $($blocking_second _)? $prefix_second _read_ $suffix_second _lock>]() {
                let dir = tempdir().unwrap();
                let path = dir.path().join("lockfile");

                #[allow(unused_mut)]
                let mut l0 = $($blocking_first::)? RwLock::new(File::create(&path).unwrap());
                let l1 = $($blocking_second::)? RwLock::new(File::open(path).unwrap());

                let g0 = l0.[<$prefix_first _write_ $suffix_first>]().await.unwrap();
                let (l1, err) = l1.[<$prefix_second _read_ $suffix_second>]().await.unwrap_err();

                assert!(matches!(err.kind(), ErrorKind::WouldBlock));
                drop(g0);

                let _g1 = l1.[<$prefix_second _read_ $suffix_second>]().await.unwrap();
            }
        }
    };
}

generate_tests!(blocking, try,     ref; blocking, try,     ref);
generate_tests!(blocking, try,     ref;         , try,     ref);
generate_tests!(        , try,     ref; blocking, try,     ref);
generate_tests!(        , try,     ref;         , try,     ref);
generate_tests!(blocking, try,     ref; blocking, try,     own);
generate_tests!(blocking, try,     ref;         , try,     own);
generate_tests!(        , try,     ref; blocking, try,     own);
generate_tests!(        , try,     ref;         , try,     own);
generate_tests!(blocking, try,     own; blocking, try,     ref);
generate_tests!(blocking, try,     own;         , try,     ref);
generate_tests!(        , try,     own; blocking, try,     ref);
generate_tests!(        , try,     own;         , try,     ref);
generate_tests!(blocking, try,     own; blocking, try,     own);
generate_tests!(blocking, try,     own;         , try,     own);
generate_tests!(        , try,     own; blocking, try,     own);
generate_tests!(        , try,     own;         , try,     own);

generate_tests!(blocking, try,     ref;         , timeout, ref);
generate_tests!(        , try,     ref;         , timeout, ref);
generate_tests!(blocking, try,     own;         , timeout, ref);
generate_tests!(        , try,     own;         , timeout, ref);

generate_tests!(        , timeout, ref; blocking, try,     ref);
generate_tests!(        , timeout, ref;         , try,     ref);
generate_tests!(        , timeout, ref; blocking, try,     own);
generate_tests!(        , timeout, ref;         , try,     own);

generate_tests!(        , timeout, ref;         , timeout, ref);

#[cfg(windows)]
mod windows {
    use super::*;
    use std::os::windows::fs::OpenOptionsExt;

    #[test]
    fn try_lock_error() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("lockfile");

        // On Windows, opening with an access_mode as 0 will prevent all locking operations from succeeding, simulating an I/O error.
        let mut l0 = blocking::RwLock::new(
            File::options()
                .create(true)
                .read(true)
                .write(true)
                .truncate(true)
                .access_mode(0)
                .open(path)
                .unwrap(),
        );

        let err1 = l0.try_read().unwrap_err();
        assert!(matches!(err1.kind(), ErrorKind::PermissionDenied));

        let err2 = l0.try_write().unwrap_err();
        assert!(matches!(err2.kind(), ErrorKind::PermissionDenied));
    }
}
