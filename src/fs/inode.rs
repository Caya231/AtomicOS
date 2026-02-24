/// Inode represents a filesystem node (file or directory).
#[derive(Debug, Clone)]
pub struct Inode {
    pub id: u64,
    pub file_type: FileType,
    pub size: usize,
}

/// Type of filesystem node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileType {
    File,
    Directory,
}
