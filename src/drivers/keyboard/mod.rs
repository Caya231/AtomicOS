pub mod scancodes;

use spin::Mutex;
use lazy_static::lazy_static;
use core::sync::atomic::{AtomicUsize, Ordering};
use scancodes::{KeyCode, KeyboardState};
use x86_64::instructions::port::Port;

const BUFFER_SIZE: usize = 256;

pub struct KeyboardBuffer {
    buffer: [KeyCode; BUFFER_SIZE],
    head: AtomicUsize,
    tail: AtomicUsize,
}

impl KeyboardBuffer {
    pub const fn new() -> Self {
        Self {
            // Arrays of non-Copy enum requiring explicit loop initialization normally,
            // but for a strict `const` contexts under `no_std`, we force repeated items
            buffer: [KeyCode::Unknown; BUFFER_SIZE],
            head: AtomicUsize::new(0),
            tail: AtomicUsize::new(0),
        }
    }

    pub fn push(&self, key: KeyCode) -> Result<(), ()> {
        let head = self.head.load(Ordering::Acquire);
        let next_head = (head + 1) % BUFFER_SIZE;
        
        if next_head == self.tail.load(Ordering::Acquire) {
            return Err(()); // Buffer Full
        }

        // To mutate within an atomic-tracked struct without `&mut self` or Cell wrapper in interrupts:
        // We cast the reference natively relying on the atomic offsets preventing race overlaps between producer/consumer.
        // In this no_std minimal driver without alloc, we use unsafe pointer cast to bypass borrow checker just for the array slot.
        unsafe {
            let slot = self.buffer.as_ptr().add(head) as *mut KeyCode;
            *slot = key;
        }

        self.head.store(next_head, Ordering::Release);
        Ok(())
    }

    pub fn pop(&self) -> Option<KeyCode> {
        let tail = self.tail.load(Ordering::Acquire);
        
        if tail == self.head.load(Ordering::Acquire) {
            return None; // Buffer Empty
        }

        let key = unsafe {
            let slot = self.buffer.as_ptr().add(tail) as *mut KeyCode;
            *slot
        };

        self.tail.store((tail + 1) % BUFFER_SIZE, Ordering::Release);
        Some(key)
    }
}

lazy_static! {
    pub static ref KEYBOARD_BUFFER: KeyboardBuffer = KeyboardBuffer::new();
    pub static ref KEYBOARD_STATE: Mutex<KeyboardState> = Mutex::new(KeyboardState::new());
}

pub fn init() {
    // Escoar teclado inicial
    let mut port: Port<u8> = Port::new(0x60);
    // Lê o scancode residual se existir na inicialização da controladora 8042
    let _scancode = unsafe { port.read() };
    crate::log_info!("PS/2 Keyboard driver initialized.");
}

pub fn push_scancode(scancode: u8) {
    let mut state = KEYBOARD_STATE.lock();
    let keycode = state.process_scancode(scancode);
    
    // Ignore Unknowns like standalone modifiers
    if let KeyCode::Unknown = keycode {
        return;
    }

    // Try to enqueue
    let _ = KEYBOARD_BUFFER.push(keycode);
}

pub fn try_read_char() -> Option<KeyCode> {
    KEYBOARD_BUFFER.pop()
}

pub fn read_char() -> KeyCode {
    loop {
        if let Some(key) = try_read_char() {
            return key;
        }
        crate::scheduler::yield_now();
        x86_64::instructions::interrupts::enable_and_hlt();
    }
}
