use x86_64::instructions::port::Port;
use core::fmt;

// ──────────────────────────────────────────────────────────────
//  ATA PIO port offsets (relative to io_base)
// ──────────────────────────────────────────────────────────────

const DATA_REG: u16        = 0; // R/W data (16-bit)
const ERROR_REG: u16       = 1; // R: error / W: features
const SECTOR_COUNT: u16    = 2;
const LBA_LOW: u16         = 3;
const LBA_MID: u16         = 4;
const LBA_HIGH: u16        = 5;
const DRIVE_HEAD: u16      = 6;
const CMD_STATUS: u16      = 7; // R: status / W: command

// Status register bits
const STATUS_BSY: u8  = 0x80;
const STATUS_DRDY: u8 = 0x40;
const STATUS_DRQ: u8  = 0x08;
const STATUS_ERR: u8  = 0x01;
const STATUS_DF: u8   = 0x20;

// ATA commands
const CMD_IDENTIFY: u8      = 0xEC;
const CMD_READ_SECTORS: u8  = 0x20;
const CMD_WRITE_SECTORS: u8 = 0x30;
const CMD_CACHE_FLUSH: u8   = 0xE7;

// ──────────────────────────────────────────────────────────────
//  Error type
// ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
pub enum AtaError {
    DeviceNotFound,
    DeviceFault,
    BusyTimeout,
    DrqTimeout,
    IoError,
}

impl fmt::Display for AtaError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AtaError::DeviceNotFound => write!(f, "Device not found"),
            AtaError::DeviceFault    => write!(f, "Device fault"),
            AtaError::BusyTimeout    => write!(f, "BSY timeout"),
            AtaError::DrqTimeout     => write!(f, "DRQ timeout"),
            AtaError::IoError        => write!(f, "I/O error"),
        }
    }
}

pub type AtaResult<T> = Result<T, AtaError>;

// ──────────────────────────────────────────────────────────────
//  ATA Device
// ──────────────────────────────────────────────────────────────

pub struct AtaDevice {
    io_base: u16,
    ctrl_base: u16,
    is_master: bool,
    pub detected: bool,
}

impl AtaDevice {
    pub fn new(io_base: u16, ctrl_base: u16, is_master: bool) -> Self {
        AtaDevice {
            io_base,
            ctrl_base,
            is_master,
            detected: false,
        }
    }

    // ── Port I/O helpers ─────────────────────────────────────

    fn read_port(&self, offset: u16) -> u8 {
        let mut port = Port::<u8>::new(self.io_base + offset);
        unsafe { port.read() }
    }

    fn write_port(&self, offset: u16, val: u8) {
        let mut port = Port::<u8>::new(self.io_base + offset);
        unsafe { port.write(val) }
    }

    fn read_data16(&self) -> u16 {
        let mut port = Port::<u16>::new(self.io_base + DATA_REG);
        unsafe { port.read() }
    }

    fn write_data16(&self, val: u16) {
        let mut port = Port::<u16>::new(self.io_base + DATA_REG);
        unsafe { port.write(val) }
    }

    fn read_ctrl(&self) -> u8 {
        let mut port = Port::<u8>::new(self.ctrl_base);
        unsafe { port.read() }
    }

    // ── Status polling ───────────────────────────────────────

    /// Wait until BSY bit clears. Returns Err on timeout.
    fn wait_bsy(&self) -> AtaResult<()> {
        for _ in 0..100_000 {
            let status = self.read_port(CMD_STATUS);
            if status & STATUS_BSY == 0 {
                return Ok(());
            }
            core::hint::spin_loop();
        }
        Err(AtaError::BusyTimeout)
    }

    /// Wait until DRQ is set (data ready). Checks for errors.
    fn wait_drq(&self) -> AtaResult<()> {
        for _ in 0..100_000 {
            let status = self.read_port(CMD_STATUS);
            if status & STATUS_ERR != 0 || status & STATUS_DF != 0 {
                return Err(AtaError::DeviceFault);
            }
            if status & STATUS_DRQ != 0 {
                return Ok(());
            }
            core::hint::spin_loop();
        }
        Err(AtaError::DrqTimeout)
    }

