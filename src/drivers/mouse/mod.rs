use core::sync::atomic::{AtomicUsize, Ordering};
use spin::Mutex;
use x86_64::instructions::port::Port;
use lazy_static::lazy_static;

const BUFFER_SIZE: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MouseEvent {
    pub left_button: bool,
    pub right_button: bool,
    pub middle_button: bool,
    pub x_movement: i16,
    pub y_movement: i16,
}

impl MouseEvent {
    pub const fn empty() -> Self {
        Self { left_button: false, right_button: false, middle_button: false, x_movement: 0, y_movement: 0 }
    }
}

pub struct MouseBuffer {
    buffer: [MouseEvent; BUFFER_SIZE],
    head: AtomicUsize,
    tail: AtomicUsize,
}

impl MouseBuffer {
    pub const fn new() -> Self {
        Self {
            buffer: [MouseEvent::empty(); BUFFER_SIZE],
            head: AtomicUsize::new(0),
            tail: AtomicUsize::new(0),
        }
    }

    pub fn push(&self, event: MouseEvent) -> Result<(), ()> {
        let head = self.head.load(Ordering::Acquire);
        let next_head = (head + 1) % BUFFER_SIZE;
        
        if next_head == self.tail.load(Ordering::Acquire) {
            return Err(()); // Buffer Full
        }

        unsafe {
            let slot = self.buffer.as_ptr().add(head) as *mut MouseEvent;
            *slot = event;
        }

        self.head.store(next_head, Ordering::Release);
        Ok(())
    }

    pub fn pop(&self) -> Option<MouseEvent> {
        let tail = self.tail.load(Ordering::Acquire);
        
        if tail == self.head.load(Ordering::Acquire) {
            return None; // Buffer Empty
        }

        let event = unsafe {
            let slot = self.buffer.as_ptr().add(tail) as *mut MouseEvent;
            *slot
        };

        self.tail.store((tail + 1) % BUFFER_SIZE, Ordering::Release);
        Some(event)
    }
}

pub struct MouseState {
    packet: [u8; 3],
    bytes_received: usize,
}

impl MouseState {
    pub const fn new() -> Self {
        Self {
            packet: [0; 3],
            bytes_received: 0,
        }
    }

    pub fn process_byte(&mut self, byte: u8) -> Option<MouseEvent> {
        // PS/2 limits packet alignment with bit 3 of the 1st byte always being 1
        if self.bytes_received == 0 && (byte & 0x08) == 0 {
            return None; // Out of sync, ignore byte
        }

        self.packet[self.bytes_received] = byte;
        self.bytes_received += 1;

        if self.bytes_received == 3 {
            self.bytes_received = 0;
            return Some(self.parse_packet());
        }

        None
    }

    fn parse_packet(&self) -> MouseEvent {
        let flags = self.packet[0];
        let x_mov = self.packet[1] as i16;
        let y_mov = self.packet[2] as i16;

        let left = (flags & 0b0000_0001) != 0;
        let right = (flags & 0b0000_0010) != 0;
        let middle = (flags & 0b0000_0100) != 0;

        // Apply 9-bit sign extensions if they are backwards
        let x_sign = (flags & 0b0001_0000) != 0;
        let y_sign = (flags & 0b0010_0000) != 0;

        // X overflow/sign
        let x_final = if x_sign { x_mov - 256 } else { x_mov };
        
        // Y overflow/sign (also PS/2 Y is bottom-left, typically screens are top-left so we invert later)
        let y_final = if y_sign { y_mov - 256 } else { y_mov };

        MouseEvent {
            left_button: left,
            right_button: right,
            middle_button: middle,
            x_movement: x_final,
            y_movement: -y_final, // Inverted for top-left 0,0 mapping
        }
    }
}

lazy_static! {
    pub static ref MOUSE_BUFFER: MouseBuffer = MouseBuffer::new();
    pub static ref MOUSE_STATE: Mutex<MouseState> = Mutex::new(MouseState::new());
}

fn wait_write_ready() {
    let mut status_port: Port<u8> = Port::new(0x64);
    // Wait until bit 1 is clear (input buffer empty)
    while unsafe { status_port.read() } & 0x02 != 0 {}
}

fn wait_read_ready() {
    let mut status_port: Port<u8> = Port::new(0x64);
    // Wait until bit 0 is set (output buffer full)
    while unsafe { status_port.read() } & 0x01 == 0 {}
}

fn write_command(cmd: u8) {
    wait_write_ready();
    let mut cmd_port: Port<u8> = Port::new(0x64);
    unsafe { cmd_port.write(cmd) };
}

fn write_data(data: u8) {
    wait_write_ready();
    let mut data_port: Port<u8> = Port::new(0x60);
    unsafe { data_port.write(data) };
}

fn read_data() -> u8 {
    wait_read_ready();
    let mut data_port: Port<u8> = Port::new(0x60);
    unsafe { data_port.read() }
}

pub fn init() {
    // Enable Aux Port on Controller
    write_command(0xA8); 

    // Retrieve Compaq Status Byte
    write_command(0x20); 
    let mut status = read_data();

    // Enable IRQ12 and disable clock line
    status |= 0b0000_0010; 
    status &= 0b1101_1111;

    // Write back Compaq Status Byte
    write_command(0x60);
    write_data(status);

    // Send the Enable Packet Streaming command to the mouse
    write_command(0xD4); // Tell controller to talk to mouse
    write_data(0xF4);    // Enable packet streaming

    // Read the ACK from the mouse (should be 0xFA)
    let _ack = read_data();
    
    crate::log_info!("PS/2 Mouse driver initialized.");
}

pub fn push_byte(byte: u8) {
    let mut state = MOUSE_STATE.lock();
    if let Some(event) = state.process_byte(byte) {
        let _ = MOUSE_BUFFER.push(event);
    }
}

pub fn try_read_event() -> Option<MouseEvent> {
    MOUSE_BUFFER.pop()
}
