use crate::println;

const LOGO: &str = r#"
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

pub fn run(_args: &str) {
    println!("        AtomicOS x86_64");
    println!("  ========================");
    println!("{}", LOGO);
    println!("  OS:       AtomicOS 0.2.0");
    println!("  Arch:     x86_64");
    println!("  Kernel:   Rust (no_std)");
    println!("  Shell:    AtomicTTY v2");
    println!("  Memory:   Heap (Bump Alloc)");
    println!("  Drivers:  PS/2 KB + Mouse");
    println!("  Display:  VGA Text 80x25");
}
