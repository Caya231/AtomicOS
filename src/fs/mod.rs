pub mod vfs;
pub mod inode;
pub mod file;
pub mod dentry;
pub mod mount;
pub mod error;
pub mod pipe;
pub mod fd;
pub mod ramfs;
pub mod fat32;

use spin::Mutex;
use lazy_static::lazy_static;
use vfs::Vfs;

lazy_static! {
    pub static ref VFS: Mutex<Vfs> = Mutex::new(Vfs::new());
}

// Static holder for the FAT32 filesystem instance (initialized at runtime)
static mut FAT32_FS: Option<fat32::Fat32Fs> = None;

/// Initialize the VFS with RAMFS at root.
pub fn init() {
    let mut vfs = VFS.lock();

    // Mount the primary RAMFS at "/"
    let ramfs: &'static ramfs::RamFs = &ramfs::RAMFS_INSTANCE;
    vfs.mount("/", ramfs);

    // Mount a separate TmpFS at "/tmp"
    let tmpfs: &'static ramfs::RamFs = &ramfs::TMPFS_INSTANCE;
    vfs.mount("/tmp", tmpfs);

    drop(vfs);
    seed_default_files();

    crate::log_info!("VFS initialized: ramfs at /, tmpfs at /tmp.");
}

/// Mount FAT32 from ATA disk. Must be called AFTER drivers::ata::init().
pub fn mount_fat32() {
    match fat32::Fat32Fs::init() {
        Ok(fs) => {
            unsafe {
                FAT32_FS = Some(fs);
                if let Some(ref fat) = FAT32_FS {
                    let mut vfs = VFS.lock();
                    let fat_ref: &'static fat32::Fat32Fs = &*(fat as *const fat32::Fat32Fs);
                    vfs.mount("/disk", fat_ref);
                }
            }
            crate::log_info!("FAT32 mounted at /disk.");
        }
        Err(e) => {
            crate::log_warn!("FAT32 mount failed: {} — /disk unavailable.", e);
        }
    }
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
