use x86_64::{
    structures::paging::{FrameAllocator, PhysFrame, Size4KiB},
    PhysAddr,
};
use multiboot2::{MemoryArea, MemoryAreaType};

/// A simple bump allocator for physical memory frames.
pub struct BumpFrameAllocator {
    memory_areas: Option<&'static [MemoryArea]>,
    next_free_frame: usize,
}

impl BumpFrameAllocator {
    /// Create a new Empty BumpFrameAllocator.
    pub const fn new() -> Self {
        BumpFrameAllocator {
            memory_areas: None,
            next_free_frame: 0,
        }
    }

    /// Initialize the allocator with the multiboot memory map.
    pub unsafe fn init(&mut self, memory_areas: &'static [MemoryArea]) {
        self.memory_areas = Some(memory_areas);
    }
    
    /// Returns an iterator over the usable memory areas specified in the memory map.
    fn usable_frames(&self) -> impl Iterator<Item = PhysFrame> {
        // get usable areas from memory map
        let regions = self.memory_areas.unwrap().iter();
        let usable_regions = regions.filter(|r| r.typ() == MemoryAreaType::Available);
        
        // map each region to its address range
        let addr_ranges = usable_regions.map(|r| r.start_address()..r.end_address());
        
        // transform to an iterator of physical frames
        let frame_addresses = addr_ranges.flat_map(|r| r.step_by(4096));
        
        // Return valid physical frames
        frame_addresses.map(|addr| PhysFrame::containing_address(PhysAddr::new(addr)))
    }
}

unsafe impl FrameAllocator<Size4KiB> for BumpFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        let frame = self.usable_frames().nth(self.next_free_frame);
        self.next_free_frame += 1;
        frame
    }
}
