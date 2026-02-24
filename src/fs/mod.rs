pub mod vfs;
pub mod inode;
pub mod file;
pub mod dentry;
pub mod mount;
pub mod error;
pub mod ramfs;

use spin::Mutex;
use lazy_static::lazy_static;
use vfs::Vfs;

lazy_static! {
    pub static ref VFS: Mutex<Vfs> = Mutex::new(Vfs::new());
}

/// Initialize the VFS and mount a RAMFS at root.
pub fn init() {
    let mut vfs = VFS.lock();
    // Mount the primary RAMFS at "/"
    let ramfs: &'static ramfs::RamFs = &ramfs::RAMFS_INSTANCE;
    vfs.mount("/", ramfs);

    // Mount a separate TmpFS at "/tmp"
    let tmpfs: &'static ramfs::RamFs = &ramfs::TMPFS_INSTANCE;
    vfs.mount("/tmp", tmpfs);

    // Seed with default files
    drop(vfs);
    seed_default_files();

    crate::log_info!("VFS initialized: ramfs at /, tmpfs at /tmp.");
}

fn seed_default_files() {
    use crate::fs::VFS;
    let mut vfs = VFS.lock();
    let _ = vfs.mkdir("/boot");
    let _ = vfs.mkdir("/etc");
    let _ = vfs.mkdir("/home");
    let _ = vfs.create("/README.md");
    let _ = vfs.write_file("/README.md", b"# AtomicOS\nA hobby x86_64 kernel written in Rust.\n");
    let _ = vfs.create("/BUILD.md");
    let _ = vfs.write_file("/BUILD.md", b"# Build Dependencies\nnasm, ld, gcc, grub-mkrescue, qemu, rust nightly\n");
    let _ = vfs.create("/boot/kernel.bin");
    let _ = vfs.write_file("/boot/kernel.bin", b"[ELF binary]");
    let _ = vfs.create("/etc/hostname");
    let _ = vfs.write_file("/etc/hostname", b"atomicos\n");
}
