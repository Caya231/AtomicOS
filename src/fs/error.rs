use alloc::string::String;
use core::fmt;

/// Filesystem error types.
#[derive(Debug, Clone)]
pub enum FsError {
    NotFound,
    AlreadyExists,
    NotADirectory,
    IsADirectory,
    InvalidPath,
    IoError,
    NoSpace,
    NotMounted,
}

impl fmt::Display for FsError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            FsError::NotFound => write!(f, "No such file or directory"),
            FsError::AlreadyExists => write!(f, "File exists"),
            FsError::NotADirectory => write!(f, "Not a directory"),
            FsError::IsADirectory => write!(f, "Is a directory"),
            FsError::InvalidPath => write!(f, "Invalid path"),
            FsError::IoError => write!(f, "I/O error"),
            FsError::NoSpace => write!(f, "No space left"),
            FsError::NotMounted => write!(f, "No filesystem mounted at path"),
        }
    }
}

pub type FsResult<T> = Result<T, FsError>;
