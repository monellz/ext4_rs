use crate::dir::Dir;
use crate::file::File;
use crate::fs::FileSystem;
use crate::io::{Read, ReadLeExt, ReadWriteSeek, Seek, Write, WriteLeExt};
use crate::utils::crc::crc32c;
use bitflags::bitflags;

bitflags! {
  #[derive(Debug, Copy, Clone, PartialEq, Eq)]
  pub struct DirEntryFileType: u8 {
    const UNKNOWN = 0;
    const REG_FILE = 1;
    const DIR = 2;
    const CHRDEV = 3;
    const BLKDEV = 4;
    const FIFO = 5;
    const SOCK = 6;
    const SYMLINK = 7;
  }
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct DirEntry1 {
  pub inode: u32,
  pub rec_len: u16,
  pub name_len: u16,
  pub name: [u8; 255],
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct DirEntry2 {
  pub inode: u32,
  pub rec_len: u16,
  pub name_len: u8,
  pub file_type: u8,
  pub name: [u8; 255],
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct DirEntryTail {
  pub reserved_zero1: u32, // 0
  pub rec_len: u16,        // 12
  pub reserved_zero2: u8,  // 0
  pub reserved_ft: u8,     // 0xDE
  pub checksum: u32,
}

#[derive(Debug, Copy, Clone)]
pub enum DirEntryData {
  DirEntry1(DirEntry1),
  DirEntry2(DirEntry2),
  DirEntryTail(DirEntryTail),
}

impl DirEntryData {
  pub fn deserialize<R: Read + Seek>(
    reader: &mut R,
    feature_incompat_filetype: bool,
    max_size: usize,
  ) -> Result<Self, R::Error> {
    assert!(max_size >= 4 + 2);
    let inode = reader.read_u32_le()?;
    let rec_len = reader.read_u16_le()?;
    assert!(max_size >= rec_len as usize);

    // dir entry tail
    if inode == 0 && rec_len == 12 {
      let reserved_zero2 = reader.read_u8()?;
      let reserved_ft = reader.read_u8()?;
      // TODO: use error
      assert_eq!(reserved_zero2, 0);
      assert_eq!(reserved_ft, 0xDE);
      let checksum = reader.read_u32_le()?;
      let dir_entry_tail = DirEntryTail {
        reserved_zero1: 0,
        rec_len: 12,
        reserved_zero2,
        reserved_ft,
        checksum,
      };
      return Ok(DirEntryData::DirEntryTail(dir_entry_tail));
    }

    // TODO: avoid redundant copy?
    let entry = if feature_incompat_filetype {
      let name_len = reader.read_u8()?;
      let file_type = reader.read_u8()?;
      let mut name = [0u8; 255];
      reader.read_exact(&mut name[0..name_len as usize])?;
      let dir_entry = DirEntry2 {
        inode,
        rec_len,
        name_len,
        file_type,
        name,
      };
      DirEntryData::DirEntry2(dir_entry)
    } else {
      let name_len = reader.read_u16_le()?;
      let mut name = [0u8; 255];
      reader.read_exact(&mut name[0..name_len as usize])?;
      let dir_entry = DirEntry1 {
        inode,
        rec_len,
        name_len,
        name,
      };
      DirEntryData::DirEntry1(dir_entry)
    };

    Ok(entry)
  }
  pub fn serialize<W: Write>(&self, writer: &mut W) -> Result<(), W::Error> {
    match self {
      DirEntryData::DirEntry1(entry) => {
        writer.write_u32_le(entry.inode)?;
        writer.write_u16_le(entry.rec_len)?;
        writer.write_u16_le(entry.name_len)?;
        writer.write_all(&entry.name[0..entry.name_len as usize])?;
        let padding = entry.rec_len - entry.name_len as u16 - 8;
        writer.write_all(&vec![0u8; padding as usize])?;
      }
      DirEntryData::DirEntry2(entry) => {
        writer.write_u32_le(entry.inode)?;
        writer.write_u16_le(entry.rec_len)?;
        writer.write_u8(entry.name_len)?;
        writer.write_u8(entry.file_type)?;
        writer.write_all(&entry.name[0..entry.name_len as usize])?;
        let padding = entry.rec_len - entry.name_len as u16 - 8;
        writer.write_all(&vec![0u8; padding as usize])?;
      }
      DirEntryData::DirEntryTail(entry) => {
        assert!(entry.rec_len == 12);
        writer.write_u32_le(entry.reserved_zero1)?;
        writer.write_u16_le(entry.rec_len)?;
        writer.write_u8(entry.reserved_zero2)?;
        writer.write_u8(entry.reserved_ft)?;
        writer.write_u32_le(entry.checksum)?;
      }
    }
    Ok(())
  }

  pub fn new(ino: u32, name: &str, file_type: Option<DirEntryFileType>, feature_incompat_filetype: bool) -> Self {
    if feature_incompat_filetype {
      let name_len = name.len();
      let rec_len = (8 + name_len + 3) / 4 * 4;
      let mut name_bytes = [0u8; 255];
      name_bytes[..name_len].copy_from_slice(name.as_bytes());
      let entry = DirEntry2 {
        inode: ino,
        rec_len: rec_len as u16,
        name_len: name_len as u8,
        file_type: file_type.unwrap_or(DirEntryFileType::UNKNOWN).bits(),
        name: name_bytes,
      };
      DirEntryData::DirEntry2(entry)
    } else {
      let name_len = name.len();
      let rec_len = (8 + name_len + 3) / 4 * 4;
      let mut name_bytes = [0u8; 255];
      name_bytes[..name_len].copy_from_slice(name.as_bytes());
      let entry = DirEntry1 {
        inode: ino,
        rec_len: rec_len as u16,
        name_len: name_len as u16,
        name: name_bytes,
      };
      DirEntryData::DirEntry1(entry)
    }
  }

  // 获得新创建的目录下的三个entry(., .., tail)
  pub fn new_dir_entries(
    ino: u32,
    parent_ino: u32,
    feature_incompat_filetype: bool,
    block_size: u64,
    uuid: &[u8],
    ino_gen: u32,
  ) -> [DirEntryData; 3] {
    let mut self_name = [0u8; 255];
    let mut parent_name = [0u8; 255];
    self_name[0] = b'.';
    parent_name[0] = b'.';
    parent_name[1] = b'.';

    if feature_incompat_filetype {
      let self_entry = DirEntry2 {
        inode: ino,
        rec_len: 12,
        name_len: 1,
        file_type: DirEntryFileType::DIR.bits(),
        name: self_name,
      };
      let parent_entry = DirEntry2 {
        inode: parent_ino,
        rec_len: block_size as u16 - 12 - 12,
        name_len: 2,
        file_type: DirEntryFileType::DIR.bits(),
        name: parent_name,
      };
      let mut tail_entry = DirEntryTail {
        reserved_zero1: 0,
        rec_len: 12,
        reserved_zero2: 0,
        reserved_ft: 0xDE,
        checksum: 0,
      };

      // checksum
      let mut block = vec![0u8; self_entry.rec_len as usize + parent_entry.rec_len as usize];
      unsafe {
        let self_entry_ptr = &self_entry as *const DirEntry2 as *const u8;
        let parent_entry_ptr = &parent_entry as *const DirEntry2 as *const u8;
        block[0..self_entry.rec_len as usize]
          .copy_from_slice(core::slice::from_raw_parts(self_entry_ptr, self_entry.rec_len as usize));
        block[self_entry.rec_len as usize..].copy_from_slice(core::slice::from_raw_parts(
          parent_entry_ptr,
          parent_entry.rec_len as usize,
        ));
      }
      let mut csum = crc32c(!0, uuid, uuid.len() as u32);
      csum = crc32c(csum, &ino.to_le_bytes(), 4);
      csum = crc32c(csum, &ino_gen.to_le_bytes(), 4);
      csum = crc32c(csum, &block, block.len() as u32);
      tail_entry.checksum = csum;

      [
        DirEntryData::DirEntry2(self_entry),
        DirEntryData::DirEntry2(parent_entry),
        DirEntryData::DirEntryTail(tail_entry),
      ]
    } else {
      unimplemented!();
    }
  }

  pub fn get_inode(&self) -> u32 {
    match self {
      DirEntryData::DirEntry1(entry) => entry.inode,
      DirEntryData::DirEntry2(entry) => entry.inode,
      DirEntryData::DirEntryTail(_) => 0,
    }
  }

  pub fn get_rec_len(&self) -> u16 {
    match self {
      DirEntryData::DirEntry1(entry) => entry.rec_len,
      DirEntryData::DirEntry2(entry) => entry.rec_len,
      DirEntryData::DirEntryTail(entry) => entry.rec_len,
    }
  }

  pub fn set_rec_len(&mut self, rec_len: u16) {
    match self {
      DirEntryData::DirEntry1(entry) => entry.rec_len = rec_len,
      DirEntryData::DirEntry2(entry) => entry.rec_len = rec_len,
      DirEntryData::DirEntryTail(entry) => entry.rec_len = rec_len,
    }
  }

  pub fn get_name_str(&self) -> String {
    let name = match self {
      DirEntryData::DirEntry1(entry) => &entry.name[0..entry.name_len as usize],
      DirEntryData::DirEntry2(entry) => &entry.name[0..entry.name_len as usize],
      DirEntryData::DirEntryTail(_) => &[],
    };
    String::from_utf8_lossy(name).to_string()
  }

  pub fn get_real_rec_len(&self) -> u16 {
    match self {
      DirEntryData::DirEntry1(entry) => (entry.name_len as u16 + 8 + 3) / 4 * 4,
      DirEntryData::DirEntry2(entry) => (entry.name_len as u16 + 8 + 3) / 4 * 4,
      DirEntryData::DirEntryTail(entry) => entry.rec_len,
    }
  }
}

#[derive(Clone)]
pub struct DirEntry<'a, IO: ReadWriteSeek> {
  pub data: DirEntryData,
  pub fs: &'a FileSystem<IO>,
}

impl<'a, IO: ReadWriteSeek> DirEntry<'a, IO> {
  pub fn to_dir(&self) -> Dir<'a, IO> {
    let ino = self.data.get_inode();
    let inode = self.fs.get_inode(ino as u64).unwrap();
    assert!(inode.is_dir(), "only support dir");
    Dir::new(ino as u64, inode, self.fs)
  }

  pub fn to_file(&self) -> File<'a, IO> {
    let ino = self.data.get_inode();
    let inode = self.fs.get_inode(ino as u64).unwrap();
    assert!(inode.is_file(), "only support file");
    File::new(ino as u64, inode, self.fs)
  }
}
