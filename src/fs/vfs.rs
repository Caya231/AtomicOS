use alloc::string::String;
use alloc::vec::Vec;
use super::dentry::DirEntry;
use super::error::{FsError, FsResult};
use super::inode::Inode;
use super::mount::FileSystem;

/// A mount point associates a path prefix with a concrete filesystem.
struct MountPoint {
    path: String,
    fs: &'static dyn FileSystem,
}

/// The Virtual File System — resolves paths to mount points and delegates.
pub struct Vfs {
    mounts: Vec<MountPoint>,
}

impl Vfs {
    pub fn new() -> Self {
        Vfs { mounts: Vec::new() }
    }

    /// Mount a filesystem at the given path.
    pub fn mount(&mut self, path: &str, fs: &'static dyn FileSystem) {
        self.mounts.push(MountPoint {
            path: String::from(path),
            fs,
        });
        // Sort by path length descending so longer prefixes match first
        self.mounts.sort_by(|a, b| b.path.len().cmp(&a.path.len()));
    }

    /// Resolve which mount point handles a given absolute path.
    /// Returns (filesystem, path relative to mount point).
    fn resolve(&self, abs_path: &str) -> FsResult<(&dyn FileSystem, String)> {
        for mp in &self.mounts {
            if abs_path == mp.path || abs_path.starts_with(&alloc::format!("{}/", mp.path.trim_end_matches('/'))) || mp.path == "/" {
                let relative = if mp.path == "/" {
                    String::from(abs_path)
                } else {
                    let stripped = &abs_path[mp.path.len()..];
                    if stripped.is_empty() {
                        String::from("/")
                    } else {
                        String::from(stripped)
                    }
                };
                return Ok((mp.fs, relative));
            }
        }
        Err(FsError::NotMounted)
    }

    // ---- VFS public API (delegates to resolved filesystem) ----

    pub fn create(&mut self, path: &str) -> FsResult<Inode> {
        let (fs, rel) = self.resolve(path)?;
        fs.create(&rel)
    }

    pub fn mkdir(&mut self, path: &str) -> FsResult<Inode> {
        let (fs, rel) = self.resolve(path)?;
        fs.mkdir(&rel)
    }

    pub fn lookup(&self, path: &str) -> FsResult<Inode> {
        let (fs, rel) = self.resolve(path)?;
        fs.lookup(&rel)
    }

    pub fn read_file(&self, path: &str, offset: usize, buf: &mut [u8]) -> FsResult<usize> {
        let (fs, rel) = self.resolve(path)?;
        fs.read(&rel, offset, buf)
    }

    pub fn write_file(&mut self, path: &str, data: &[u8]) -> FsResult<usize> {
        let (fs, rel) = self.resolve(path)?;
        fs.write(&rel, 0, data)
    }

    pub fn readdir(&self, path: &str) -> FsResult<Vec<DirEntry>> {
        let (fs, rel) = self.resolve(path)?;
        fs.readdir(&rel)
    }

    pub fn unlink(&mut self, path: &str) -> FsResult<()> {
        let (fs, rel) = self.resolve(path)?;
        fs.unlink(&rel)
    }

    /// Check if path exists.
    pub fn exists(&self, path: &str) -> bool {
        self.lookup(path).is_ok()
    }

    /// Check if path is a directory.
    pub fn is_dir(&self, path: &str) -> bool {
        self.lookup(path)
            .map(|inode| inode.file_type == super::inode::FileType::Directory)
            .unwrap_or(false)
    }
}
