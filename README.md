# async-fd-lock
[![crates.io version][1]][2] 
[![downloads][5]][6] [![docs.rs docs][7]][8]

Advisory cross-platform file locks using file descriptors, with async support
by off-loading blocking operations to newly spawned blocking tasks.
Adapted from [yoshuawuyts/fd-lock], which was adapted from [mafintosh/fd-lock].

Note that advisory lock compliance is opt-in, and can freely be ignored by other
parties. This means this crate __should never be used for security purposes__,
but solely to coordinate file access.

[yoshuawuyts/fd-lock]: https://github.com/yoshuawuyts/fd-lock
[mafintosh/fd-lock]: https://github.com/mafintosh/fd-lock

- [Documentation][8]
- [Crates.io][2]
- [Releases][releases]

## Examples
__Basic usage__
```rust
use std::path::PathBuf;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use async_fd_lock::{LockRead, LockWrite};

let dir = tempfile::tempdir().unwrap();
let path = dir.path().join("foo.txt");

// Lock it for writing.
{
    let mut write_guard = File::create_new(&path).await?.lock_write().await?;
    write_guard.write(b"bongo cat").await?;
}

// Lock it for reading.
{
    let mut read_guard_1 = File::open(&path).await?.lock_read().await?;
    let mut read_guard_2 = File::open(&path).await?.lock_read().await?;
    let byte_1 = read_guard_1.read_u8().await?;
    let byte_2 = read_guard_2.read_u8().await?;
}
```

## Installation
```sh
$ cargo add async-fd-lock
```

## Safety
This crate uses `unsafe` on Windows to interface with `windows-sys`. All
invariants have been carefully checked, and are manually enforced.

## References
- [LockFile function - WDC](https://docs.microsoft.com/en-us/windows/desktop/api/fileapi/nf-fileapi-lockfile)
- [flock(2) - Linux Man Page](https://man7.org/linux/man-pages/man2/flock.2.html)
- [`rustix::fs::flock`](https://docs.rs/rustix/*/rustix/fs/fn.flock.html)
- [`windows_sys::Win32::Storage::FileSystem::LockFile`](https://microsoft.github.io/windows-docs-rs/doc/windows/Win32/Storage/FileSystem/fn.LockFile.html)

## License
[MIT](./LICENSE-MIT) OR [Apache-2.0](./LICENSE-APACHE)

[1]: https://img.shields.io/crates/v/async-fd-lock.svg?style=flat-square
[2]: https://crates.io/crates/async-fd-lock
[3]: https://img.shields.io/travis/Limeth/async-fd-lock/master.svg?style=flat-square
[4]: https://travis-ci.org/Limeth/async-fd-lock
[5]: https://img.shields.io/crates/d/async-fd-lock.svg?style=flat-square
[6]: https://crates.io/crates/async-fd-lock
[7]: https://img.shields.io/badge/docs-latest-blue.svg?style=flat-square
[8]: https://docs.rs/async-fd-lock

[releases]: https://github.com/Limeth/async-fd-lock/releases
