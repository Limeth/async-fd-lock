use std::io;

use thiserror::Error;

use crate::{RwLockReadGuard, RwLockWriteGuard};

#[derive(Debug, Error)]
pub struct LockError<T> {
    pub file: T,
    #[source]
    pub error: io::Error,
}

impl<T> LockError<T> {
    pub fn new(file: T, error: io::Error) -> Self {
        Self { file, error }
    }
}

impl<T> From<LockError<T>> for io::Error {
    fn from(value: LockError<T>) -> Self {
        value.error
    }
}

impl<T> From<LockError<T>> for (T, io::Error) {
    fn from(value: LockError<T>) -> Self {
        (value.file, value.error)
    }
}

pub type LockReadResult<T> = Result<RwLockReadGuard<T>, LockError<T>>;
pub type LockWriteResult<T> = Result<RwLockWriteGuard<T>, LockError<T>>;
