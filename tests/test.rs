use fd_lock::{AsOpenFile, RwLock, RwLockReadGuard, RwLockWriteGuard};
use paste::paste;
use polonius_the_crab::prelude::*;
use std::fs::File;
use std::io;
use std::io::ErrorKind;
use tempfile::tempdir;

/// An adaptor for [`RwLock::try_read`] and [`RwLock::try_write`] with the same signature as
/// [`RwLock::try_read_owned`] and [`RwLock::try_write_owned`], respectively.
///
/// This enables a shared implementation of tests.
trait RwLockExt<T: AsOpenFile>: Sized {
    fn try_read_ref(&self) -> Result<RwLockReadGuard<'_, T>, (&Self, io::Error)>;
    fn try_write_ref(&mut self) -> Result<RwLockWriteGuard<'_, T>, (&mut Self, io::Error)>;
}

impl<T: AsOpenFile> RwLockExt<T> for RwLock<T> {
    fn try_read_ref(&self) -> Result<RwLockReadGuard<'_, T>, (&Self, io::Error)> {
        self.try_read().map_err(move |err| (self, err))
    }

    fn try_write_ref(&mut self) -> Result<RwLockWriteGuard<'_, T>, (&mut Self, io::Error)> {
        let mut this = self;
        let err = polonius!(|this| -> Result<
            RwLockWriteGuard<'polonius, T>,
            (&'polonius mut RwLock<T>, io::Error),
        > {
            match this.try_write() {
                Ok(ok) => polonius_return!(Ok(ok)),
                Err(err) => err,
            }
        });
        Err((this, err))
    }
}

macro_rules! generate_tests {
    ($($suffix_first:ident)?, $($suffix_second:ident)?) => {
        paste! {
            #[test]
            fn [<read $($suffix_first)? _read $($suffix_second)? _lock>]() {
                let dir = tempdir().unwrap();
                let path = dir.path().join("lockfile");

                let l0 = RwLock::new(File::create(&path).unwrap());
                let l1 = RwLock::new(File::open(path).unwrap());

                let _g0 = l0.[<try_read $($suffix_first)?>]().unwrap();
                let _g1 = l1.[<try_read $($suffix_second)?>]().unwrap();
            }

            #[test]
            fn [<write $($suffix_first)? _write $($suffix_second)? _lock>]() {
                let dir = tempdir().unwrap();
                let path = dir.path().join("lockfile");

                #[allow(unused_mut)]
                let mut l0 = RwLock::new(File::create(&path).unwrap());
                #[allow(unused_mut)]
                let mut l1 = RwLock::new(File::open(path).unwrap());

                let g0 = l0.[<try_write $($suffix_first)?>]().unwrap();
                let (l1, err) = l1.[<try_write $($suffix_second)?>]().unwrap_err();

                assert!(matches!(err.kind(), ErrorKind::WouldBlock));
                drop(g0);

                let _g1 = l1.[<try_write $($suffix_second)?>]().unwrap();
            }

            #[test]
            fn [<read $($suffix_first)? _write $($suffix_second)? _lock>]() {
                let dir = tempdir().unwrap();
                let path = dir.path().join("lockfile");

                let l0 = RwLock::new(File::create(&path).unwrap());
                #[allow(unused_mut)]
                let mut l1 = RwLock::new(File::open(path).unwrap());

                let g0 = l0.[<try_read $($suffix_first)?>]().unwrap();
                let (l1, err) = l1.[<try_write $($suffix_second)?>]().unwrap_err();

                assert!(matches!(err.kind(), ErrorKind::WouldBlock));
                drop(g0);

                let _g1 = l1.[<try_write $($suffix_second)?>]().unwrap();
            }

            #[test]
            fn [<write $($suffix_first)? _read $($suffix_second)? _lock>]() {
                let dir = tempdir().unwrap();
                let path = dir.path().join("lockfile");

                #[allow(unused_mut)]
                let mut l0 = RwLock::new(File::create(&path).unwrap());
                let l1 = RwLock::new(File::open(path).unwrap());

                let g0 = l0.[<try_write $($suffix_first)?>]().unwrap();
                let (l1, err) = l1.[<try_read $($suffix_second)?>]().unwrap_err();

                assert!(matches!(err.kind(), ErrorKind::WouldBlock));
                drop(g0);

                let _g1 = l1.[<try_read $($suffix_second)?>]().unwrap();
            }
        }
    };
}

generate_tests!(_ref, _ref);
generate_tests!(_ref, _owned);
generate_tests!(_owned, _ref);
generate_tests!(_owned, _owned);

#[cfg(windows)]
mod windows {
    use super::*;
    use std::os::windows::fs::OpenOptionsExt;

    #[test]
    fn try_lock_error() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("lockfile");

        // On Windows, opening with an access_mode as 0 will prevent all locking operations from succeeding, simulating an I/O error.
        let mut l0 = RwLock::new(
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
