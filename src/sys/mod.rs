use std::{io, ops::Deref};

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

pub trait RwLockTrait: Deref<Target = Self::AsOpenFile> {
    type AsOpenFile: AsOpenFile;
    type BorrowedOpenFile<'a>: AsOpenFile
    where
        Self: 'a;
    type OwnedOpenFile: AsOpenFile;

    fn new(inner: Self::AsOpenFile) -> Self;

    fn into_inner(self) -> Self::AsOpenFile
    where
        Self::AsOpenFile: Sized;

    fn borrow_open_file(&self) -> Self::BorrowedOpenFile<'_>;

    fn acquire_lock_from_file<const WRITE: bool, const BLOCK: bool>(
        // handle: Self::BorrowedOpenFile<'_>,
        handle: impl AsOpenFile,
    ) -> io::Result<()>;

    fn release_lock(&self) -> io::Result<()>;
}

pub trait RwLockTraitExt: RwLockTrait {
    fn acquire_lock<const WRITE: bool, const BLOCK: bool>(&self) -> io::Result<()>;
}

impl<T> RwLockTraitExt for T
where
    T: RwLockTrait,
{
    fn acquire_lock<const WRITE: bool, const BLOCK: bool>(&self) -> io::Result<()> {
        T::acquire_lock_from_file::<WRITE, BLOCK>(self.borrow_open_file())
    }
}
