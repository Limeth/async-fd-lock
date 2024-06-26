use std::io;

use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(unix)] {
        mod unix;

        pub use rustix::fd::AsFd as AsOpenFile;
    } else if #[cfg(windows)] {
        mod windows;

        #[doc(no_inline)]
        pub use std::os::windows::io::AsHandle as AsOpenFile;
    }
}

pub(crate) trait AsOpenFileExt: AsOpenFile {
    type BorrowedOpenFile<'a>: AsOpenFile
    where
        Self: 'a;
    type OwnedOpenFile: AsOpenFile;

    cfg_if! {
        if #[cfg(feature = "async")] {
            fn borrow_open_file(&self) -> Self::BorrowedOpenFile<'_>;
        } else {
            #[allow(unused)]
            fn borrow_open_file(&self) -> Self::BorrowedOpenFile<'_>;
        }
    }

    fn acquire_lock_blocking<const WRITE: bool, const BLOCK: bool>(
        &self,
    ) -> io::Result<RwLockGuard<Self::OwnedOpenFile>>
    where
        Self: Sized;
    fn release_lock_blocking(&self) -> io::Result<()>;
}

#[must_use = "if unused the RwLock will immediately unlock"]
pub struct RwLockGuard<T: AsOpenFile> {
    handle: Option<<T as AsOpenFileExt>::OwnedOpenFile>,
}

impl<T: AsOpenFile> RwLockGuard<T> {
    pub fn new(handle: <T as AsOpenFileExt>::OwnedOpenFile) -> Self {
        Self {
            handle: Some(handle),
        }
    }

    pub fn defuse(mut self) -> <T as AsOpenFileExt>::OwnedOpenFile {
        self.handle.take().expect("handle should always be present")
    }

    pub fn defuse_with<R>(self, map: impl FnOnce(<T as AsOpenFileExt>::OwnedOpenFile) -> R) -> R {
        (map)(self.defuse())
    }
}

impl<T: AsOpenFile> Drop for RwLockGuard<T> {
    fn drop(&mut self) {
        if let Some(handle) = self.handle.take() {
            let _ = handle.release_lock_blocking();
        }
    }
}
