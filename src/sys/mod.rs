use std::io;

use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(unix)] {
        mod unix;
        pub use unix::*;
        pub use rustix::fd::AsFd as AsOpenFile;
    } else if #[cfg(windows)] {
        mod windows;
        pub use windows::*;
        #[doc(no_inline)]
        pub use std::os::windows::io::AsHandle as AsOpenFile;
    }
}

pub trait RwLockTrait<T: AsOpenFile> {
    fn new(inner: T) -> Self;

    fn into_inner(self) -> T
    where
        T: Sized;

    fn acquire_lock<const WRITE: bool, const BLOCK: bool>(&self) -> io::Result<()>;

    fn release_lock(&self) -> io::Result<()>;
}
