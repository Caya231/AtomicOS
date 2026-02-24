use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use spin::Mutex;

use crate::drivers::ata::PRIMARY_ATA;
use crate::fs::dentry::DirEntry as VfsDirEntry;
use crate::fs::error::{FsError, FsResult};
use crate::fs::inode::{FileType, Inode};
use crate::fs::mount::FileSystem;

// ══════════════════════════════════════════════════════════════
//  Constants
// ══════════════════════════════════════════════════════════════

const SECTOR_SIZE: usize = 512;
const DIR_ENTRY_SIZE: usize = 32;
const ENTRIES_PER_SECTOR: usize = SECTOR_SIZE / DIR_ENTRY_SIZE;

// FAT32 special cluster values
const FAT_EOC: u32   = 0x0FFF_FFF8; // end-of-chain marker (>= this)
const FAT_FREE: u32  = 0x0000_0000;

// Directory entry attribute bits
const ATTR_READ_ONLY: u8 = 0x01;
const ATTR_HIDDEN: u8    = 0x02;
const ATTR_SYSTEM: u8    = 0x04;
const ATTR_VOLUME_ID: u8 = 0x08;
const ATTR_DIRECTORY: u8 = 0x10;
const ATTR_ARCHIVE: u8   = 0x20;
const ATTR_LFN: u8       = 0x0F;

// ══════════════════════════════════════════════════════════════
//  BPB — BIOS Parameter Block (parsed from boot sector)
// ══════════════════════════════════════════════════════════════

#[derive(Debug, Clone)]
struct Bpb {
    bytes_per_sector: u16,
    sectors_per_cluster: u8,
    reserved_sectors: u16,
    num_fats: u8,
    total_sectors: u32,
    fat_size: u32,         // sectors per FAT
    root_cluster: u32,
    // Computed
    fat_start: u32,        // first sector of FAT
    data_start: u32,       // first sector of data area
}

impl Bpb {
    fn parse(sector: &[u8; 512]) -> FsResult<Self> {
        // Validate boot signature
        if sector[510] != 0x55 || sector[511] != 0xAA {
            return Err(FsError::InvalidPath); // not a valid boot sector
        }

        let bytes_per_sector = u16::from_le_bytes([sector[11], sector[12]]);
        let sectors_per_cluster = sector[13];
        let reserved_sectors = u16::from_le_bytes([sector[14], sector[15]]);
        let num_fats = sector[16];

        // Total sectors: try 16-bit first, then 32-bit
        let total_16 = u16::from_le_bytes([sector[19], sector[20]]);
        let total_32 = u32::from_le_bytes([sector[32], sector[33], sector[34], sector[35]]);
        let total_sectors = if total_16 != 0 { total_16 as u32 } else { total_32 };

        // FAT size: try 16-bit first (FAT12/16), then 32-bit (FAT32)
        let fat16 = u16::from_le_bytes([sector[22], sector[23]]);
        let fat32 = u32::from_le_bytes([sector[36], sector[37], sector[38], sector[39]]);
        let fat_size = if fat16 != 0 { fat16 as u32 } else { fat32 };

        let root_cluster = u32::from_le_bytes([sector[44], sector[45], sector[46], sector[47]]);

        let fat_start = reserved_sectors as u32;
        let data_start = fat_start + (num_fats as u32) * fat_size;

        Ok(Bpb {
            bytes_per_sector,
            sectors_per_cluster,
            reserved_sectors,
            num_fats,
            total_sectors,
            fat_size,
            root_cluster,
            fat_start,
            data_start,
        })
    }

    /// Convert a cluster number to its first sector in the data area.
    fn cluster_to_sector(&self, cluster: u32) -> u32 {
        self.data_start + (cluster - 2) * self.sectors_per_cluster as u32
    }
}

// ══════════════════════════════════════════════════════════════
//  Raw FAT32 directory entry (32 bytes)
// ══════════════════════════════════════════════════════════════

#[derive(Clone)]
struct RawDirEntry {
    name: [u8; 11],    // 8.3 name
    attr: u8,
    cluster_hi: u16,
    cluster_lo: u16,
    file_size: u32,
}

