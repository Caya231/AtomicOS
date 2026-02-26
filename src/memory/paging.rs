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
