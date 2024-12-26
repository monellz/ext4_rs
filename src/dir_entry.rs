use crate::dir::Dir;
use crate::file::File;
use crate::fs::FileSystem;
use crate::io::{Read, ReadLeExt, ReadWriteSeek, Seek};
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
  inode: u32,
  pub rec_len: u16,
  name_len: u16,
  name: [u8; 255],
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct DirEntry2 {
  inode: u32,
  pub rec_len: u16,
  name_len: u8,
  file_type: u8,
  name: [u8; 255],
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct DirEntryTail {
  reserved_zero1: u32, // 0
  pub rec_len: u16,    // 12
  reserved_zero2: u8,  // 0
  reserved_ft: u8,     // 0xDE
  checksum: u32,
}

#[derive(Debug)]
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

  pub fn get_name_str(&self) -> String {
    let name = match self {
      DirEntryData::DirEntry1(entry) => &entry.name[0..entry.name_len as usize],
      DirEntryData::DirEntry2(entry) => &entry.name[0..entry.name_len as usize],
      DirEntryData::DirEntryTail(_) => &[],
    };
    String::from_utf8_lossy(name).to_string()
  }
}

pub struct DirEntry<'a, IO: ReadWriteSeek> {
  pub data: DirEntryData,
  pub fs: &'a FileSystem<IO>,
}

impl<'a, IO: ReadWriteSeek> DirEntry<'a, IO> {
  pub fn to_dir(&self) -> Dir<'a, IO> {
    let ino = self.data.get_inode();
    let inode = self.fs.get_inode(ino as u64).unwrap();
    assert!(inode.is_dir(), "only support dir");
    Dir::new(inode, self.fs)
  }

  pub fn to_file(&self) -> File<'a, IO> {
    let ino = self.data.get_inode();
    let inode = self.fs.get_inode(ino as u64).unwrap();
    assert!(inode.is_file(), "only support file");
    File::new(inode, self.fs)
  }
}
