use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use core::fmt;

// ══════════════════════════════════════════════════════════════
//  ELF64 constants
// ══════════════════════════════════════════════════════════════

const ELF_MAGIC: [u8; 4] = [0x7F, b'E', b'L', b'F'];
const ELFCLASS64: u8    = 2;
const ELFDATA2LSB: u8   = 1;
const ET_EXEC: u16      = 2;
const EM_X86_64: u16    = 62;
const PT_LOAD: u32      = 1;

// ══════════════════════════════════════════════════════════════
//  ELF64 structures
// ══════════════════════════════════════════════════════════════

struct Elf64Ehdr {
    e_entry: u64,
    e_phoff: u64,
    e_phentsize: u16,
    e_phnum: u16,
}

impl Elf64Ehdr {
    fn parse(data: &[u8]) -> Result<Self, ExecError> {
        if data.len() < 64 { return Err(ExecError::InvalidFormat); }
        if data[0..4] != ELF_MAGIC { return Err(ExecError::InvalidFormat); }
        if data[4] != ELFCLASS64 { return Err(ExecError::UnsupportedArch); }
        if data[5] != ELFDATA2LSB { return Err(ExecError::UnsupportedArch); }

        let e_type = u16::from_le_bytes([data[16], data[17]]);
        let e_machine = u16::from_le_bytes([data[18], data[19]]);
        if e_type != ET_EXEC { return Err(ExecError::UnsupportedType); }
        if e_machine != EM_X86_64 { return Err(ExecError::UnsupportedArch); }

        Ok(Elf64Ehdr {
            e_entry: u64::from_le_bytes(data[24..32].try_into().unwrap()),
            e_phoff: u64::from_le_bytes(data[32..40].try_into().unwrap()),
            e_phentsize: u16::from_le_bytes([data[54], data[55]]),
            e_phnum: u16::from_le_bytes([data[56], data[57]]),
        })
    }
}

struct Elf64Phdr {
    p_type: u32,
    p_offset: u64,
    p_vaddr: u64,
    p_filesz: u64,
    p_memsz: u64,
}

