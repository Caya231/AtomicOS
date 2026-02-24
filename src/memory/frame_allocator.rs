use x86_64::{
    structures::paging::{FrameAllocator, PhysFrame, Size4KiB},
    PhysAddr,
};

/// A simple bump allocator for physical memory frames.
pub struct BumpFrameAllocator {
    next_free_frame: PhysFrame,
    current_limit: PhysFrame,
}

impl BumpFrameAllocator {
    /// Create a new Empty BumpFrameAllocator.
    pub fn new() -> Self {
        BumpFrameAllocator {
            next_free_frame: PhysFrame::containing_address(PhysAddr::new(0)),
            current_limit: PhysFrame::containing_address(PhysAddr::new(0)),
        }
    }

    /// Initialize the allocator with a start and end physical address.
    /// In a fully featured OS, this parses the multiboot memory map.
    pub unsafe fn init(&mut self, start: PhysAddr, end: PhysAddr) {
        self.next_free_frame = PhysFrame::containing_address(start);
        self.current_limit = PhysFrame::containing_address(end);
    }
}

unsafe impl FrameAllocator<Size4KiB> for BumpFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        if self.next_free_frame <= self.current_limit {
            let frame = self.next_free_frame;
            self.next_free_frame += 1;
            Some(frame)
        } else {
            None
        }
    }
}
