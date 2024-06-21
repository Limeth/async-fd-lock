use std::io;
use std::ops::Deref;

use crate::sys;

pub mod blocking;

#[cfg(feature = "async")]
pub mod nonblocking;

pub use nonblocking::*;

pub trait RwLockTrait: Deref<Target = Self::AsOpenFile> {
    type AsOpenFile: sys::AsOpenFile;

    fn into_inner(self) -> Self::AsOpenFile
    where
        Self::AsOpenFile: Sized;

    // TODO: Make sure this is not in the public API
    fn release_lock(&self) -> io::Result<()>;
}
