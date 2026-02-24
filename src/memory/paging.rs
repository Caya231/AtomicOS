use x86_64::{
    structures::paging::PageTable,
    VirtAddr,
};

/// Initialize the Paging module.
/// 
/// This provides the base structure to manage memory.
/// In a more complex layout, this would construct the OffsetPageTable 
/// from the active level 4 table and abstract Virtual Memory.
pub unsafe fn init_paging(physical_memory_offset: VirtAddr) {
    let _active_level_4 = active_level_4_table(physical_memory_offset);
    // Future expansion: returning x86_64::structures::paging::OffsetPageTable<'static>
}

unsafe fn active_level_4_table(physical_memory_offset: VirtAddr) -> &'static mut PageTable {
    use x86_64::registers::control::Cr3;
    
    let (level_4_table_frame, _) = Cr3::read();
    let phys = level_4_table_frame.start_address();
    let virt = physical_memory_offset + phys.as_u64();
    let page_table_ptr: *mut PageTable = virt.as_mut_ptr();
    
    &mut *page_table_ptr
}