impl RawDirEntry {
    fn from_bytes(data: &[u8]) -> Self {
        RawDirEntry {
            name: {
                let mut n = [0u8; 11];
                n.copy_from_slice(&data[0..11]);
                n
            },
            attr: data[11],
            cluster_hi: u16::from_le_bytes([data[20], data[21]]),
            cluster_lo: u16::from_le_bytes([data[26], data[27]]),
            file_size: u32::from_le_bytes([data[28], data[29], data[30], data[31]]),
        }
    }

    fn to_bytes(&self) -> [u8; 32] {
        let mut buf = [0u8; 32];
        buf[0..11].copy_from_slice(&self.name);
        buf[11] = self.attr;
        buf[20] = self.cluster_hi as u8;
        buf[21] = (self.cluster_hi >> 8) as u8;
        buf[26] = self.cluster_lo as u8;
        buf[27] = (self.cluster_lo >> 8) as u8;
        buf[28..32].copy_from_slice(&self.file_size.to_le_bytes());
        buf
    }

    fn first_cluster(&self) -> u32 {
        ((self.cluster_hi as u32) << 16) | (self.cluster_lo as u32)
    }

    fn is_free(&self) -> bool {
        self.name[0] == 0x00
    }

    fn is_deleted(&self) -> bool {
        self.name[0] == 0xE5
    }

    fn is_lfn(&self) -> bool {
        self.attr == ATTR_LFN
    }

    fn is_dir(&self) -> bool {
        self.attr & ATTR_DIRECTORY != 0
    }

    fn is_volume_id(&self) -> bool {
        self.attr & ATTR_VOLUME_ID != 0
    }

    /// Convert the 8.3 name to a human-readable string.
    fn display_name(&self) -> String {
        let base = core::str::from_utf8(&self.name[0..8]).unwrap_or("").trim();
        let ext = core::str::from_utf8(&self.name[8..11]).unwrap_or("").trim();
        if ext.is_empty() {
            String::from(base)
        } else {
            alloc::format!("{}.{}", base, ext)
        }
    }
}

/// Encode a filename into 8.3 format. Returns None if invalid.
fn encode_83_name(name: &str) -> Option<[u8; 11]> {
    let name = name.trim();
    if name.is_empty() || name.len() > 12 {
        return None;
    }

    let mut result = [0x20u8; 11]; // space-padded

    let (base, ext) = if let Some(dot_pos) = name.rfind('.') {
        (&name[..dot_pos], &name[dot_pos + 1..])
    } else {
        (name, "")
    };

    if base.len() > 8 || ext.len() > 3 {
        return None;
    }

    for (i, c) in base.chars().enumerate() {
        if i >= 8 { break; }
        result[i] = (c as u8).to_ascii_uppercase();
    }
    for (i, c) in ext.chars().enumerate() {
        if i >= 3 { break; }
        result[8 + i] = (c as u8).to_ascii_uppercase();
    }

    Some(result)
}

// ══════════════════════════════════════════════════════════════
//  Fat32Fs — main filesystem struct
// ══════════════════════════════════════════════════════════════

struct Fat32Inner {
    bpb: Bpb,
}

pub struct Fat32Fs {
    inner: Mutex<Fat32Inner>,
}

impl Fat32Fs {
    /// Create and initialize a Fat32Fs by reading the BPB from disk.
    pub fn init() -> FsResult<Self> {
        let mut sector = [0u8; 512];
        {
            let ata = PRIMARY_ATA.lock();
            ata.read_sector(0, &mut sector).map_err(|_| FsError::IoError)?;
        }

        let bpb = Bpb::parse(&sector)?;

        crate::log_info!("FAT32: BPS={} SPC={} FATs={} FATsz={} root_clus={} data_start={}",
            bpb.bytes_per_sector, bpb.sectors_per_cluster,
            bpb.num_fats, bpb.fat_size, bpb.root_cluster, bpb.data_start);

        Ok(Fat32Fs {
            inner: Mutex::new(Fat32Inner { bpb }),
        })
    }

    // ── Low-level disk I/O helpers ──────────────────────────

    fn read_sector_raw(lba: u32) -> FsResult<[u8; 512]> {
        let mut buf = [0u8; 512];
        let ata = PRIMARY_ATA.lock();
        ata.read_sector(lba, &mut buf).map_err(|_| FsError::IoError)?;
        Ok(buf)
    }

