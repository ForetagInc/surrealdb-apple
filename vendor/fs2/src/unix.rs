extern crate libc;

use std::ffi::CString;
use std::fs::File;
use std::io::{Error, ErrorKind, Result};
use std::mem;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::MetadataExt;
use std::os::unix::io::{AsRawFd, FromRawFd};
use std::path::Path;

use FsStats;

pub fn duplicate(file: &File) -> Result<File> {
    unsafe {
        let fd = libc::dup(file.as_raw_fd());

        if fd < 0 {
            Err(Error::last_os_error())
        } else {
            Ok(File::from_raw_fd(fd))
        }
    }
}

pub fn lock_shared(file: &File) -> Result<()> {
    flock(file, libc::LOCK_SH)
}

pub fn lock_exclusive(file: &File) -> Result<()> {
    flock(file, libc::LOCK_EX)
}

pub fn try_lock_shared(file: &File) -> Result<()> {
    flock(file, libc::LOCK_SH | libc::LOCK_NB)
}

pub fn try_lock_exclusive(file: &File) -> Result<()> {
    flock(file, libc::LOCK_EX | libc::LOCK_NB)
}

pub fn unlock(file: &File) -> Result<()> {
    flock(file, libc::LOCK_UN)
}

pub fn lock_error() -> Error {
    Error::from_raw_os_error(libc::EWOULDBLOCK)
}

#[cfg(not(target_os = "solaris"))]
fn flock(file: &File, flag: libc::c_int) -> Result<()> {
    let ret = unsafe { libc::flock(file.as_raw_fd(), flag) };
    if ret < 0 {
        Err(Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(target_os = "solaris")]
fn flock(file: &File, flag: libc::c_int) -> Result<()> {
    let mut fl = libc::flock {
        l_whence: 0,
        l_start: 0,
        l_len: 0,
        l_type: 0,
        l_pad: [0; 4],
        l_pid: 0,
        l_sysid: 0,
    };

    let (cmd, operation) = match flag & libc::LOCK_NB {
        0 => (libc::F_SETLKW, flag),
        _ => (libc::F_SETLK, flag & !libc::LOCK_NB),
    };

    match operation {
        libc::LOCK_SH => fl.l_type |= libc::F_RDLCK,
        libc::LOCK_EX => fl.l_type |= libc::F_WRLCK,
        libc::LOCK_UN => fl.l_type |= libc::F_UNLCK,
        _ => return Err(Error::from_raw_os_error(libc::EINVAL)),
    }

    let ret = unsafe { libc::fcntl(file.as_raw_fd(), cmd, &fl) };
    match ret {
        -1 => match Error::last_os_error().raw_os_error() {
            Some(libc::EACCES) => Err(lock_error()),
            _ => Err(Error::last_os_error()),
        },
        _ => Ok(()),
    }
}

pub fn allocated_size(file: &File) -> Result<u64> {
    file.metadata().map(|m| m.blocks() as u64 * 512)
}

#[cfg(any(
    target_os = "linux",
    target_os = "freebsd",
    target_os = "android"
))]
pub fn allocate(file: &File, len: u64) -> Result<()> {
    let ret = unsafe { libc::posix_fallocate(file.as_raw_fd(), 0, len as libc::off_t) };
    if ret == 0 {
        Ok(())
    } else {
        Err(Error::last_os_error())
    }
}

#[cfg(any(
    target_os = "macos",
    target_os = "ios",
    target_os = "tvos",
    target_os = "watchos",
    target_os = "visionos"
))]
pub fn allocate(file: &File, len: u64) -> Result<()> {
    let stat = file.metadata()?;

    if len > stat.blocks() as u64 * 512 {
        let mut fstore = libc::fstore_t {
            fst_flags: libc::F_ALLOCATECONTIG,
            fst_posmode: libc::F_PEOFPOSMODE,
            fst_offset: 0,
            fst_length: len as libc::off_t,
            fst_bytesalloc: 0,
        };

        let ret = unsafe { libc::fcntl(file.as_raw_fd(), libc::F_PREALLOCATE, &fstore) };
        if ret == -1 {
            fstore.fst_flags = libc::F_ALLOCATEALL;
            let ret = unsafe { libc::fcntl(file.as_raw_fd(), libc::F_PREALLOCATE, &fstore) };
            if ret == -1 {
                return Err(Error::last_os_error());
            }
        }
    }

    if len > stat.size() as u64 {
        file.set_len(len)
    } else {
        Ok(())
    }
}

#[cfg(any(
    target_os = "openbsd",
    target_os = "netbsd",
    target_os = "dragonfly",
    target_os = "solaris",
    target_os = "haiku"
))]
pub fn allocate(file: &File, len: u64) -> Result<()> {
    if len > file.metadata()?.len() as u64 {
        file.set_len(len)
    } else {
        Ok(())
    }
}

pub fn statvfs(path: &Path) -> Result<FsStats> {
    let cstr = match CString::new(path.as_os_str().as_bytes()) {
        Ok(cstr) => cstr,
        Err(..) => return Err(Error::new(ErrorKind::InvalidInput, "path contained a null")),
    };

    unsafe {
        let mut stat: libc::statvfs = mem::zeroed();
        if libc::statvfs(cstr.as_ptr() as *const _, &mut stat) != 0 {
            Err(Error::last_os_error())
        } else {
            Ok(FsStats {
                free_space: stat.f_frsize as u64 * stat.f_bfree as u64,
                available_space: stat.f_frsize as u64 * stat.f_bavail as u64,
                total_space: stat.f_frsize as u64 * stat.f_blocks as u64,
                allocation_granularity: stat.f_frsize as u64,
            })
        }
    }
}
