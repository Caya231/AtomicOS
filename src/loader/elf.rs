use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use core::fmt;

// ══════════════════════════════════════════════════════════════
//  ELF64 constants
// ══════════════════════════════════════════════════════════════

const ELF_MAGIC: [u8; 4] = [0x7F, b'E', b'L', b'F'];
const ELFCLASS64: u8    = 2;
const ELFDATA2LSB: u8   = 1;  // Little endian
const ET_EXEC: u16      = 2;  // Executable file
const EM_X86_64: u16    = 62;
const PT_LOAD: u32      = 1;

// ══════════════════════════════════════════════════════════════
//  ELF64 structures (parsed from bytes)
// ══════════════════════════════════════════════════════════════

/// ELF64 File Header (64 bytes)
struct Elf64Ehdr {
    e_type: u16,
    e_machine: u16,
    e_entry: u64,
    e_phoff: u64,
    e_phentsize: u16,
    e_phnum: u16,
}

impl Elf64Ehdr {
    fn parse(data: &[u8]) -> Result<Self, ExecError> {
        if data.len() < 64 {
            return Err(ExecError::InvalidFormat);
        }

        // Validate magic
        if data[0..4] != ELF_MAGIC {
            return Err(ExecError::InvalidFormat);
        }

        // Validate class (64-bit)
        if data[4] != ELFCLASS64 {
            return Err(ExecError::UnsupportedArch);
        }

        // Validate endianness (little-endian)
        if data[5] != ELFDATA2LSB {
            return Err(ExecError::UnsupportedArch);
        }

        let e_type = u16::from_le_bytes([data[16], data[17]]);
        let e_machine = u16::from_le_bytes([data[18], data[19]]);

        if e_type != ET_EXEC {
            return Err(ExecError::UnsupportedType);
        }
        if e_machine != EM_X86_64 {
            return Err(ExecError::UnsupportedArch);
        }

        let e_entry = u64::from_le_bytes(data[24..32].try_into().unwrap());
        let e_phoff = u64::from_le_bytes(data[32..40].try_into().unwrap());
        let e_phentsize = u16::from_le_bytes([data[54], data[55]]);
        let e_phnum = u16::from_le_bytes([data[56], data[57]]);

        Ok(Elf64Ehdr {
            e_type,
            e_machine,
            e_entry,
            e_phoff,
            e_phentsize,
            e_phnum,
        })
    }
}

/// ELF64 Program Header (56 bytes)
struct Elf64Phdr {
    p_type: u32,
    p_flags: u32,
    p_offset: u64,
    p_vaddr: u64,
    p_filesz: u64,
    p_memsz: u64,
}

impl Elf64Phdr {
    fn parse(data: &[u8]) -> Result<Self, ExecError> {
        if data.len() < 56 {
            return Err(ExecError::InvalidFormat);
        }

        Ok(Elf64Phdr {
            p_type: u32::from_le_bytes(data[0..4].try_into().unwrap()),
            p_flags: u32::from_le_bytes(data[4..8].try_into().unwrap()),
            p_offset: u64::from_le_bytes(data[8..16].try_into().unwrap()),
            p_vaddr: u64::from_le_bytes(data[16..24].try_into().unwrap()),
            p_filesz: u64::from_le_bytes(data[32..40].try_into().unwrap()),
            p_memsz: u64::from_le_bytes(data[40..48].try_into().unwrap()),
        })
    }
}

// ══════════════════════════════════════════════════════════════
//  ExecError
// ══════════════════════════════════════════════════════════════

#[derive(Debug)]
pub enum ExecError {
    FileNotFound,
    InvalidFormat,
    UnsupportedArch,
    UnsupportedType,
    MemoryError,
    ReadError,
}

impl fmt::Display for ExecError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ExecError::FileNotFound    => write!(f, "File not found"),
            ExecError::InvalidFormat   => write!(f, "Invalid ELF format"),
            ExecError::UnsupportedArch => write!(f, "Unsupported architecture"),
            ExecError::UnsupportedType => write!(f, "Unsupported ELF type (need ET_EXEC)"),
            ExecError::MemoryError     => write!(f, "Memory allocation error"),
            ExecError::ReadError       => write!(f, "File read error"),
        }
    }
}

