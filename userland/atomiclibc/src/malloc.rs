use core::alloc::{GlobalAlloc, Layout};
use crate::unistd;

struct UserAllocator;

unsafe impl GlobalAlloc for UserAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // Simple bump allocator over `sys_brk`.
        // A real sys_brk requires requesting the current brk (sys_brk(0)),
        // adding the size, and setting the new brk.
        let current = unistd::brk(core::ptr::null_mut());
        
        // Align the current pointer
        let current_addr = current as usize;
        let align_offset = current_addr % layout.align();
        let aligned_addr = if align_offset == 0 {
            current_addr
        } else {
            current_addr + layout.align() - align_offset
        };
        
        let new_brk = (aligned_addr + layout.size()) as *mut u8;
        let set_brk = unistd::brk(new_brk);
        
        if set_brk == new_brk {
            aligned_addr as *mut u8
        } else {
            core::ptr::null_mut()
        }
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        // Bump allocator doesn't free.
    }
}

// Global allocator registration is commented out for now since we 
// haven't added `alloc` crate dependencies to atomiclibc yet, but 
// the primitive is ready when we want full Vec/String in userland!
/*
#[global_allocator]
static ALLOCATOR: UserAllocator = UserAllocator;
*/