    fn write_sector_raw(lba: u32, buf: &[u8; 512]) -> FsResult<()> {
        let ata = PRIMARY_ATA.lock();
        ata.write_sector(lba, buf).map_err(|_| FsError::IoError)?;
        Ok(())
    }

    // ── FAT operations ──────────────────────────────────────

    /// Read the next cluster from the FAT.
    fn fat_read(bpb: &Bpb, cluster: u32) -> FsResult<u32> {
        let fat_offset = cluster * 4;
        let fat_sector = bpb.fat_start + (fat_offset / SECTOR_SIZE as u32);
        let offset_in_sector = (fat_offset % SECTOR_SIZE as u32) as usize;

        let sector = Self::read_sector_raw(fat_sector)?;
        let val = u32::from_le_bytes([
            sector[offset_in_sector],
            sector[offset_in_sector + 1],
            sector[offset_in_sector + 2],
            sector[offset_in_sector + 3],
        ]) & 0x0FFF_FFFF;

        Ok(val)
    }

    /// Write a value to the FAT (both copies).
    fn fat_write(bpb: &Bpb, cluster: u32, value: u32) -> FsResult<()> {
        let fat_offset = cluster * 4;
        let fat_sector_offset = fat_offset / SECTOR_SIZE as u32;
        let offset_in_sector = (fat_offset % SECTOR_SIZE as u32) as usize;

        // Update each FAT copy
        for fat_idx in 0..bpb.num_fats as u32 {
            let sector_lba = bpb.fat_start + fat_idx * bpb.fat_size + fat_sector_offset;
            let mut sector = Self::read_sector_raw(sector_lba)?;

            // Preserve top 4 bits
            let existing = u32::from_le_bytes([
                sector[offset_in_sector],
                sector[offset_in_sector + 1],
                sector[offset_in_sector + 2],
                sector[offset_in_sector + 3],
            ]);
            let new_val = (existing & 0xF000_0000) | (value & 0x0FFF_FFFF);
            let bytes = new_val.to_le_bytes();
            sector[offset_in_sector..offset_in_sector + 4].copy_from_slice(&bytes);

            Self::write_sector_raw(sector_lba, &sector)?;
        }
        Ok(())
    }

    /// Find a free cluster in the FAT.
    fn fat_alloc(bpb: &Bpb) -> FsResult<u32> {
        // Total data clusters
        let total_clusters = (bpb.total_sectors - bpb.data_start) / bpb.sectors_per_cluster as u32;
        for cluster in 2..total_clusters + 2 {
            let val = Self::fat_read(bpb, cluster)?;
            if val == FAT_FREE {
                return Ok(cluster);
            }
        }
        Err(FsError::NoSpace)
    }

    /// Allocate a new cluster, mark as EOC, optionally chain from `prev`.
    fn alloc_cluster(bpb: &Bpb, prev: Option<u32>) -> FsResult<u32> {
        let new = Self::fat_alloc(bpb)?;
        Self::fat_write(bpb, new, 0x0FFF_FFFF)?; // mark as end-of-chain
        if let Some(p) = prev {
            Self::fat_write(bpb, p, new)?; // link previous to new
        }
        // Zero the cluster
        let start_sector = bpb.cluster_to_sector(new);
        let zero = [0u8; 512];
        for s in 0..bpb.sectors_per_cluster as u32 {
            Self::write_sector_raw(start_sector + s, &zero)?;
        }
        Ok(new)
    }

    // ── Cluster chain reading ───────────────────────────────

    /// Read all data from a cluster chain into a Vec.
    fn read_chain(bpb: &Bpb, start_cluster: u32) -> FsResult<Vec<u8>> {
        let mut data = Vec::new();
        let cluster_bytes = bpb.sectors_per_cluster as usize * SECTOR_SIZE;
        let mut cluster = start_cluster;

        loop {
            if cluster < 2 { break; }
            let sector = bpb.cluster_to_sector(cluster);
            for s in 0..bpb.sectors_per_cluster as u32 {
                let buf = Self::read_sector_raw(sector + s)?;
                data.extend_from_slice(&buf);
            }
            let next = Self::fat_read(bpb, cluster)?;
            if next >= FAT_EOC { break; }
            cluster = next;
            // Safety: prevent infinite loops
            if data.len() > 16 * 1024 * 1024 { break; }
        }

        Ok(data)
    }

