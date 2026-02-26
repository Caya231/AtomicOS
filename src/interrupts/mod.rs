pub mod gdt;
pub mod idt;
pub mod usermode;

pub fn init() {
    gdt::init();
    idt::init();
    unsafe { idt::PICS.lock().initialize() };
}