impl Elf64Phdr {
    fn parse(data: &[u8]) -> Result<Self, ExecError> {
        if data.len() < 56 { return Err(ExecError::InvalidFormat); }
        Ok(Elf64Phdr {
            p_type: u32::from_le_bytes(data[0..4].try_into().unwrap()),
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
//  User-mode task info — stored globally so the trampoline can access it
// ══════════════════════════════════════════════════════════════

use spin::Mutex;

/// Info needed by the usermode trampoline (one at a time).
struct UserTaskInfo {
    entry: u64,
    user_stack_top: u64,
}

static PENDING_USER_TASK: Mutex<Option<UserTaskInfo>> = Mutex::new(None);

/// Trampoline function — runs as a kernel task, then jumps to Ring 3.
fn usermode_trampoline() {
    let info = {
        let mut pending = PENDING_USER_TASK.lock();
        pending.take().expect("no pending user task info")
    };

    let user_cs = crate::interrupts::gdt::user_code_selector().0;
    let user_ss = crate::interrupts::gdt::user_data_selector().0;

    crate::log_info!("ELF: jumping to Ring 3 — entry={:#x} stack={:#x} cs={:#x} ss={:#x}",
        info.entry, info.user_stack_top, user_cs, user_ss);

    crate::interrupts::usermode::jump_to_usermode(
        info.entry,
        info.user_stack_top,
        user_cs,
        user_ss,
    );
}

// ══════════════════════════════════════════════════════════════
//  ELF Loader
// ══════════════════════════════════════════════════════════════

/// Stack size for user programs (16 KiB).
const USER_STACK_SIZE: usize = 4096 * 4;

/// Load an ELF64 binary and create a Ring 3 task.
pub fn load(path: &str) -> Result<u64, ExecError> {
    // 1. Read entire file
    let file_data = read_file_all(path)?;

    // 2. Parse ELF header
    let ehdr = Elf64Ehdr::parse(&file_data)?;

    crate::log_info!("ELF: entry={:#x} phoff={} phnum={}", ehdr.e_entry, ehdr.e_phoff, ehdr.e_phnum);

    // 3. Find load range
    let mut load_base: u64 = u64::MAX;
    let mut load_end: u64 = 0;

    for i in 0..ehdr.e_phnum as usize {
        let off = ehdr.e_phoff as usize + i * ehdr.e_phentsize as usize;
        let phdr = Elf64Phdr::parse(&file_data[off..])?;
        if phdr.p_type != PT_LOAD { continue; }
        if phdr.p_vaddr < load_base { load_base = phdr.p_vaddr; }
        let seg_end = phdr.p_vaddr + phdr.p_memsz;
        if seg_end > load_end { load_end = seg_end; }
    }

    if load_base == u64::MAX {
        return Err(ExecError::InvalidFormat);
    }

    // 4. Compute user stack bounds
    let load_end_aligned = (load_end + 4095) & !4095;
    let user_stack_base = load_end_aligned;
    let user_stack_top = user_stack_base + USER_STACK_SIZE as u64;

    // 5. Allocate pages for the ELF segments
    let image_size = (load_end - load_base) as u64;
    if !crate::memory::paging::allocate_user_memory(x86_64::VirtAddr::new(load_base), image_size) {
        return Err(ExecError::MemoryError);
    }

    // 6. Allocate pages for the User Stack
    if !crate::memory::paging::allocate_user_memory(x86_64::VirtAddr::new(user_stack_base), USER_STACK_SIZE as u64) {
        return Err(ExecError::MemoryError);
    }

    // 7. Copy file data directly into the allocated memory
    for i in 0..ehdr.e_phnum as usize {
        let off = ehdr.e_phoff as usize + i * ehdr.e_phentsize as usize;
        let phdr = Elf64Phdr::parse(&file_data[off..])?;
        if phdr.p_type != PT_LOAD { continue; }

        let dest_ptr = phdr.p_vaddr as *mut u8;
        let file_offset = phdr.p_offset as usize;
        let file_size = phdr.p_filesz as usize;

        if file_offset + file_size <= file_data.len() {
            unsafe {
                core::ptr::copy_nonoverlapping(
                    file_data[file_offset..].as_ptr(),
                    dest_ptr,
                    file_size,
                );
            }
        }

        // BSS zeroing
        if phdr.p_memsz > phdr.p_filesz {
            let bss_size = (phdr.p_memsz - phdr.p_filesz) as usize;
            unsafe {
                core::ptr::write_bytes(dest_ptr.add(file_size), 0, bss_size);
            }
        }
    }

    let real_entry = ehdr.e_entry;

    crate::log_info!("ELF: mapped at {:#x}, entry={:#x} stack_top={:#x}", load_base, real_entry, user_stack_top);

    // 7. Set pending task info for trampoline
    {
        let mut pending = PENDING_USER_TASK.lock();
        *pending = Some(UserTaskInfo {
            entry: real_entry,
            user_stack_top,
        });
    }

    // 8. Spawn kernel task that will trampoline to Ring 3
    let task_name = extract_filename(path);
    let task_id = {
        let mut sched = crate::scheduler::SCHEDULER.lock();
        let id = sched.spawn_raw(
            usermode_trampoline as *const () as u64,
            &task_name,
            alloc::vec::Vec::new(), // memory is dynamically mapped now
        );
        id
    };

    crate::log_info!("ELF: spawned task '{}' (id {})", task_name, task_id.0);
    Ok(task_id.0)
}

fn read_file_all(path: &str) -> Result<Vec<u8>, ExecError> {
    let vfs = crate::fs::VFS.lock();
    let inode = vfs.lookup(path).map_err(|_| ExecError::FileNotFound)?;
    if inode.size == 0 { return Err(ExecError::InvalidFormat); }
    let mut buf = vec![0u8; inode.size];
    let bytes_read = vfs.read_file(path, 0, &mut buf).map_err(|_| ExecError::ReadError)?;
    buf.truncate(bytes_read);
    Ok(buf)
}

fn extract_filename(path: &str) -> String {
    path.rsplit('/').next().unwrap_or(path).into()
}
