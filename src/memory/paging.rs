use x86_64::{
    structures::paging::{
        FrameAllocator, Mapper, OffsetPageTable, Page, PageTable, PageTableFlags, PhysFrame, Size4KiB
    },
    PhysAddr, VirtAddr,
};

/// Initialize a new OffsetPageTable.
pub unsafe fn init_paging(physical_memory_offset: VirtAddr) -> OffsetPageTable<'static> {
    let level_4_table = active_level_4_table(physical_memory_offset);
    OffsetPageTable::new(level_4_table, physical_memory_offset)
}

/// Returns a mutable reference to the active level 4 table.
unsafe fn active_level_4_table(physical_memory_offset: VirtAddr) -> &'static mut PageTable {
    use x86_64::registers::control::Cr3;

    let (level_4_table_frame, _) = Cr3::read();
    let phys = level_4_table_frame.start_address();
    let virt = physical_memory_offset + phys.as_u64();
    let page_table_ptr: *mut PageTable = virt.as_mut_ptr();

    &mut *page_table_ptr
}

/// Map a specific virtual page to a physical frame.
pub fn create_mapping(
    page: Page,
    frame: PhysFrame,
    mapper: &mut impl Mapper<Size4KiB>,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
) {
    let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
    // Map the page and trigger internal memory allocation tables using the Frame Allocator if missing
    let map_to_result = unsafe {
        mapper.map_to(page, frame, flags, frame_allocator)
    };
    map_to_result.expect("Map to failed").flush();
}

/// Allocate and map memory for a user program at a specific virtual address.
/// Returns true if successful.
pub fn allocate_user_memory(start_addr: VirtAddr, size_bytes: u64) -> bool {
    use x86_64::structures::paging::{PageTableFlags, Page, Mapper};
    if size_bytes == 0 { return true; }

    let phys_mem_offset = VirtAddr::new(0);
    let mut mapper = unsafe { init_paging(phys_mem_offset) };
    let mut frame_allocator = crate::memory::FRAME_ALLOCATOR.lock();

    let start_page = Page::<Size4KiB>::containing_address(start_addr);
    let end_page = Page::<Size4KiB>::containing_address(start_addr + size_bytes - 1u64);

    let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::USER_ACCESSIBLE;

    for page in Page::range_inclusive(start_page, end_page) {
        // Allocate physical frame
        let frame = match frame_allocator.allocate_frame() {
            Some(f) => f,
            None => return false, // Out of memory
        };

        // Map it
        unsafe {
            match mapper.map_to(page, frame, flags, &mut *frame_allocator) {
                Ok(flush) => flush.flush(),
                Err(_) => return false, // Mapping failed
            }
        }
    }
    true
}

/// Create a new isolated Page Table (P4) for a new Process.
/// It clones the Kernel's higher-half mappings into the new P4, leaving user-space empty.
pub fn create_new_page_table() -> Option<PhysAddr> {
    let mut frame_allocator = crate::memory::FRAME_ALLOCATOR.lock();
    
    // Allocate a new physical frame for the P4 table
    let p4_frame = frame_allocator.allocate_frame()?;
    
    let phys_mem_offset = VirtAddr::new(0);
    
    unsafe {
        // Zero out the new P4
        let p4_virt = phys_mem_offset + p4_frame.start_address().as_u64();
        core::ptr::write_bytes(p4_virt.as_mut_ptr::<u8>(), 0, 4096);
        
        // Clone the active Kernel P4
        use x86_64::registers::control::Cr3;
        let (active_p4_frame, _) = Cr3::read();
        let active_p4_virt = phys_mem_offset + active_p4_frame.start_address().as_u64();
        
        let new_p4 = &mut *p4_virt.as_mut_ptr::<PageTable>();
        let active_p4 = &*active_p4_virt.as_ptr::<PageTable>();
        
        // We only copy indices that correspond to Kernel space (higher half).
        // Since we are identity mapping for now (lower half = kernel), we'll do an exact copy
        // for indices EXCEPT the ones where the Elf loader places User code (e.g., 0x80000000 -> index 1)
        // For simplicity in Phase 5.3, we clone *everything* except explicitly cleared user regions.
        for i in 256..512 {
            new_p4[i] = active_p4[i].clone();
        }
        
        // Also clone Kernel Heap Mapping (which AtomicOS placed at index 136)
        new_p4[136] = active_p4[136].clone();
        
        // Hack for Phase 5.3 Identity Mapping preservation:
        // We cannot just clone `active_p4[0]` because that shares the P3 table.
        // User space loads at `0x80000000` (P3 index 2). If we share P3, we break isolation!
        // Instead, we allocate a NEW P3 table, and copy only the kernel's portion (P3 index 0).
        if active_p4[0].flags().contains(x86_64::structures::paging::PageTableFlags::PRESENT) {
            let p3_frame = frame_allocator.allocate_frame()?;
            let p3_virt = phys_mem_offset + p3_frame.start_address().as_u64();
            core::ptr::write_bytes(p3_virt.as_mut_ptr::<u8>(), 0, 4096);
            let new_p3 = &mut *p3_virt.as_mut_ptr::<PageTable>();

            let active_p3_virt = phys_mem_offset + active_p4[0].addr().as_u64();
            let active_p3 = &*active_p3_virt.as_ptr::<PageTable>();

            // Copy only the first 2 GB (P3 index 0 and 1) representing the kernel's lower mapping
            new_p3[0] = active_p3[0].clone();
            new_p3[1] = active_p3[1].clone();
            
            // Set P4[0] to point to the isolated P3. MUST include USER_ACCESSIBLE because
            // User Space lives in P4[0] -> P3[2] -> P2 etc.
            let mut flags = active_p4[0].flags();
            flags.insert(x86_64::structures::paging::PageTableFlags::USER_ACCESSIBLE);
            new_p4[0].set_addr(p3_frame.start_address(), flags);
        }
    }
    
    Some(p4_frame.start_address())
}

