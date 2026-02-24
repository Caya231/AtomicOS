use crate::println;

pub fn run(_args: &str) {
    let cwd = crate::shell::state::CWD.lock().clone();
    println!("{}", cwd);
}
