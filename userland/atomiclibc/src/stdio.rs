use crate::unistd;

pub fn puts(s: &str) {
    unistd::write(1, s.as_bytes());
    unistd::write(1, b"\n");
}

pub fn putchar(c: u8) {
    let buf = [c];
    unistd::write(1, &buf);
}

// A minimal `print!` macro equivalent mechanism for `printf`.
// It takes a string with custom formatting: %s (string), %d (int), %x (hex).
pub fn printf(format: &str, args: &[PrintfArg]) {
    let mut i = 0;
    let bytes = format.as_bytes();
    let mut arg_idx = 0;
    
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 1 < bytes.len() {
            i += 1;
            let fmt_char = bytes[i];
            
            if arg_idx < args.len() {
                match fmt_char {
                    b's' => {
                        if let PrintfArg::Str(s) = &args[arg_idx] {
                            unistd::write(1, s.as_bytes());
                        }
                    }
                    b'd' => {
                        if let PrintfArg::Int(n) = args[arg_idx] {
                            print_int(n);
                        }
                    }
                    b'x' => {
                        if let PrintfArg::Int(n) = args[arg_idx] {
                            print_hex(n as u64);
                        }
                    }
                    b'%' => {
                        putchar(b'%');
                        arg_idx -= 1; // Compensate
                    }
                    _ => {
                        putchar(b'%');
                        putchar(fmt_char);
                        arg_idx -= 1;
                    }
                }
                arg_idx += 1;
            } else if fmt_char == b'%' {
                putchar(b'%');
            } else {
                putchar(b'%');
                putchar(fmt_char);
            }
        } else {
            putchar(bytes[i]);
        }
        i += 1;
    }
}

pub enum PrintfArg<'a> {
    Str(&'a str),
    Int(i64),
}

fn print_int(mut n: i64) {
    if n == 0 {
        putchar(b'0');
        return;
    }
    if n < 0 {
        putchar(b'-');
        n = -n;
    }
    
    let mut buf = [0u8; 32];
    let mut i = 0;
    while n > 0 {
        buf[i] = b'0' + (n % 10) as u8;
        n /= 10;
        i += 1;
    }
    
    while i > 0 {
        i -= 1;
        putchar(buf[i]);
    }
}

fn print_hex(mut n: u64) {
    if n == 0 {
        putchar(b'0');
        return;
    }
    
    let mut buf = [0u8; 32];
    let mut i = 0;
    while n > 0 {
        let rem = (n % 16) as u8;
        buf[i] = if rem < 10 { b'0' + rem } else { b'a' + (rem - 10) };
        n /= 16;
        i += 1;
    }
    
    while i > 0 {
        i -= 1;
        putchar(buf[i]);
    }
}

#[macro_export]
macro_rules! printf {
    ($fmt:expr) => {
        $crate::stdio::printf($fmt, &[]);
    };
    ($fmt:expr, $($arg:expr),*) => {
        $crate::stdio::printf($fmt, &[
            $(
                $crate::stdio::into_printf_arg!($arg)
            ),*
        ]);
    };
}

#[macro_export]
macro_rules! into_printf_arg {
    ($arg:expr) => {
        // Simple type inference hack
        (&$arg).into_printf_arg()
    };
}

pub trait IntoPrintfArg {
    fn into_printf_arg(&self) -> PrintfArg;
}

impl IntoPrintfArg for &str {
    fn into_printf_arg(&self) -> PrintfArg { PrintfArg::Str(*self) }
}
impl IntoPrintfArg for i32 {
    fn into_printf_arg(&self) -> PrintfArg { PrintfArg::Int(*self as i64) }
}
impl IntoPrintfArg for u32 {
    fn into_printf_arg(&self) -> PrintfArg { PrintfArg::Int(*self as i64) }
}
impl IntoPrintfArg for usize {
    fn into_printf_arg(&self) -> PrintfArg { PrintfArg::Int(*self as i64) }
}
impl IntoPrintfArg for u64 {
    fn into_printf_arg(&self) -> PrintfArg { PrintfArg::Int(*self as i64) }
}
impl IntoPrintfArg for isize {
    fn into_printf_arg(&self) -> PrintfArg { PrintfArg::Int(*self as i64) }
}
