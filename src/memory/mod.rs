pub mod paging;
pub mod frame_allocator;

pub fn init() {
    crate::log_info!("Memory management initialized.");
}
