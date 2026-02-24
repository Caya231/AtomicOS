use crate::println;
use x86_64::instructions::port::Port;

/// Read a single RTC register via CMOS ports 0x70/0x71.
fn read_cmos(reg: u8) -> u8 {
    let mut addr: Port<u8> = Port::new(0x70);
    let mut data: Port<u8> = Port::new(0x71);
    unsafe {
        addr.write(reg);
        data.read()
    }
}

/// Convert BCD-encoded byte to decimal.
fn bcd_to_dec(bcd: u8) -> u8 {
    (bcd & 0x0F) + ((bcd >> 4) * 10)
}

pub fn run(_args: &str) {
    let seconds = bcd_to_dec(read_cmos(0x00));
    let minutes = bcd_to_dec(read_cmos(0x02));
    let hours   = bcd_to_dec(read_cmos(0x04));
    let day     = bcd_to_dec(read_cmos(0x07));
    let month   = bcd_to_dec(read_cmos(0x08));
    let year    = bcd_to_dec(read_cmos(0x09)) as u16 + 2000;

    println!("{:04}-{:02}-{:02} {:02}:{:02}:{:02} UTC", year, month, day, hours, minutes, seconds);
}