    /// Write data to a cluster chain, allocating new clusters as needed.
    fn write_chain(bpb: &Bpb, start_cluster: u32, data: &[u8]) -> FsResult<u32> {
        let cluster_bytes = bpb.sectors_per_cluster as usize * SECTOR_SIZE;
        let mut cluster = start_cluster;
        let mut offset = 0usize;

        loop {
            // Write data to current cluster
            let sector = bpb.cluster_to_sector(cluster);
            for s in 0..bpb.sectors_per_cluster as u32 {
                let mut buf = [0u8; 512];
                let start = offset;
                let end = (offset + SECTOR_SIZE).min(data.len());
                if start < data.len() {
                    let len = end - start;
                    buf[..len].copy_from_slice(&data[start..end]);
                }
                Self::write_sector_raw(sector + s, &buf)?;
                offset += SECTOR_SIZE;
            }

            if offset >= data.len() {
                // Mark this as end of chain
                Self::fat_write(bpb, cluster, 0x0FFF_FFFF)?;
                break;
            }

            // Need more clusters
            let next = Self::fat_read(bpb, cluster)?;
            if next >= FAT_EOC || next < 2 {
                // Allocate new cluster
                let new_cluster = Self::alloc_cluster(bpb, Some(cluster))?;
                cluster = new_cluster;
            } else {
                cluster = next;
            }
        }

        Ok(start_cluster)
    }

    // ── Directory operations ────────────────────────────────

    /// Read all directory entries from a directory cluster chain.
    fn read_dir_entries(bpb: &Bpb, dir_cluster: u32) -> FsResult<Vec<(RawDirEntry, u32, usize)>> {
        // Returns (entry, sector_lba, offset_in_sector) for each valid entry
        let mut entries = Vec::new();
        let mut cluster = dir_cluster;

        loop {
            if cluster < 2 { break; }
            let base_sector = bpb.cluster_to_sector(cluster);

            for s in 0..bpb.sectors_per_cluster as u32 {
                let sector_lba = base_sector + s;
                let sector = Self::read_sector_raw(sector_lba)?;

                for i in 0..ENTRIES_PER_SECTOR {
                    let off = i * DIR_ENTRY_SIZE;
                    let entry = RawDirEntry::from_bytes(&sector[off..off + DIR_ENTRY_SIZE]);

                    if entry.is_free() {
                        return Ok(entries); // no more entries
                    }
                    if entry.is_deleted() || entry.is_lfn() || entry.is_volume_id() {
                        continue;
                    }

                    entries.push((entry, sector_lba, off));
                }
            }

            let next = Self::fat_read(bpb, cluster)?;
            if next >= FAT_EOC { break; }
            cluster = next;
        }

        Ok(entries)
    }

    /// Resolve a path to the target directory entry.
    /// Returns (entry, parent_cluster).
    fn resolve_path_entry(bpb: &Bpb, path: &str) -> FsResult<(RawDirEntry, u32)> {
        let path = path.trim_start_matches('/');
        if path.is_empty() {
            // Root directory — synthesize entry
            let mut entry = RawDirEntry {
                name: [0x20; 11],
                attr: ATTR_DIRECTORY,
                cluster_hi: (bpb.root_cluster >> 16) as u16,
                cluster_lo: bpb.root_cluster as u16,
                file_size: 0,
            };
            entry.name[0] = b'/';
            return Ok((entry, 0));
        }

        let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
        let mut current_cluster = bpb.root_cluster;

        for (idx, segment) in segments.iter().enumerate() {
            let entries = Self::read_dir_entries(bpb, current_cluster)?;
            let target_name = encode_83_name(segment).ok_or(FsError::InvalidPath)?;

            let mut found = false;
            for (entry, _, _) in &entries {
                if entry.name == target_name {
                    if idx == segments.len() - 1 {
                        // Final segment — return this entry
                        return Ok((entry.clone(), current_cluster));
                    }
                    if !entry.is_dir() {
                        return Err(FsError::NotADirectory);
                    }
                    current_cluster = entry.first_cluster();
                    found = true;
                    break;
                }
            }
            if !found {
                return Err(FsError::NotFound);
            }
        }

        Err(FsError::NotFound)
    }

