pub mod paging;
pub mod frame_allocator;

use frame_allocator::BumpFrameAllocator;
use spin::Mutex;
use lazy_static::lazy_static;

lazy_static! {
    pub static ref FRAME_ALLOCATOR: Mutex<BumpFrameAllocator> = Mutex::new(BumpFrameAllocator::new());
}

pub fn init(multiboot_info_addr: usize) {
    let boot_info = unsafe { multiboot2::BootInformation::load(multiboot_info_addr as *const _).expect("Failed to load Multiboot2 info!") };
    let memory_map_tag = boot_info.memory_map_tag().expect("Memory map tag required");

    // Rust no_std hack to keep the parser happy: Because memory areas live behind the BootInformation struct
    // we need to materialize them if we want to bypass lifetime constraints, but as we don't have alloc yet
    // we limit our Bump Allocator to borrow directly from the boot_info pointer memory segment.
    let areas = memory_map_tag.memory_areas();
    // Reconstruct a static slice from the raw pointer since multiboot2 tag memory is static anyway.
    let static_areas: &'static [multiboot2::MemoryArea] = unsafe {
        core::slice::from_raw_parts(
            areas.as_ptr(),
            areas.len()
        )
    };

    let mut allocator = FRAME_ALLOCATOR.lock();
    unsafe { allocator.init(static_areas) };
    
    // Test native single frame allocation visually
    use x86_64::structures::paging::FrameAllocator;
    let _first_frame = allocator.allocate_frame().unwrap();

    crate::log_info!("Physical Memory Frame Allocator initialized using Multiboot2 Map.");

    // Setup Paging
    // In our architecture, the bootloader (boot.asm) identity maps the first 1GB of memory.
    // This allows us to use physical address 0 as virtual address 0.
    use x86_64::VirtAddr;
    let phys_mem_offset = VirtAddr::new(0); // For identity mapping
    let mut mapper = unsafe { paging::init_paging(phys_mem_offset) };
    crate::log_info!("Virtual Memory Paging subsystem initialized.");

    // Initialize Heap Support (Dynamic Memory Allocation via #[global_allocator])
    crate::allocator::init_heap(&mut mapper, &mut *allocator)
        .expect("Heap initialization failed");
    
    crate::log_info!("Heap Allocator initialized successfully.");

    // Validate dynamic allocation features
    use alloc::vec;
    use alloc::string::String;
    let mut dynam_vec: vec::Vec<u32> = vec::Vec::new();
    for i in 0..500 {
        dynam_vec.push(i);
    }
    crate::log_info!("Dynamically allocated a {} elements vector at {:p}", dynam_vec.len(), dynam_vec.as_slice());

    let hello_alloc = String::from("String built from Heap!");
    crate::log_info!("Test dynamically stored string: {}", hello_alloc);
}
