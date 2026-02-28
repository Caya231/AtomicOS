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
//  Usermode Trampoline
// ══════════════════════════════════════════════════════════════

/// Trampoline function — runs as a kernel task starting point for Ring 3 processes.
/// Since it's a raw entry, we receive the target entry and stack via registers R12 and R13
/// which we will craft manually in the `Context` builder.
#[unsafe(naked)]
pub extern "C" fn usermode_trampoline() {
    unsafe {
        core::arch::naked_asm!(
            "
            // R12 = user_entry
            // R13 = user_stack_top
            
            // Log entry
            // (Skipped complex logging in naked assembly for stability)

            // Calculate selectors
            // user_cs = 0x23 (Index 4, RPL 3)
            // user_ss = 0x1B (Index 3, RPL 3)
            mov ax, 0x1B
            mov ds, ax
            mov es, ax
            mov fs, ax
            mov gs, ax

            // IRETQ Frame construction on the Kernel Stack
            push 0x1B         // SS
            push r13          // RSP
            push 0x202        // RFLAGS (Interrupts enabled)
            push 0x23         // CS
            push r12          // RIP (entry point)

            iretq             // Jump to Ring 3!
            "
        );
    }
}

// ══════════════════════════════════════════════════════════════
//  ELF Loader
// ══════════════════════════════════════════════════════════════

/// Stack size for user programs (16 KiB).
const USER_STACK_SIZE: usize = 4096 * 4;

/// Load an ELF64 binary and create a Ring 3 task (Legacy boot support API).
pub fn load(path: &str) -> Result<u64, ExecError> {
    let params = parse_and_map_elf(path)?;

    // 8. Spawn process using Phase 5.3 Custom Scheduler Builder
    let task_name = extract_filename(path);
    
    let task_id = crate::scheduler::spawn_process(
        &task_name,
        params.page_table,
        usermode_trampoline as *const () as u64, // The Ring 0 kernel entry of this new process
        params.user_stack_top,
        params.allocations,
    );
    
    // Inject R12 and R13 into the freshly spawned process Context to feed the trampoline
    {
        let mut sched = crate::scheduler::SCHEDULER.lock();
        if let Some(proc) = sched.ready_queue.iter_mut().find(|p| p.pid == task_id) {
            proc.context.r12 = params.entry;
            proc.context.r13 = params.user_stack_top;
        }
    }

    crate::log_info!("ELF: spawned process '{}' (PID {})", task_name, task_id.0);
    Ok(task_id.0)
}

/// Represents the extracted core parameters of an ELF binary 
/// used to execute it seamlessly on an isolated Page Table.
pub struct ElfExecParams {
    pub page_table: u64,
    pub entry: u64,
    pub user_stack_top: u64,
    pub allocations: alloc::vec::Vec<(u64, u64)>,
}

/// Parse and map an ELF into a brand new isolated Address Space.
/// Returns the mapping parameters without modifying the scheduler.
pub fn parse_and_map_elf(path: &str) -> Result<ElfExecParams, ExecError> {
    let file_data = read_file_all(path)?;
    let ehdr = Elf64Ehdr::parse(&file_data)?;

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

    if load_base == u64::MAX { return Err(ExecError::InvalidFormat); }

    let load_end_aligned = (load_end + 4095) & !4095;
    let user_stack_base = load_end_aligned;
    let user_stack_top = user_stack_base + USER_STACK_SIZE as u64;

    let new_p4_phys = crate::memory::paging::create_new_page_table().ok_or(ExecError::MemoryError)?;
    let mut mapped_allocations = alloc::vec::Vec::new();
    
    use x86_64::registers::control::Cr3;
    let (old_p4, flags) = Cr3::read();
    
    unsafe {
        let new_frame = x86_64::structures::paging::PhysFrame::containing_address(new_p4_phys);
        Cr3::write(new_frame, flags);
    }
    
    let phys_mem_offset = x86_64::VirtAddr::new(0);
    let mut mapper = unsafe { crate::memory::paging::init_paging(phys_mem_offset) };

    let image_size = (load_end - load_base) as u64;
    if !crate::memory::paging::allocate_process_memory(&mut mapper, x86_64::VirtAddr::new(load_base), image_size) {
        unsafe { Cr3::write(old_p4, flags); }
        return Err(ExecError::MemoryError);
    }
    mapped_allocations.push((load_base, image_size));

    if !crate::memory::paging::allocate_process_memory(&mut mapper, x86_64::VirtAddr::new(user_stack_base), USER_STACK_SIZE as u64) {
        unsafe { Cr3::write(old_p4, flags); }
        return Err(ExecError::MemoryError);
    }
    mapped_allocations.push((user_stack_base, USER_STACK_SIZE as u64));

    for i in 0..ehdr.e_phnum as usize {
        let off = ehdr.e_phoff as usize + i * ehdr.e_phentsize as usize;
        let phdr = Elf64Phdr::parse(&file_data[off..])?;
        if phdr.p_type != PT_LOAD { continue; }

        let dest_ptr = phdr.p_vaddr as *mut u8;
        let file_offset = phdr.p_offset as usize;
        let file_size = phdr.p_filesz as usize;

        if file_offset + file_size <= file_data.len() {
            unsafe {
                core::ptr::copy_nonoverlapping(file_data[file_offset..].as_ptr(), dest_ptr, file_size);
            }
        }

        if phdr.p_memsz > phdr.p_filesz {
            let bss_size = (phdr.p_memsz - phdr.p_filesz) as usize;
            unsafe { core::ptr::write_bytes(dest_ptr.add(file_size), 0, bss_size); }
        }
    }

    unsafe { Cr3::write(old_p4, flags); }

    let real_entry = ehdr.e_entry;
    crate::log_info!("ELF Parsed: mapped at {:#x}, entry={:#x} stack_top={:#x} (Isolated P4 at {:#x})", load_base, real_entry, user_stack_top, new_p4_phys.as_u64());

    Ok(ElfExecParams {
        page_table: new_p4_phys.as_u64(),
        entry: real_entry,
        user_stack_top,
        allocations: mapped_allocations,
    })
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
