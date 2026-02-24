use super::inode::Inode;

/// An open file handle with a read/write offset.
#[derive(Debug, Clone)]
pub struct FileHandle {
    pub inode: Inode,
    pub offset: usize,
}

impl FileHandle {
    pub fn new(inode: Inode) -> Self {
        FileHandle { inode, offset: 0 }
    }
}
