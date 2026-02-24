pub fn run(_args: &str) {
    crate::vga::WRITER.lock().clear_screen();
}
