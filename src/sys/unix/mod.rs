mod rw_lock;

pub use rw_lock::RwLock;

use rustix::{fd::AsFd, fs};

pub(crate) fn compatible_unix_lock<Fd: AsFd>(
    fd: Fd,
    operation: fs::FlockOperation,
) -> rustix::io::Result<()> {
    #[cfg(not(target_os = "solaris"))]
    return fs::flock(fd, operation);

    #[cfg(target_os = "solaris")]
    return fs::fcntl_lock(fd, operation);
}
