//! Extended utilities for working with files and filesystems in Rust.

#![doc(html_root_url = "https://docs.rs/fs2/0.4.3")]
#![cfg_attr(test, feature(test))]

#[cfg(windows)]
extern crate winapi;

#[cfg(unix)]
mod unix;
#[cfg(unix)]
use unix as sys;

#[cfg(windows)]
mod windows;
#[cfg(windows)]
use windows as sys;

use std::fs::File;
use std::io::{Error, Result};
use std::path::Path;

/// Extension trait for `std::fs::File` which provides allocation, duplication and locking methods.
pub trait FileExt {
    fn duplicate(&self) -> Result<File>;
    fn allocated_size(&self) -> Result<u64>;
    fn allocate(&self, len: u64) -> Result<()>;
    fn lock_shared(&self) -> Result<()>;
    fn lock_exclusive(&self) -> Result<()>;
    fn try_lock_shared(&self) -> Result<()>;
    fn try_lock_exclusive(&self) -> Result<()>;
    fn unlock(&self) -> Result<()>;
}

impl FileExt for File {
    fn duplicate(&self) -> Result<File> {
        sys::duplicate(self)
    }
    fn allocated_size(&self) -> Result<u64> {
        sys::allocated_size(self)
    }
    fn allocate(&self, len: u64) -> Result<()> {
        sys::allocate(self, len)
    }
    fn lock_shared(&self) -> Result<()> {
        sys::lock_shared(self)
    }
    fn lock_exclusive(&self) -> Result<()> {
        sys::lock_exclusive(self)
    }
    fn try_lock_shared(&self) -> Result<()> {
        sys::try_lock_shared(self)
    }
    fn try_lock_exclusive(&self) -> Result<()> {
        sys::try_lock_exclusive(self)
    }
    fn unlock(&self) -> Result<()> {
        sys::unlock(self)
    }
}

pub fn lock_contended_error() -> Error {
    sys::lock_error()
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct FsStats {
    free_space: u64,
    available_space: u64,
    total_space: u64,
    allocation_granularity: u64,
}

impl FsStats {
    pub fn free_space(&self) -> u64 {
        self.free_space
    }

    pub fn available_space(&self) -> u64 {
        self.available_space
    }

    pub fn total_space(&self) -> u64 {
        self.total_space
    }

    pub fn allocation_granularity(&self) -> u64 {
        self.allocation_granularity
    }
}

pub fn statvfs<P>(path: P) -> Result<FsStats>
where
    P: AsRef<Path>,
{
    sys::statvfs(path.as_ref())
}

pub fn free_space<P>(path: P) -> Result<u64>
where
    P: AsRef<Path>,
{
    statvfs(path).map(|stat| stat.free_space)
}

pub fn available_space<P>(path: P) -> Result<u64>
where
    P: AsRef<Path>,
{
    statvfs(path).map(|stat| stat.available_space)
}

pub fn total_space<P>(path: P) -> Result<u64>
where
    P: AsRef<Path>,
{
    statvfs(path).map(|stat| stat.total_space)
}

pub fn allocation_granularity<P>(path: P) -> Result<u64>
where
    P: AsRef<Path>,
{
    statvfs(path).map(|stat| stat.allocation_granularity)
}
