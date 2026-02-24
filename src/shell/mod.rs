pub mod commands;
pub mod state;

use crate::println;

/// Parse input line into command + arguments, then dispatch.
pub fn exec_command(input: &str) {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return;
    }

    // Split by whitespace: first token = command, rest = args
    let parts: alloc::vec::Vec<&str> = trimmed.splitn(2, ' ').collect();
    let cmd = parts[0];
    let args = if parts.len() > 1 { parts[1] } else { "" };

    match cmd {
        "echo"        => commands::echo::run(args),
        "ls"          => commands::ls::run(args),
        "cat"         => commands::cat::run(args),
        "clear"       => commands::clear::run(args),
        "help"        => commands::help::run(args),
        "date"        => commands::date::run(args),
        "whoami"      => commands::whoami::run(args),
        "pwd"         => commands::pwd::run(args),
        "uptime"      => commands::uptime::run(args),
        "version"     => commands::version::run(args),
        "neofetch"    => commands::neofetch::run(args),
        "cd"          => commands::cd::run(args),
        "ps"          => commands::ps::run(args),
        "kill"        => commands::kill::run(args),
        "mkdir"       => commands::mkdir::run(args),
        "rm"          => commands::rm::run(args),
        "cp"          => commands::cp::run(args),
        "mv"          => commands::mv::run(args),
        "catbin"      => commands::catbin::run(args),
        "objdump"     => commands::objdump::run(args),
        "shellscript" => commands::shellscript::run(args),
        "log"         => commands::log::run(args),
        _             => println!("{}: command not found", cmd),
    }
}