    /// Resolve a path to the parent directory cluster and child name.
    fn resolve_parent_and_name(bpb: &Bpb, path: &str) -> FsResult<(u32, String)> {
        let path = path.trim_start_matches('/');
        if path.is_empty() {
            return Err(FsError::InvalidPath);
        }

        let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
        if segments.is_empty() {
            return Err(FsError::InvalidPath);
        }

        let child_name = String::from(*segments.last().unwrap());
        let mut parent_cluster = bpb.root_cluster;

        // Navigate to parent directory
        for segment in &segments[..segments.len() - 1] {
            let entries = Self::read_dir_entries(bpb, parent_cluster)?;
            let target = encode_83_name(segment).ok_or(FsError::InvalidPath)?;
            let mut found = false;
            for (entry, _, _) in &entries {
                if entry.name == target && entry.is_dir() {
                    parent_cluster = entry.first_cluster();
                    found = true;
                    break;
                }
            }
            if !found {
                return Err(FsError::NotFound);
            }
        }

        Ok((parent_cluster, child_name))
    }

    /// Add a new entry to a directory.
    fn add_dir_entry(bpb: &Bpb, dir_cluster: u32, entry: &RawDirEntry) -> FsResult<()> {
        let mut cluster = dir_cluster;

        loop {
            if cluster < 2 { return Err(FsError::IoError); }
            let base_sector = bpb.cluster_to_sector(cluster);

            for s in 0..bpb.sectors_per_cluster as u32 {
                let sector_lba = base_sector + s;
                let mut sector = Self::read_sector_raw(sector_lba)?;

                for i in 0..ENTRIES_PER_SECTOR {
                    let off = i * DIR_ENTRY_SIZE;
                    if sector[off] == 0x00 || sector[off] == 0xE5 {
                        // Found a free slot
                        let bytes = entry.to_bytes();
                        sector[off..off + DIR_ENTRY_SIZE].copy_from_slice(&bytes);
                        Self::write_sector_raw(sector_lba, &sector)?;
                        return Ok(());
                    }
                }
            }

            let next = Self::fat_read(bpb, cluster)?;
            if next >= FAT_EOC || next < 2 {
                // Allocate new cluster for directory
                let new_cluster = Self::alloc_cluster(bpb, Some(cluster))?;
                cluster = new_cluster;
            } else {
                cluster = next;
            }
        }
    }

    /// Update an existing directory entry (find by name in parent cluster).
    fn update_dir_entry(bpb: &Bpb, parent_cluster: u32, name: &[u8; 11], new_entry: &RawDirEntry) -> FsResult<()> {
        let mut cluster = parent_cluster;

        loop {
            if cluster < 2 { return Err(FsError::NotFound); }
            let base_sector = bpb.cluster_to_sector(cluster);

            for s in 0..bpb.sectors_per_cluster as u32 {
                let sector_lba = base_sector + s;
                let mut sector = Self::read_sector_raw(sector_lba)?;

                for i in 0..ENTRIES_PER_SECTOR {
                    let off = i * DIR_ENTRY_SIZE;
                    if sector[off] == 0x00 { return Err(FsError::NotFound); }
                    if sector[off] == 0xE5 { continue; }

                    let entry = RawDirEntry::from_bytes(&sector[off..off + DIR_ENTRY_SIZE]);
                    if entry.name == *name {
                        let bytes = new_entry.to_bytes();
                        sector[off..off + DIR_ENTRY_SIZE].copy_from_slice(&bytes);
                        Self::write_sector_raw(sector_lba, &sector)?;
                        return Ok(());
                    }
                }
            }

            let next = Self::fat_read(bpb, cluster)?;
            if next >= FAT_EOC { break; }
            cluster = next;
        }

        Err(FsError::NotFound)
    }
}

// ══════════════════════════════════════════════════════════════
//  FileSystem trait implementation
// ══════════════════════════════════════════════════════════════

impl FileSystem for Fat32Fs {
    fn name(&self) -> &str {
        "fat32"
    }

