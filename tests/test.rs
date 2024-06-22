use adaptor::*;
use futures::future::join_all;
use paste::paste;
use std::io::ErrorKind;
use tempfile::tempdir;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

mod adaptor;

macro_rules! generate_tests {
    ($($blocking_first:ident)?, $prefix_first:ident; $($blocking_second:ident)?, $prefix_second:ident) => {
        paste! {
            #[tokio::test]
            async fn [<$($blocking_first _)? $prefix_first _read_ $($blocking_second _)? $prefix_second _read_lock>]() {
                let dir = tempdir().unwrap();
                let path = dir.path().join("lockfile");

                let l0 = $($blocking_first::)? file_create(&path).await.unwrap();
                let l1 = $($blocking_second::)? file_open(path).await.unwrap();

                let _g0 = l0.[<$prefix_first _lock_read_async>]().await.unwrap();
                let _g1 = l1.[<$prefix_second _lock_read_async>]().await.unwrap();
            }

            #[tokio::test]
            async fn [<$($blocking_first _)? $prefix_first _write_ $($blocking_second _)? $prefix_second _write_lock>]() {
                let dir = tempdir().unwrap();
                let path = dir.path().join("lockfile");

                #[allow(unused_mut)]
                let mut l0 = $($blocking_first::)? file_create(&path).await.unwrap();
                #[allow(unused_mut)]
                let mut l1 = $($blocking_second::)? file_open(path).await.unwrap();

                let g0 = l0.[<$prefix_first _lock_write_async>]().await.unwrap();
                let (l1, err) = l1.[<$prefix_second _lock_write_async>]().await.unwrap_err();

                assert!(matches!(err.kind(), ErrorKind::WouldBlock));
                drop(g0);

                let _g1 = l1.[<$prefix_second _lock_write_async>]().await.unwrap();
            }

            #[tokio::test]
            async fn [<$($blocking_first _)? $prefix_first _read_ $($blocking_second _)? $prefix_second _write_lock>]() {
                let dir = tempdir().unwrap();
                let path = dir.path().join("lockfile");

                let l0 = $($blocking_first::)? file_create(&path).await.unwrap();
                #[allow(unused_mut)]
                let mut l1 = $($blocking_second::)? file_open(path).await.unwrap();

                let g0 = l0.[<$prefix_first _lock_read_async>]().await.unwrap();
                let (l1, err) = l1.[<$prefix_second _lock_write_async>]().await.unwrap_err();

                assert!(matches!(err.kind(), ErrorKind::WouldBlock));
                drop(g0);

                let _g1 = l1.[<$prefix_second _lock_write_async>]().await.unwrap();
            }

            #[tokio::test]
            async fn [<$($blocking_first _)? $prefix_first _write_ $($blocking_second _)? $prefix_second _read_lock>]() {
                let dir = tempdir().unwrap();
                let path = dir.path().join("lockfile");

                #[allow(unused_mut)]
                let mut l0 = $($blocking_first::)? file_create(&path).await.unwrap();
                let l1 = $($blocking_second::)? file_open(path).await.unwrap();

                let g0 = l0.[<$prefix_first _lock_write_async>]().await.unwrap();
                let (l1, err) = l1.[<$prefix_second _lock_read_async>]().await.unwrap_err();

                assert!(matches!(err.kind(), ErrorKind::WouldBlock));
                drop(g0);

                let _g1 = l1.[<$prefix_second _lock_read_async>]().await.unwrap();
            }
        }
    };
}

generate_tests!(blocking, try;     blocking, try);
generate_tests!(blocking, try;             , try);
generate_tests!(        , try;     blocking, try);
generate_tests!(        , try;             , try);

generate_tests!(blocking, try;             , timeout);
generate_tests!(        , try;             , timeout);

generate_tests!(        , timeout; blocking, try);
generate_tests!(        , timeout;         , try);

generate_tests!(        , timeout;         , timeout);

#[tokio::test]
async fn io_read() {
    const BYTES: &[u8] = b"Hello, world!";

    let dir = tempdir().unwrap();
    let path = dir.path().join("lockfile");

    {
        let file = tokio::fs::File::create(&path).await.unwrap(); // Create the file.
        let mut guard = file.try_lock_write_async().await.unwrap();

        {
            let file = tokio::fs::File::open(&path).await.unwrap(); // Create the file.
            let _ = file.try_lock_write_async().await.unwrap_err();
        }

        guard.write_all(BYTES).await.unwrap();
    }

    {
        let guards = join_all((0..5).map(|_| async {
            let file = File::open(&path).await.unwrap(); // Open it in read-only mode.
            file.try_lock_read_async().await.unwrap()
        }))
        .await;
        join_all(guards.into_iter().map(|mut guard| async move {
            let mut buffer = Vec::new();
            guard.read_to_end(&mut buffer).await.unwrap();
            assert_eq!(&buffer, BYTES);
        }))
        .await;
    }
}

#[cfg(windows)]
mod windows {
    use super::*;
    use async_fd_lock::blocking::{LockRead, LockWrite};
    use std::os::windows::fs::OpenOptionsExt;

    #[test]
    fn try_lock_error() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("lockfile");

        // On Windows, opening with an access_mode as 0 will prevent all locking operations from succeeding, simulating an I/O error.
        let l0 = std::fs::File::options()
            .create(true)
            .read(true)
            .write(true)
            .truncate(true)
            .access_mode(0)
            .open(path)
            .unwrap();

        let (l0, err1) = l0.try_lock_read().unwrap_err();
        assert!(matches!(err1.kind(), ErrorKind::PermissionDenied));

        let (_l0, err2) = l0.try_lock_write().unwrap_err();
        assert!(matches!(err2.kind(), ErrorKind::PermissionDenied));
    }
}
