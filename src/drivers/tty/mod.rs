use crate::{print, println};
use crate::drivers::keyboard;
use crate::drivers::keyboard::scancodes::KeyCode;
use alloc::string::String;

pub fn init() {
    crate::log_info!("Virtual TTY System initialized.");
    print_prompt();
}

pub fn print_prompt() {
    print!("root@atomicos:~$ ");
}

const NEOFETCH_ART: &str = r#"
            .       
           / \      
          /   \     
    .----' .+. '----.
    |  _.-' | '-._  |
    '-'  ___+___  '-'
      .-'  (*)  '-.  
   .-' .---/ \---. '-.
  /  .-'   | |   '-. \
 | .'   .--+-+--.   '.| 
 |/  .-'   | |   '-. \|
  '-'  '---+-+---'  '-'
       '---/ \---'   
          \ /       
           '        
"#;

fn print_neofetch() {
    println!("        AtomicOS x86_64");
    println!("  ========================");
    println!("{}", NEOFETCH_ART);
    println!("  OS:       AtomicOS 0.1.0");
    println!("  Arch:     x86_64");
    println!("  Kernel:   Rust (no_std)");
    println!("  Shell:    AtomicTTY v1");
    println!("  Memory:   Heap (Bump Alloc)");
    println!("  Drivers:  PS/2 KB + Mouse");
    println!("  Display:  VGA Text 80x25");
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
                
                match command_buffer.trim() {
                    "neofetch" => print_neofetch(),
                    "clear" => crate::vga::WRITER.lock().clear_screen(),
                    "" => {},
                    cmd => println!("Command not found: {}", cmd)
                }

                command_buffer.clear();
                print_prompt();
            },
            KeyCode::Backspace => {
                if !command_buffer.is_empty() {
                    command_buffer.pop();
                    crate::vga::WRITER.lock().backspace();
                }
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
