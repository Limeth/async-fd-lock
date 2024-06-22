use adaptor::{TimeoutRwLock, TryRwLock};
use fd_lock::{
    blocking, RwLock,
};
use paste::paste;
use std::fs::File;
use std::io::ErrorKind;
use tempfile::tempdir;

mod adaptor;

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
