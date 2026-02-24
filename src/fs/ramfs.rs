use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use spin::Mutex;
use lazy_static::lazy_static;

use super::dentry::DirEntry;
use super::error::{FsError, FsResult};
use super::inode::{FileType, Inode};
use super::mount::FileSystem;

/// An in-memory node (file or directory).
struct RamNode {
    inode: Inode,
    data: Vec<u8>,
    children: Vec<String>,
}

/// RAMFS — a fully in-memory filesystem.
pub struct RamFs {
    label: &'static str,
    nodes: Mutex<BTreeMap<String, RamNode>>,
    next_id: Mutex<u64>,
}

impl RamFs {
    pub fn new(label: &'static str) -> Self {
        let mut nodes = BTreeMap::new();
        nodes.insert(String::from("/"), RamNode {
            inode: Inode { id: 0, file_type: FileType::Directory, size: 0 },
            data: Vec::new(),
            children: Vec::new(),
        });

        RamFs {
            label,
            nodes: Mutex::new(nodes),
            next_id: Mutex::new(1),
        }
    }

    fn alloc_id(&self) -> u64 {
        let mut id = self.next_id.lock();
        let val = *id;
        *id += 1;
        val
    }

    fn parent_and_name(path: &str) -> (&str, &str) {
        let path = path.trim_end_matches('/');
        if path == "/" || path.is_empty() {
            return ("/", "");
        }
        match path.rfind('/') {
            Some(0) => ("/", &path[1..]),
            Some(i) => (&path[..i], &path[i + 1..]),
            None => ("/", path),
        }
    }

    fn normalize(path: &str) -> String {
        let p = if path.starts_with('/') { String::from(path) } else { alloc::format!("/{}", path) };
        if p.len() > 1 && p.ends_with('/') {
            String::from(p.trim_end_matches('/'))
        } else {
            p
        }
    }
}

impl FileSystem for RamFs {
    fn name(&self) -> &str {
        self.label
    }

    fn create(&self, path: &str) -> FsResult<Inode> {
        let path = Self::normalize(path);
        let mut nodes = self.nodes.lock();

        if nodes.contains_key(&path) {
            return Err(FsError::AlreadyExists);
        }

        let (parent, name) = Self::parent_and_name(&path);
        let parent_str = String::from(parent);

        let parent_node = nodes.get_mut(&parent_str).ok_or(FsError::NotFound)?;
        if parent_node.inode.file_type != FileType::Directory {
            return Err(FsError::NotADirectory);
        }
        parent_node.children.push(String::from(name));

        let id = self.alloc_id();
        let inode = Inode { id, file_type: FileType::File, size: 0 };
        nodes.insert(path, RamNode {
            inode: inode.clone(),
            data: Vec::new(),
            children: Vec::new(),
        });

        Ok(inode)
    }

    fn mkdir(&self, path: &str) -> FsResult<Inode> {
        let path = Self::normalize(path);
        let mut nodes = self.nodes.lock();

        if nodes.contains_key(&path) {
            return Err(FsError::AlreadyExists);
        }

        let (parent, name) = Self::parent_and_name(&path);
        let parent_str = String::from(parent);

        let parent_node = nodes.get_mut(&parent_str).ok_or(FsError::NotFound)?;
        if parent_node.inode.file_type != FileType::Directory {
            return Err(FsError::NotADirectory);
        }
        parent_node.children.push(String::from(name));

        let id = self.alloc_id();
        let inode = Inode { id, file_type: FileType::Directory, size: 0 };
        nodes.insert(path, RamNode {
            inode: inode.clone(),
            data: Vec::new(),
            children: Vec::new(),
        });

        Ok(inode)
    }

    fn lookup(&self, path: &str) -> FsResult<Inode> {
        let path = Self::normalize(path);
        let nodes = self.nodes.lock();
        nodes.get(&path)
            .map(|n| n.inode.clone())
            .ok_or(FsError::NotFound)
    }

    fn read(&self, path: &str, offset: usize, buf: &mut [u8]) -> FsResult<usize> {
        let path = Self::normalize(path);
        let nodes = self.nodes.lock();
        let node = nodes.get(&path).ok_or(FsError::NotFound)?;

        if node.inode.file_type == FileType::Directory {
            return Err(FsError::IsADirectory);
        }

        if offset >= node.data.len() {
            return Ok(0);
        }

        let available = &node.data[offset..];
        let to_read = buf.len().min(available.len());
        buf[..to_read].copy_from_slice(&available[..to_read]);
        Ok(to_read)
    }

    fn write(&self, path: &str, offset: usize, data: &[u8]) -> FsResult<usize> {
        let path = Self::normalize(path);
        let mut nodes = self.nodes.lock();
        let node = nodes.get_mut(&path).ok_or(FsError::NotFound)?;

        if node.inode.file_type == FileType::Directory {
            return Err(FsError::IsADirectory);
        }

        let end = offset + data.len();
        if end > node.data.len() {
            node.data.resize(end, 0);
        }
        node.data[offset..end].copy_from_slice(data);
        node.inode.size = node.data.len();

        Ok(data.len())
    }

    fn readdir(&self, path: &str) -> FsResult<Vec<DirEntry>> {
        let path = Self::normalize(path);
        let nodes = self.nodes.lock();
        let node = nodes.get(&path).ok_or(FsError::NotFound)?;

        if node.inode.file_type != FileType::Directory {
            return Err(FsError::NotADirectory);
        }

        let mut entries = Vec::new();
        for child_name in &node.children {
            let child_path = if path == "/" {
                alloc::format!("/{}", child_name)
            } else {
                alloc::format!("{}/{}", path, child_name)
            };
            if let Some(child_node) = nodes.get(&child_path) {
                entries.push(DirEntry {
                    name: child_name.clone(),
                    inode: child_node.inode.clone(),
                });
            }
        }

        Ok(entries)
    }

    fn unlink(&self, path: &str) -> FsResult<()> {
        let path = Self::normalize(path);
        if path == "/" {
            return Err(FsError::InvalidPath);
        }

        let mut nodes = self.nodes.lock();

        if let Some(node) = nodes.get(&path) {
            if node.inode.file_type == FileType::Directory && !node.children.is_empty() {
                return Err(FsError::IsADirectory);
            }
        } else {
            return Err(FsError::NotFound);
        }

        let (parent, name) = Self::parent_and_name(&path);
        let parent_str = String::from(parent);
        if let Some(parent_node) = nodes.get_mut(&parent_str) {
            parent_node.children.retain(|c| c != name);
        }

        nodes.remove(&path);
        Ok(())
    }
}

lazy_static! {
    pub static ref RAMFS_INSTANCE: RamFs = RamFs::new("ramfs");
    pub static ref TMPFS_INSTANCE: RamFs = RamFs::new("tmpfs");
}
