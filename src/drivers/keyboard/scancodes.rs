#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyCode {
    Char(char),
    Enter,
    Backspace,
    Unknown,
}

pub struct KeyboardState {
    shift_pressed: bool,
}

impl KeyboardState {
    pub const fn new() -> Self {
        Self {
            shift_pressed: false,
        }
    }

    pub fn process_scancode(&mut self, scancode: u8) -> KeyCode {
        // Set 1 Make Codes and Break Codes Handlers
        match scancode {
            // Numbers
            0x02 => self.char_with_shift('1', '!'),
            0x03 => self.char_with_shift('2', '@'),
            0x04 => self.char_with_shift('3', '#'),
            0x05 => self.char_with_shift('4', '$'),
            0x06 => self.char_with_shift('5', '%'),
            0x07 => self.char_with_shift('6', '^'),
            0x08 => self.char_with_shift('7', '&'),
            0x09 => self.char_with_shift('8', '*'),
            0x0A => self.char_with_shift('9', '('),
            0x0B => self.char_with_shift('0', ')'),
            0x0C => self.char_with_shift('-', '_'),
            0x0D => self.char_with_shift('=', '+'),

            // Letters Row 1
            0x10 => self.char_with_shift('q', 'Q'),
            0x11 => self.char_with_shift('w', 'W'),
            0x12 => self.char_with_shift('e', 'E'),
            0x13 => self.char_with_shift('r', 'R'),
            0x14 => self.char_with_shift('t', 'T'),
            0x15 => self.char_with_shift('y', 'Y'),
            0x16 => self.char_with_shift('u', 'U'),
            0x17 => self.char_with_shift('i', 'I'),
            0x18 => self.char_with_shift('o', 'O'),
            0x19 => self.char_with_shift('p', 'P'),
            0x1A => self.char_with_shift('[', '{'),
            0x1B => self.char_with_shift(']', '}'),

            // Letters Row 2
            0x1E => self.char_with_shift('a', 'A'),
            0x1F => self.char_with_shift('s', 'S'),
            0x20 => self.char_with_shift('d', 'D'),
            0x21 => self.char_with_shift('f', 'F'),
            0x22 => self.char_with_shift('g', 'G'),
            0x23 => self.char_with_shift('h', 'H'),
            0x24 => self.char_with_shift('j', 'J'),
            0x25 => self.char_with_shift('k', 'K'),
            0x26 => self.char_with_shift('l', 'L'),
            0x27 => self.char_with_shift(';', ':'),
            0x28 => self.char_with_shift('\'', '"'),
            0x29 => self.char_with_shift('`', '~'),
            0x2B => self.char_with_shift('\\', '|'),

            // Letters Row 3
            0x2C => self.char_with_shift('z', 'Z'),
            0x2D => self.char_with_shift('x', 'X'),
            0x2E => self.char_with_shift('c', 'C'),
            0x2F => self.char_with_shift('v', 'V'),
            0x30 => self.char_with_shift('b', 'B'),
            0x31 => self.char_with_shift('n', 'N'),
            0x32 => self.char_with_shift('m', 'M'),
            0x33 => self.char_with_shift(',', '<'),
            0x34 => self.char_with_shift('.', '>'),
            0x35 => self.char_with_shift('/', '?'),

            // Modifiers Make
            0x2A | 0x36 => { // LShift / RShift
                self.shift_pressed = true;
                KeyCode::Unknown
            },
            
            // Modifiers Break
            0xAA | 0xB6 => { // LShift / RShift release (scancode + 0x80)
                self.shift_pressed = false;
                KeyCode::Unknown
            },

            // Control Keys
            0x39 => KeyCode::Char(' '),
            0x1C => KeyCode::Enter,
            0x0E => KeyCode::Backspace,

            _ => KeyCode::Unknown,
        }
    }

    fn char_with_shift(&self, lower: char, upper: char) -> KeyCode {
        if self.shift_pressed {
            KeyCode::Char(upper)
        } else {
            KeyCode::Char(lower)
        }
    }
}
