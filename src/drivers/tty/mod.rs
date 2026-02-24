use crate::{print, println};
use crate::drivers::keyboard;
use crate::drivers::keyboard::scancodes::KeyCode;
use alloc::string::String;

pub fn init() {
    crate::log_info!("Virtual TTY System initialized.");
    print_prompt();
}

pub fn print_prompt() {
    let cwd = crate::shell::state::CWD.lock().clone();
    let display = if cwd == "/" { "~".into() } else { cwd };
    print!("root@atomicos:{}$ ", display);
}

pub fn process_input_loop() -> ! {
    let mut command_buffer = String::new();

    loop {
        let key = keyboard::read_char();
        
        match key {
            KeyCode::Char(c) => {
                print!("{}", c);
                command_buffer.push(c);
            }
            KeyCode::Space => {
                print!(" ");
                command_buffer.push(' ');
            }
            KeyCode::Enter => {
                println!();
                // Dispatch to shell command system
                crate::shell::exec_command(&command_buffer);
                command_buffer.clear();
                print_prompt();
            },
            KeyCode::Backspace => {
                if !command_buffer.is_empty() {
                    command_buffer.pop();
                    crate::vga::WRITER.lock().backspace();
                }
            },
            KeyCode::ArrowUp => {},
            KeyCode::ArrowDown => {},
            KeyCode::ArrowLeft => {},
            KeyCode::ArrowRight => {},
            KeyCode::F(_) => {},
            KeyCode::Unknown => {}
        }

        // Non-blocking mouse event check
        if let Some(mouse_event) = crate::drivers::mouse::try_read_event() {
            if mouse_event.left_button {
                crate::log_info!("Mouse Click at X:{}, Y:{}", mouse_event.x_movement, mouse_event.y_movement);
            }
        }
    }
}
