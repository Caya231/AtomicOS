use alloc::sync::Arc;
use spin::Mutex;
use alloc::format;
use crate::fs::pipe::PipeInner;

pub enum FileType {
    Regular,
    Directory,
    PipeRead(Arc<Mutex<PipeInner>>),
    PipeWrite(Arc<Mutex<PipeInner>>),
    Console,
}

pub struct File {
    pub file_type: FileType,
    pub path: alloc::string::String, // Only used for Regular/Directory
    pub offset: u64,
    pub readable: bool,
    pub writable: bool,
}

impl File {
    pub fn new_console() -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(File {
            file_type: FileType::Console,
            path: alloc::string::String::from("console"),
            offset: 0,
            readable: true,
            writable: true,
        }))
    }

    pub fn new_regular(path: &str, readable: bool, writable: bool) -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(File {
            file_type: FileType::Regular,
            path: alloc::string::String::from(path),
            offset: 0,
            readable,
            writable,
        }))
    }
}

impl Drop for File {
    fn drop(&mut self) {
        // When a File is dropped (all references are gone), check if it's a Pipe.
        // We must decrement the inner pipe's writer/reader counts to notify the
        // other side of the Pipe that this endpoint is closed!
        match &self.file_type {
            FileType::PipeRead(inner) => {
                inner.lock().drop_reader();
            },
            FileType::PipeWrite(inner) => {
                inner.lock().drop_writer();
            },
            _ => {}
        }
    }
}
