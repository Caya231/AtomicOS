use alloc::string::String;
use alloc::vec::Vec;
use spin::Mutex;
use lazy_static::lazy_static;

use super::dentry::DirEntry;
use super::error::{FsError, FsResult};
use super::inode::{FileType, Inode};
use super::mount::FileSystem;

// ──────────────────────────────────────────────────────────────
//  Internal tree node — stored in an arena (Vec<RamNode>)
// ──────────────────────────────────────────────────────────────

/// Each node in the RAMFS tree.
struct RamNode {
    id: u64,
    name: String,
    file_type: FileType,
    parent: Option<u64>,       // inode id of parent (None for root)
    children: Vec<u64>,        // inode ids of children (dirs only)
    data: Vec<u8>,             // file content (files only)
}

impl RamNode {
    fn size(&self) -> usize {
        match self.file_type {
            FileType::File => self.data.len(),
            FileType::Directory => self.children.len(),
        }
    }

    fn to_inode(&self) -> Inode {
        Inode {
            id: self.id,
            file_type: self.file_type,
            size: self.size(),
        }
    }
}

// ──────────────────────────────────────────────────────────────
//  RAMFS — tree-based in-memory filesystem
// ──────────────────────────────────────────────────────────────

/// Internal state protected by a Mutex.
struct RamFsInner {
    nodes: Vec<RamNode>,  // arena: index by scanning for id
    next_id: u64,
}

impl RamFsInner {
    fn new() -> Self {
        // Create root node (id = 0)
        let root = RamNode {
            id: 0,
            name: String::from("/"),
            file_type: FileType::Directory,
            parent: None,
            children: Vec::new(),
            data: Vec::new(),
        };
        RamFsInner {
            nodes: alloc::vec![root],
            next_id: 1,
        }
    }

    fn alloc_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    /// Find a node by its inode id. Returns index in the arena.
    fn find_by_id(&self, id: u64) -> Option<usize> {
        self.nodes.iter().position(|n| n.id == id)
    }

    /// Walk an absolute path from root, returning the inode id of the target.
    /// Path must start with "/".
    fn resolve_path(&self, path: &str) -> FsResult<u64> {
        let path = path.trim_end_matches('/');
        if path.is_empty() || path == "/" {
            return Ok(0); // root
        }

        let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
        let mut current_id: u64 = 0; // start at root

        for seg in segments {
            let idx = self.find_by_id(current_id).ok_or(FsError::NotFound)?;
            let node = &self.nodes[idx];

            if node.file_type != FileType::Directory {
                return Err(FsError::NotADirectory);
            }

            // Search children for matching name
            let mut found = false;
            for &child_id in &node.children {
                if let Some(ci) = self.find_by_id(child_id) {
                    if self.nodes[ci].name == seg {
                        current_id = child_id;
                        found = true;
                        break;
                    }
                }
            }
            if !found {
                return Err(FsError::NotFound);
            }
        }

        Ok(current_id)
    }

    /// Resolve parent path and return (parent_inode_id, child_name).
    fn resolve_parent(&self, path: &str) -> FsResult<(u64, String)> {
        let path = path.trim_end_matches('/');
        if path == "/" || path.is_empty() {
            return Err(FsError::InvalidPath);
        }

        let last_slash = path.rfind('/').ok_or(FsError::InvalidPath)?;
        let parent_path = if last_slash == 0 { "/" } else { &path[..last_slash] };
        let child_name = &path[last_slash + 1..];

        if child_name.is_empty() {
            return Err(FsError::InvalidPath);
        }

        let parent_id = self.resolve_path(parent_path)?;

        // Verify parent is a directory
        let pidx = self.find_by_id(parent_id).ok_or(FsError::NotFound)?;
        if self.nodes[pidx].file_type != FileType::Directory {
            return Err(FsError::NotADirectory);
        }

        // Check for duplicate name
        for &cid in &self.nodes[pidx].children {
            if let Some(ci) = self.find_by_id(cid) {
                if self.nodes[ci].name == child_name {
                    return Err(FsError::AlreadyExists);
                }
            }
        }

        Ok((parent_id, String::from(child_name)))
    }

    /// Insert a new node as a child of parent_id.
    fn insert_node(&mut self, parent_id: u64, name: String, ft: FileType) -> FsResult<Inode> {
        let id = self.alloc_id();
        let node = RamNode {
            id,
            name,
            file_type: ft,
            parent: Some(parent_id),
            children: Vec::new(),
            data: Vec::new(),
        };
        let inode = node.to_inode();
        self.nodes.push(node);

        // Add to parent's children list
        let pidx = self.find_by_id(parent_id).ok_or(FsError::NotFound)?;
        self.nodes[pidx].children.push(id);

        Ok(inode)
    }
}