/// Allocate and map memory for a user process given its specific Page Table.
pub fn allocate_process_memory(mapper: &mut OffsetPageTable, start_addr: VirtAddr, size_bytes: u64) -> bool {
    use x86_64::structures::paging::{PageTableFlags, Page, Mapper};
    if size_bytes == 0 { return true; }

    let mut frame_allocator = crate::memory::FRAME_ALLOCATOR.lock();

    let start_page = Page::<Size4KiB>::containing_address(start_addr);
    let end_page = Page::<Size4KiB>::containing_address(start_addr + size_bytes - 1u64);

    // DPL=3 mapping
    let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::USER_ACCESSIBLE;

    for page in Page::range_inclusive(start_page, end_page) {
        let frame = match frame_allocator.allocate_frame() {
            Some(f) => f,
            None => return false,
        };

        unsafe {
            match mapper.map_to(page, frame, flags, &mut *frame_allocator) {
                Ok(flush) => flush.flush(),
                Err(_) => return false,
            }
        }
    }
    true
}

/// Free virtual user memory space back into the void (Cleanup for Exit).
pub fn free_user_memory(start_addr: VirtAddr, size_bytes: u64) {
    use x86_64::structures::paging::{Page, Mapper};
    let phys_mem_offset = VirtAddr::new(0);
    // Note: since this is called during `exit_current`, the process' CR3 is still loaded.
    let mut mapper = unsafe { init_paging(phys_mem_offset) };
    
    let start_page = Page::<Size4KiB>::containing_address(start_addr);
    let end_page = Page::<Size4KiB>::containing_address(start_addr + size_bytes - 1u64);
    
    for page in Page::range_inclusive(start_page, end_page) {
        if let Ok((_frame, flush)) = mapper.unmap(page) {
            flush.flush();
            // Note: In a real system, we'd also flag the _frame as free in the Bitmap Allocator.
            // Since we're using a Bump Allocator, physical frame recycling isn't fully supported yet,
            // but the Virtual Memory space is correctly unmapped and TLB flushed!
        }
    }
}

/// Helper for `fork` syscall: Clones memory blocks mapped in the Parent's P4 into a brand new Child P4.
pub fn deep_clone_process_memory(
    child_p4_addr: PhysAddr,
    allocations: &alloc::vec::Vec<(u64, u64)>
) -> bool {
    use x86_64::registers::control::Cr3;
    use x86_64::structures::paging::{PageTableFlags, Page, Mapper, Translate};

    let phys_mem_offset = VirtAddr::new(0);
    // Active mapper (Parent)
    let mut parent_mapper = unsafe { init_paging(phys_mem_offset) };
    
    // Switch to child temporarily to allocate frames
    let (old_p4, flags) = Cr3::read();
    unsafe {
        Cr3::write(PhysFrame::containing_address(child_p4_addr), flags);
    }
    
    let mut child_mapper = unsafe { init_paging(phys_mem_offset) };
    
    for (start_vaddr, size) in allocations {
        if !allocate_process_memory(&mut child_mapper, VirtAddr::new(*start_vaddr), *size) {
            unsafe { Cr3::write(old_p4, flags); }
            return false;
        }
    }
    
    // Switch back to parent to read from User Space
    unsafe {
        Cr3::write(old_p4, flags);
    }
    
    // Now, for every allocated page, we must copy data from parent virtual address
    // to child's physical frame. Since identity mapping covers all physical memory (0 offset)
    // we can write directly to the physical frames mapped by the child!
    unsafe {
        // Build child mapper again (virtually but pointing to child's P4 physical address manually)
        let child_p4_virt = phys_mem_offset + child_p4_addr.as_u64();
        let child_page_table = &mut *child_p4_virt.as_mut_ptr::<PageTable>();
        let child_mapper_offset = OffsetPageTable::new(child_page_table, phys_mem_offset);

        for (start_vaddr, size) in allocations {
            let start_page = Page::<Size4KiB>::containing_address(VirtAddr::new(*start_vaddr));
            let end_page = Page::<Size4KiB>::containing_address(VirtAddr::new(*start_vaddr + *size - 1));
            
            for page in Page::range_inclusive(start_page, end_page) {
                // Address in Parent Space (Source)
                let parent_ptr = page.start_address().as_ptr::<u8>();
                
                // Get the physical frame the Child allocated for this page
                if let Ok(child_phys_frame) = child_mapper_offset.translate_page(page) {
                    // The identity map gives us direct access to any physical memory.
                    let target_ptr = (phys_mem_offset + child_phys_frame.start_address().as_u64()).as_mut_ptr::<u8>();
                    
                    // Deep copy 4096 bytes
                    core::ptr::copy_nonoverlapping(parent_ptr, target_ptr, 4096);
                }
            }
        }
    }

    true
}
