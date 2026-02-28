use alloc::sync::Arc;
use alloc::vec;
use alloc::vec::Vec;
use spin::Mutex;
use crate::scheduler::ProcessId;

const PIPE_BUFFER_SIZE: usize = 4096;

/// Shared internal state of a Pipe representing the data conduit between processes.
pub struct PipeInner {
    buffer: Vec<u8>,
    read_pos: usize,
    write_pos: usize,
    readers: usize,
    writers: usize,
}

impl PipeInner {
    pub fn new() -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(PipeInner {
            buffer: vec![0; PIPE_BUFFER_SIZE],
            read_pos: 0,
            write_pos: 0,
            readers: 0,
            writers: 0,
        }))
    }

    pub fn add_reader(&mut self) {
        self.readers += 1;
    }

    pub fn add_writer(&mut self) {
        self.writers += 1;
    }

    pub fn drop_reader(&mut self) {
        if self.readers > 0 {
            self.readers -= 1;
        }
    }

    pub fn drop_writer(&mut self) {
        if self.writers > 0 {
            self.writers -= 1;
        }
    }

    pub fn is_empty(&self) -> bool {
        self.read_pos == self.write_pos
    }

    pub fn is_full(&self) -> bool {
        (self.write_pos + 1) % PIPE_BUFFER_SIZE == self.read_pos
    }

    pub fn active_writers(&self) -> usize {
        self.writers
    }

    pub fn active_readers(&self) -> usize {
        self.readers
    }

    /// Read up to `buf.len()` bytes. Returns the number of bytes read.
    pub fn read(&mut self, buf: &mut [u8]) -> usize {
        let mut bytes_read = 0;
        while bytes_read < buf.len() && !self.is_empty() {
            buf[bytes_read] = self.buffer[self.read_pos];
            self.read_pos = (self.read_pos + 1) % PIPE_BUFFER_SIZE;
            bytes_read += 1;
        }
        bytes_read
    }

    /// Write up to `buf.len()` bytes. Returns the number of bytes written.
    pub fn write(&mut self, buf: &[u8]) -> usize {
        let mut bytes_written = 0;
        while bytes_written < buf.len() && !self.is_full() {
            self.buffer[self.write_pos] = buf[bytes_written];
            self.write_pos = (self.write_pos + 1) % PIPE_BUFFER_SIZE;
            bytes_written += 1;
        }
        bytes_written
    }
}
