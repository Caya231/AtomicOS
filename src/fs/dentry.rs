use alloc::string::String;
use super::inode::Inode;

/// A directory entry — maps a name to an inode.
#[derive(Debug, Clone)]
pub struct DirEntry {
    pub name: String,
    pub inode: Inode,
}
