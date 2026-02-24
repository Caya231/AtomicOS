use crate::drivers::keyboard;
use crate::drivers::keyboard::scancodes::KeyCode;
use crate::{print, println};

// Requer alocação (Heap) ativada.
// Como no_std ainda não ligou Bump Allocator em Phase 1,
// o TTY consumirá o Ring Buffer imprimindo os dados nativamente no momento.

pub fn init() {
    crate::log_info!("Virtual TTY System initialized.");
    print_prompt();
}

pub fn print_prompt() {
    print!("root@atomicos:~$ ");
}

pub fn process_input_loop() -> ! {
    // Endless loop consuming keyboard packets and piping them to VGA context
    loop {
        let key = keyboard::read_char();
        
        match key {
            KeyCode::Char(c) => print!("{}", c),
            KeyCode::Space => print!(" "),
            KeyCode::Enter => {
                println!();
                print_prompt();
            },
            KeyCode::Backspace => {
                crate::vga::WRITER.lock().backspace();
            },
            KeyCode::ArrowUp => print!("[Up]"),
            KeyCode::ArrowDown => print!("[Down]"),
            KeyCode::ArrowLeft => print!("[Left]"),
            KeyCode::ArrowRight => print!("[Right]"),
            KeyCode::F(num) => print!("[F{}]", num),
            KeyCode::Unknown => {}
        }

        // Tenta ler do mouse também de forma não-bloqueante
        if let Some(mouse_event) = crate::drivers::mouse::try_read_event() {
            // Se clicar com o esquerdo, avisa no log
            if mouse_event.left_button {
                crate::log_info!("Mouse Left Click at X: {}, Y: {}", mouse_event.x_movement, mouse_event.y_movement);
            }
        }
    }
}
