use alloc::string::String;
use alloc::format;
use alloc::vec::Vec;
use spin::Mutex;
use lazy_static::lazy_static;

/// Kernel log ring buffer.
pub struct KernelLog {
    pub entries: Vec<String>,
}

impl KernelLog {
    pub fn new() -> Self {
        KernelLog { entries: Vec::new() }
    }

    pub fn push(&mut self, msg: String) {
        if self.entries.len() >= 64 {
            self.entries.remove(0);
        }
        self.entries.push(msg);
    }
}

lazy_static! {
    pub static ref KLOG: Mutex<KernelLog> = Mutex::new(KernelLog::new());
    pub static ref CWD: Mutex<String> = Mutex::new(String::from("/"));
}

/// Resolve a path relative to the current working directory.
/// Handles absolute paths, relative paths, `.` and `..`.
pub fn resolve_path(input: &str) -> String {
    let cwd = CWD.lock().clone();
    let raw = if input.starts_with('/') {
        String::from(input)
    } else {
        if cwd == "/" {
            format!("/{}", input)
        } else {
            format!("{}/{}", cwd, input)
        }
    };

    // Normalize: split by '/', handle . and ..
    let mut parts: Vec<&str> = Vec::new();
    for segment in raw.split('/') {
        match segment {
            "" | "." => {},
            ".." => { parts.pop(); },
            s => parts.push(s),
        }
    }

    if parts.is_empty() {
        String::from("/")
    } else {
        let mut result = String::new();
        for p in parts {
            result.push('/');
            result.push_str(p);
        }
        result
    }
}

/// Helper: log a command execution to the kernel log buffer.
pub fn log_cmd(msg: &str) {
    let ticks = crate::shell::commands::uptime::TICKS.load(core::sync::atomic::Ordering::Relaxed);
    KLOG.lock().push(format!("[{}] {}", ticks, msg));
}
