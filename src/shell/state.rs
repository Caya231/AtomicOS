use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::format;
use alloc::vec::Vec;
use spin::Mutex;
use lazy_static::lazy_static;

/// In-memory simulated filesystem.
/// Maps full path (e.g. "/home/readme.txt") to file contents.
/// Directories are entries with None content.
pub struct MemFs {
    pub files: BTreeMap<String, Option<String>>,
}

impl MemFs {
    pub fn new() -> Self {
        let mut fs = MemFs { files: BTreeMap::new() };
        // Seed with some default entries
        fs.files.insert(String::from("/"), None); // root dir
        fs.files.insert(String::from("/kernel.bin"), Some(String::from("[binary]")));
        fs.files.insert(String::from("/README.md"), Some(String::from("# AtomicOS\nA hobby x86_64 kernel written in Rust.")));
        fs.files.insert(String::from("/BUILD.md"), Some(String::from("# Build Dependencies\nnasm, ld, gcc, grub-mkrescue, qemu, rust nightly")));
        fs.files.insert(String::from("/grub.cfg"), Some(String::from("multiboot2 /boot/kernel.bin")));
        fs
    }

    pub fn is_dir(&self, path: &str) -> bool {
        self.files.get(path).map_or(false, |v| v.is_none())
    }

    pub fn exists(&self, path: &str) -> bool {
        self.files.contains_key(path)
    }

    pub fn list_dir(&self, dir: &str) -> Vec<&String> {
        let prefix = if dir == "/" { String::from("/") } else { format!("{}/", dir) };
        self.files.keys()
            .filter(|k| {
                if *k == dir { return false; }
                k.starts_with(&prefix) && k[prefix.len()..].find('/').is_none()
            })
            .collect()
    }
}

/// Simulated process table.
pub struct Process {
    pub pid: u32,
    pub name: String,
    pub state: String,
}

pub struct ProcessTable {
    pub procs: Vec<Process>,
    pub next_pid: u32,
}

impl ProcessTable {
    pub fn new() -> Self {
        let mut pt = ProcessTable { procs: Vec::new(), next_pid: 3 };
        // Seed with kernel threads
        pt.procs.push(Process { pid: 0, name: String::from("kernel"), state: String::from("running") });
        pt.procs.push(Process { pid: 1, name: String::from("idle"), state: String::from("sleeping") });
        pt.procs.push(Process { pid: 2, name: String::from("shell"), state: String::from("running") });
        pt
    }
}

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
    pub static ref MEMFS: Mutex<MemFs> = Mutex::new(MemFs::new());
    pub static ref PROCS: Mutex<ProcessTable> = Mutex::new(ProcessTable::new());
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