// ══════════════════════════════════════════════════════════════
//  ELF Loader
// ══════════════════════════════════════════════════════════════

/// Stack size for loaded programs (16 KiB).
const PROGRAM_STACK_SIZE: usize = 4096 * 4;

/// Load an ELF64 binary from the filesystem and create a scheduler task.
/// Returns the task ID on success.
pub fn load(path: &str) -> Result<u64, ExecError> {
    // 1. Read the entire file from VFS
    let file_data = read_file_all(path)?;

    // 2. Parse ELF header
    let ehdr = Elf64Ehdr::parse(&file_data)?;

    crate::log_info!("ELF: entry={:#x} phoff={} phnum={} phentsz={}",
        ehdr.e_entry, ehdr.e_phoff, ehdr.e_phnum, ehdr.e_phentsize);

    // 3. Process PT_LOAD segments — copy into a flat memory buffer
    //    Since we're Ring 0, we allocate a heap buffer and copy segments there.
    //    The entry point is relative to the loaded segments.
    let mut load_base: u64 = u64::MAX;
    let mut load_end: u64 = 0;

    // First pass: find the virtual address range
    for i in 0..ehdr.e_phnum as usize {
        let off = ehdr.e_phoff as usize + i * ehdr.e_phentsize as usize;
        let phdr = Elf64Phdr::parse(&file_data[off..])?;
        if phdr.p_type != PT_LOAD { continue; }

        if phdr.p_vaddr < load_base {
            load_base = phdr.p_vaddr;
        }
        let seg_end = phdr.p_vaddr + phdr.p_memsz;
        if seg_end > load_end {
            load_end = seg_end;
        }
    }

    if load_base == u64::MAX {
        return Err(ExecError::InvalidFormat); // no PT_LOAD segments
    }

    let total_size = (load_end - load_base) as usize;
    crate::log_info!("ELF: load range {:#x}..{:#x} ({} bytes)", load_base, load_end, total_size);

    // Allocate memory for the program image
    let mut image = vec![0u8; total_size];

    // Second pass: copy file data into the image
    for i in 0..ehdr.e_phnum as usize {
        let off = ehdr.e_phoff as usize + i * ehdr.e_phentsize as usize;
        let phdr = Elf64Phdr::parse(&file_data[off..])?;
        if phdr.p_type != PT_LOAD { continue; }

        let dest_offset = (phdr.p_vaddr - load_base) as usize;
        let file_offset = phdr.p_offset as usize;
        let file_size = phdr.p_filesz as usize;

        // Copy p_filesz bytes from file
        if file_offset + file_size <= file_data.len() {
            image[dest_offset..dest_offset + file_size]
                .copy_from_slice(&file_data[file_offset..file_offset + file_size]);
        }
        // p_memsz > p_filesz region is already zeroed (BSS)
    }

    // 4. Calculate the real entry point address in our heap buffer
    let image_base = image.as_ptr() as u64;
    let real_entry = image_base + (ehdr.e_entry - load_base);

    crate::log_info!("ELF: image at {:#x}, entry at {:#x}", image_base, real_entry);

    // 5. Create a task using spawn_raw
    let task_name = extract_filename(path);
    let task_id = {
        let mut sched = crate::scheduler::SCHEDULER.lock();
        let id = sched.spawn_raw(real_entry, &task_name, image);
        id
    };

    crate::log_info!("ELF: spawned task '{}' (id {})", task_name, task_id.0);

    Ok(task_id.0)
}

/// Read entire file contents from VFS.
fn read_file_all(path: &str) -> Result<Vec<u8>, ExecError> {
    let vfs = crate::fs::VFS.lock();

    // First, look up the inode to get file size
    let inode = vfs.lookup(path).map_err(|_| ExecError::FileNotFound)?;
    let size = inode.size;

    if size == 0 {
        return Err(ExecError::InvalidFormat);
    }

    // Read the file
    let mut buf = vec![0u8; size];
    let bytes_read = vfs.read_file(path, 0, &mut buf).map_err(|_| ExecError::ReadError)?;
    buf.truncate(bytes_read);

    Ok(buf)
}

/// Extract filename from path (for task name).
fn extract_filename(path: &str) -> String {
    path.rsplit('/').next().unwrap_or(path).into()
}
