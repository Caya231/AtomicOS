use alloc::string::String;
use alloc::vec::Vec;
use super::dentry::DirEntry;
use super::error::FsResult;
use super::inode::Inode;

/// The FileSystem trait — every concrete filesystem must implement this.
/// All paths passed to these methods are relative to the mount point.
pub trait FileSystem: Send + Sync {
    /// Name of this filesystem (e.g. "ramfs", "fat32").
    fn name(&self) -> &str;

    /// Create a new regular file at `path`.
    fn create(&self, path: &str) -> FsResult<Inode>;

    /// Create a new directory at `path`.
    fn mkdir(&self, path: &str) -> FsResult<Inode>;

    /// Look up an inode by path.
    fn lookup(&self, path: &str) -> FsResult<Inode>;

    /// Read up to `buf.len()` bytes from file at `path`, starting at `offset`.
    /// Returns number of bytes read.
    fn read(&self, path: &str, offset: usize, buf: &mut [u8]) -> FsResult<usize>;

    /// Write `data` to file at `path`, starting at `offset`.
    /// Returns number of bytes written.
    fn write(&self, path: &str, offset: usize, data: &[u8]) -> FsResult<usize>;

    /// List entries in directory at `path`.
    fn readdir(&self, path: &str) -> FsResult<Vec<DirEntry>>;

    /// Remove a file or empty directory at `path`.
    fn unlink(&self, path: &str) -> FsResult<()>;
}