// ──────────────────────────────────────────────────────────────
//  Public RamFs struct
// ──────────────────────────────────────────────────────────────

pub struct RamFs {
    label: &'static str,
    inner: Mutex<RamFsInner>,
}

impl RamFs {
    pub fn new(label: &'static str) -> Self {
        RamFs {
            label,
            inner: Mutex::new(RamFsInner::new()),
        }
    }

    /// Normalize path: ensure starts with / and no trailing /
    fn normalize(path: &str) -> String {
        let p = if path.starts_with('/') {
            String::from(path)
        } else {
            alloc::format!("/{}", path)
        };
        if p.len() > 1 && p.ends_with('/') {
            String::from(p.trim_end_matches('/'))
        } else {
            p
        }
    }
}

// ──────────────────────────────────────────────────────────────
//  FileSystem trait implementation
// ──────────────────────────────────────────────────────────────

impl FileSystem for RamFs {
    fn name(&self) -> &str {
        self.label
    }

    fn create(&self, path: &str) -> FsResult<Inode> {
        let path = Self::normalize(path);
        let mut inner = self.inner.lock();
        let (parent_id, name) = inner.resolve_parent(&path)?;
        inner.insert_node(parent_id, name, FileType::File)
    }

    fn mkdir(&self, path: &str) -> FsResult<Inode> {
        let path = Self::normalize(path);
        let mut inner = self.inner.lock();
        let (parent_id, name) = inner.resolve_parent(&path)?;
        inner.insert_node(parent_id, name, FileType::Directory)
    }

    fn lookup(&self, path: &str) -> FsResult<Inode> {
        let path = Self::normalize(path);
        let inner = self.inner.lock();
        let id = inner.resolve_path(&path)?;
        let idx = inner.find_by_id(id).ok_or(FsError::NotFound)?;
        Ok(inner.nodes[idx].to_inode())
    }

    fn read(&self, path: &str, offset: usize, buf: &mut [u8]) -> FsResult<usize> {
        let path = Self::normalize(path);
        let inner = self.inner.lock();
        let id = inner.resolve_path(&path)?;
        let idx = inner.find_by_id(id).ok_or(FsError::NotFound)?;
        let node = &inner.nodes[idx];

        if node.file_type == FileType::Directory {
            return Err(FsError::IsADirectory);
        }

        // EOF check
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
        let mut inner = self.inner.lock();
        let id = inner.resolve_path(&path)?;
        let idx = inner.find_by_id(id).ok_or(FsError::NotFound)?;
        let node = &mut inner.nodes[idx];

        if node.file_type == FileType::Directory {
            return Err(FsError::IsADirectory);
        }

        // Expand with zeros if offset > current len (gap fill)
        let end = offset + data.len();
        if end > node.data.len() {
            node.data.resize(end, 0);
        }
        node.data[offset..end].copy_from_slice(data);
        Ok(data.len())
    }

    fn readdir(&self, path: &str) -> FsResult<Vec<DirEntry>> {
        let path = Self::normalize(path);
        let inner = self.inner.lock();
        let id = inner.resolve_path(&path)?;
        let idx = inner.find_by_id(id).ok_or(FsError::NotFound)?;
        let node = &inner.nodes[idx];

        if node.file_type != FileType::Directory {
            return Err(FsError::NotADirectory);
        }

        let mut entries = Vec::new();
        for &child_id in &node.children {
            if let Some(ci) = inner.find_by_id(child_id) {
                let child = &inner.nodes[ci];
                entries.push(DirEntry {
                    name: child.name.clone(),
                    inode: child.to_inode(),
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

        let mut inner = self.inner.lock();
        let id = inner.resolve_path(&path)?;
        let idx = inner.find_by_id(id).ok_or(FsError::NotFound)?;

        // Cannot remove non-empty directory
        if inner.nodes[idx].file_type == FileType::Directory
            && !inner.nodes[idx].children.is_empty()
        {
            return Err(FsError::IsADirectory);
        }

        let parent_id = inner.nodes[idx].parent.ok_or(FsError::InvalidPath)?;

        // Remove from parent's children
        let pidx = inner.find_by_id(parent_id).ok_or(FsError::NotFound)?;
        inner.nodes[pidx].children.retain(|&c| c != id);

        // Remove node from arena
        inner.nodes.remove(idx);

        Ok(())
    }
}

// ──────────────────────────────────────────────────────────────
//  Global instances
// ──────────────────────────────────────────────────────────────

lazy_static! {
    pub static ref RAMFS_INSTANCE: RamFs = RamFs::new("ramfs");
    pub static ref TMPFS_INSTANCE: RamFs = RamFs::new("tmpfs");
}