    fn create(&self, path: &str) -> FsResult<Inode> {
        let inner = self.inner.lock();
        let bpb = &inner.bpb;

        let (parent_cluster, child_name) = Self::resolve_parent_and_name(bpb, path)?;
        let name83 = encode_83_name(&child_name).ok_or(FsError::InvalidPath)?;

        // Check for duplicates
        let entries = Self::read_dir_entries(bpb, parent_cluster)?;
        for (e, _, _) in &entries {
            if e.name == name83 {
                return Err(FsError::AlreadyExists);
            }
        }

        // Allocate a cluster for the file
        let cluster = Self::alloc_cluster(bpb, None)?;

        let entry = RawDirEntry {
            name: name83,
            attr: ATTR_ARCHIVE,
            cluster_hi: (cluster >> 16) as u16,
            cluster_lo: cluster as u16,
            file_size: 0,
        };

        Self::add_dir_entry(bpb, parent_cluster, &entry)?;

        Ok(Inode {
            id: cluster as u64,
            file_type: FileType::File,
            size: 0,
        })
    }

    fn mkdir(&self, path: &str) -> FsResult<Inode> {
        let inner = self.inner.lock();
        let bpb = &inner.bpb;

        let (parent_cluster, child_name) = Self::resolve_parent_and_name(bpb, path)?;
        let name83 = encode_83_name(&child_name).ok_or(FsError::InvalidPath)?;

        // Check duplicates
        let entries = Self::read_dir_entries(bpb, parent_cluster)?;
        for (e, _, _) in &entries {
            if e.name == name83 {
                return Err(FsError::AlreadyExists);
            }
        }

        // Allocate cluster for new directory
        let cluster = Self::alloc_cluster(bpb, None)?;

        // Create . and .. entries
        let dot_entry = RawDirEntry {
            name: {
                let mut n = [0x20u8; 11];
                n[0] = b'.';
                n
            },
            attr: ATTR_DIRECTORY,
            cluster_hi: (cluster >> 16) as u16,
            cluster_lo: cluster as u16,
            file_size: 0,
        };
        let dotdot_entry = RawDirEntry {
            name: {
                let mut n = [0x20u8; 11];
                n[0] = b'.';
                n[1] = b'.';
                n
            },
            attr: ATTR_DIRECTORY,
            cluster_hi: (parent_cluster >> 16) as u16,
            cluster_lo: parent_cluster as u16,
            file_size: 0,
        };

        Self::add_dir_entry(bpb, cluster, &dot_entry)?;
        Self::add_dir_entry(bpb, cluster, &dotdot_entry)?;

        // Add entry in parent
        let dir_entry = RawDirEntry {
            name: name83,
            attr: ATTR_DIRECTORY,
            cluster_hi: (cluster >> 16) as u16,
            cluster_lo: cluster as u16,
            file_size: 0,
        };
        Self::add_dir_entry(bpb, parent_cluster, &dir_entry)?;

        Ok(Inode {
            id: cluster as u64,
            file_type: FileType::Directory,
            size: 0,
        })
    }

    fn lookup(&self, path: &str) -> FsResult<Inode> {
        let inner = self.inner.lock();
        let bpb = &inner.bpb;

        let (entry, _) = Self::resolve_path_entry(bpb, path)?;
        let ft = if entry.is_dir() { FileType::Directory } else { FileType::File };

        Ok(Inode {
            id: entry.first_cluster() as u64,
            file_type: ft,
            size: entry.file_size as usize,
        })
    }

    fn read(&self, path: &str, offset: usize, buf: &mut [u8]) -> FsResult<usize> {
        let inner = self.inner.lock();
        let bpb = &inner.bpb;

        let (entry, _) = Self::resolve_path_entry(bpb, path)?;
        if entry.is_dir() {
            return Err(FsError::IsADirectory);
        }

        let file_size = entry.file_size as usize;
        if offset >= file_size {
            return Ok(0);
        }

        let data = Self::read_chain(bpb, entry.first_cluster())?;
        let available = &data[offset..file_size.min(data.len())];
        let to_read = buf.len().min(available.len());
        buf[..to_read].copy_from_slice(&available[..to_read]);

        Ok(to_read)
    }