    /// Perform the 400ns delay by reading the alternate status register 4 times.
    fn delay_400ns(&self) {
        for _ in 0..4 {
            let _ = self.read_ctrl();
        }
    }

    /// Select drive (master or slave).
    fn select_drive(&self) {
        let val = if self.is_master { 0xA0 } else { 0xB0 };
        self.write_port(DRIVE_HEAD, val);
        self.delay_400ns();
    }

    // ── IDENTIFY ─────────────────────────────────────────────

    /// Identify the disk. Sets `detected` to true on success.
    pub fn identify(&mut self) -> AtaResult<()> {
        self.select_drive();
        self.write_port(SECTOR_COUNT, 0);
        self.write_port(LBA_LOW, 0);
        self.write_port(LBA_MID, 0);
        self.write_port(LBA_HIGH, 0);
        self.write_port(CMD_STATUS, CMD_IDENTIFY);

        // Read status — if 0, no device
        let status = self.read_port(CMD_STATUS);
        if status == 0 {
            return Err(AtaError::DeviceNotFound);
        }

        // Wait for BSY to clear
        self.wait_bsy()?;

        // Check LBA mid/high — if non-zero, it's not ATA
        let mid = self.read_port(LBA_MID);
        let high = self.read_port(LBA_HIGH);
        if mid != 0 || high != 0 {
            return Err(AtaError::DeviceNotFound); // Not ATA (possibly ATAPI)
        }

        // Wait for DRQ or ERR
        self.wait_drq()?;

        // Read 256 words of identify data (discard for now)
        for _ in 0..256 {
            let _ = self.read_data16();
        }

        self.detected = true;
        Ok(())
    }

    // ── READ SECTOR (LBA28) ─────────────────────────────────

    /// Read one 512-byte sector at the given LBA.
    pub fn read_sector(&self, lba: u32, buf: &mut [u8; 512]) -> AtaResult<()> {
        if !self.detected {
            return Err(AtaError::DeviceNotFound);
        }

        self.wait_bsy()?;

        // Select drive + top 4 bits of LBA
        let head = if self.is_master { 0xE0 } else { 0xF0 };
        self.write_port(DRIVE_HEAD, head | ((lba >> 24) as u8 & 0x0F));
        self.delay_400ns();

        self.write_port(ERROR_REG, 0);            // features = 0
        self.write_port(SECTOR_COUNT, 1);          // read 1 sector
        self.write_port(LBA_LOW, lba as u8);
        self.write_port(LBA_MID, (lba >> 8) as u8);
        self.write_port(LBA_HIGH, (lba >> 16) as u8);
        self.write_port(CMD_STATUS, CMD_READ_SECTORS);

        self.wait_drq()?;

        // Read 256 words (512 bytes)
        for i in 0..256 {
            let word = self.read_data16();
            buf[i * 2]     = (word & 0xFF) as u8;
            buf[i * 2 + 1] = (word >> 8) as u8;
        }

        Ok(())
    }

    // ── WRITE SECTOR (LBA28) ────────────────────────────────

    /// Write one 512-byte sector at the given LBA.
    pub fn write_sector(&self, lba: u32, buf: &[u8; 512]) -> AtaResult<()> {
        if !self.detected {
            return Err(AtaError::DeviceNotFound);
        }

        self.wait_bsy()?;

        let head = if self.is_master { 0xE0 } else { 0xF0 };
        self.write_port(DRIVE_HEAD, head | ((lba >> 24) as u8 & 0x0F));
        self.delay_400ns();

        self.write_port(ERROR_REG, 0);
        self.write_port(SECTOR_COUNT, 1);
        self.write_port(LBA_LOW, lba as u8);
        self.write_port(LBA_MID, (lba >> 8) as u8);
        self.write_port(LBA_HIGH, (lba >> 16) as u8);
        self.write_port(CMD_STATUS, CMD_WRITE_SECTORS);

        self.wait_drq()?;

        // Write 256 words (512 bytes)
        for i in 0..256 {
            let word = (buf[i * 2] as u16) | ((buf[i * 2 + 1] as u16) << 8);
            self.write_data16(word);
        }

        // Cache flush
        self.write_port(CMD_STATUS, CMD_CACHE_FLUSH);
        self.wait_bsy()?;

        Ok(())
    }
}