    fn write(&self, path: &str, offset: usize, data: &[u8]) -> FsResult<usize> {
        let inner = self.inner.lock();
        let bpb = &inner.bpb;

        let (entry, parent_cluster) = Self::resolve_path_entry(bpb, path)?;
        if entry.is_dir() {
            return Err(FsError::IsADirectory);
        }

        // Read existing data
        let cluster = entry.first_cluster();
        let mut file_data = if entry.file_size > 0 {
            let d = Self::read_chain(bpb, cluster)?;
            d[..entry.file_size as usize].to_vec()
        } else {
            Vec::new()
        };

        // Expand if needed
        let end = offset + data.len();
        if end > file_data.len() {
            file_data.resize(end, 0);
        }
        file_data[offset..end].copy_from_slice(data);

        // Write back
        Self::write_chain(bpb, cluster, &file_data)?;

        // Update directory entry with new size
        let mut updated = entry.clone();
        updated.file_size = file_data.len() as u32;
        Self::update_dir_entry(bpb, parent_cluster, &entry.name, &updated)?;

        Ok(data.len())
    }

    fn readdir(&self, path: &str) -> FsResult<Vec<VfsDirEntry>> {
        let inner = self.inner.lock();
        let bpb = &inner.bpb;

        let dir_cluster = if path.trim_start_matches('/').is_empty() {
            bpb.root_cluster
        } else {
            let (entry, _) = Self::resolve_path_entry(bpb, path)?;
            if !entry.is_dir() {
                return Err(FsError::NotADirectory);
            }
            entry.first_cluster()
        };

        let entries = Self::read_dir_entries(bpb, dir_cluster)?;
        let mut result = Vec::new();

        for (e, _, _) in &entries {
            let name = e.display_name();
            // Skip . and ..
            if name == "." || name == ".." {
                continue;
            }
            let ft = if e.is_dir() { FileType::Directory } else { FileType::File };
            result.push(VfsDirEntry {
                name: name.to_lowercase(),
                inode: Inode {
                    id: e.first_cluster() as u64,
                    file_type: ft,
                    size: e.file_size as usize,
                },
            });
        }

        Ok(result)
    }

    fn unlink(&self, path: &str) -> FsResult<()> {
        let inner = self.inner.lock();
        let bpb = &inner.bpb;

        let (entry, parent_cluster) = Self::resolve_path_entry(bpb, path)?;

        // Don't delete non-empty directories
        if entry.is_dir() {
            let children = Self::read_dir_entries(bpb, entry.first_cluster())?;
            let real_children: Vec<_> = children.iter()
                .filter(|(e, _, _)| {
                    let n = e.display_name();
                    n != "." && n != ".."
                })
                .collect();
            if !real_children.is_empty() {
                return Err(FsError::IsADirectory);
            }
        }

        // Mark directory entry as deleted
        let mut cluster = parent_cluster;
        let name83 = entry.name;

        'outer: loop {
            if cluster < 2 { break; }
            let base_sector = bpb.cluster_to_sector(cluster);

            for s in 0..bpb.sectors_per_cluster as u32 {
                let sector_lba = base_sector + s;
                let mut sector = Self::read_sector_raw(sector_lba)?;

                for i in 0..ENTRIES_PER_SECTOR {
                    let off = i * DIR_ENTRY_SIZE;
                    if sector[off] == 0x00 { break 'outer; }
                    if sector[off] == 0xE5 { continue; }

                    let e = RawDirEntry::from_bytes(&sector[off..off + DIR_ENTRY_SIZE]);
                    if e.name == name83 {
                        sector[off] = 0xE5; // mark as deleted
                        Self::write_sector_raw(sector_lba, &sector)?;

                        // Free the cluster chain
                        let mut c = entry.first_cluster();
                        while c >= 2 && c < FAT_EOC {
                            let next = Self::fat_read(bpb, c)?;
                            Self::fat_write(bpb, c, FAT_FREE)?;
                            if next >= FAT_EOC { break; }
                            c = next;
                        }

                        return Ok(());
                    }
                }
            }

            let next = Self::fat_read(bpb, cluster)?;
            if next >= FAT_EOC { break; }
            cluster = next;
        }

        Err(FsError::NotFound)
    }
}
