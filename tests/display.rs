use std::fs;

use ext4fs::dir::Dir;
use ext4fs::dir_entry::DirEntryData;
use ext4fs::inode::{Inode, InodeFilePerm};
use ext4fs::io::{ReadWriteSeek, StdIoWrapper};
use fscommon::BufStream;
use std::time::{SystemTime, UNIX_EPOCH};

const EXT4_1M_IMG: &str = "imgs/ext4_1m.img";

type FileSystem = ext4fs::fs::FileSystem<StdIoWrapper<BufStream<fs::File>>>;

fn call_with_fs<F: Fn(FileSystem)>(f: F, filename: &str) {
  let _ = env_logger::builder().is_test(true).try_init();
  let file = fs::OpenOptions::new().read(true).write(true).open(filename).unwrap();
  let buf_file = BufStream::new(file);
  let fs = FileSystem::new(buf_file).unwrap();
  f(fs);
}

fn get_current_time() -> u32 {
  let now = SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .expect("Time went backwards");
  now.as_secs() as u32
}

fn display_metadata(fs: FileSystem) {
  println!("{:?}", fs.super_block);
  println!(
    "bgd count: {}(vec len: {})",
    fs.super_block.borrow().get_block_group_count(),
    fs.block_group_descriptors.borrow().len()
  );

  println!("{:?}", fs.super_block.borrow().get_feature_compat());
  println!("{:?}", fs.super_block.borrow().get_feature_incompat());
  println!("{:?}", fs.super_block.borrow().get_feature_ro_compat());

  println!("{:?}", fs.block_group_descriptors.borrow()[0]);

  for i in 0..fs.super_block.borrow().get_block_group_count() {
    let bgd = &fs.block_group_descriptors.borrow()[i as usize];
    println!("{}: {:?}", i, bgd.get_flags());
  }
}

fn display_inode_of_root_dir(fs: FileSystem) {
  let mut root_dir = fs.root_dir();
  println!("{:?}", root_dir.inode);
  println!("{:?}", root_dir.inode.get_file_type());
  println!("{:?}", root_dir.inode.get_file_perm());
  println!("{:?}", root_dir.inode.get_flags());

  let mut disk = fs.disk.borrow_mut();
  let extents = root_dir.inode.get_extents(&mut *disk).unwrap();
  println!("{:?}", extents);

  let csum = root_dir.inode.get_checksum();
  let cmp_csum = root_dir.inode.compute_checksum(
    root_dir.ino as u32,
    fs.super_block.borrow().get_inode_size() as u16,
    &fs.super_block.borrow().uuid,
  );
  assert_eq!(csum, cmp_csum);
}

fn check_inode_checksum(ino: u64, fs: &FileSystem) {
  let mut inode = fs.get_inode(ino).unwrap();
  let csum = inode.get_checksum();
  let cmp_csum = inode.compute_checksum(
    ino as u32,
    fs.super_block.borrow().get_inode_size() as u16,
    &fs.super_block.borrow().uuid,
  );
  assert_eq!(csum, cmp_csum);
}

fn check_inode_checksum_of_root_dir(fs: FileSystem) {
  check_inode_checksum(Inode::ROOT_INO, &fs);
}

fn check_dirblock_checksum<IO: ReadWriteSeek>(dir: &Dir<IO>) {
  let extents = {
    let mut disk = dir.fs.disk.borrow_mut();
    dir.inode.get_extents(&mut *disk).unwrap()
  };
  assert_eq!(extents.len(), 1);
  let (entries, tail_entry) = {
    let mut entries = Vec::new();
    let mut iter = dir.iter();
    for entry in &mut iter {
      entries.push(entry.unwrap().data);
    }
    let tail_entry = iter.tail_entry.unwrap();
    (entries, tail_entry)
  };
  let csum = tail_entry.get_checksum();
  let cmp_csum = DirEntryData::compute_dirblock_checksum(
    &entries,
    dir.fs.super_block.borrow().get_block_size(),
    &dir.fs.super_block.borrow().uuid,
    dir.ino as u32,
    dir.inode.generation,
  );
  assert_eq!(csum, cmp_csum);
}

fn check_dirblock_checksum_of_root_dir(fs: FileSystem) {
  let root_dir = fs.root_dir();
  check_dirblock_checksum(&root_dir);
}

#[test]
fn check_dirblock_checksum_of_root_dir_1m() {
  call_with_fs(check_dirblock_checksum_of_root_dir, EXT4_1M_IMG)
}

#[test]
fn check_inode_checksum_of_root_dir_1m() {
  call_with_fs(check_inode_checksum_of_root_dir, EXT4_1M_IMG)
}

#[test]
fn metadata_1m() {
  call_with_fs(display_metadata, EXT4_1M_IMG)
}

#[test]
fn inode_of_root_dir_1m() {
  call_with_fs(display_inode_of_root_dir, EXT4_1M_IMG)
}

#[test]
fn inode_of_dir3_1m() {
  call_with_fs(
    |fs| {
      let root_dir = fs.root_dir();
      let entry = root_dir.find_entry("dir3").unwrap();
      assert_eq!(entry.data.get_name_str(), "dir3");
      let ino = entry.data.get_inode();
      let inode = fs.get_inode(ino as u64).unwrap();
      println!("{:?}", inode);

      let mut disk = fs.disk.borrow_mut();
      let extent = inode.get_extents(&mut *disk).unwrap();
      println!("{:?}", extent);
    },
    EXT4_1M_IMG,
  )
}

#[test]
fn read_root_dir() {
  call_with_fs(
    |fs| {
      let root_dir = fs.root_dir();
      println!("{:?}", root_dir.inode);
      for entry in root_dir.iter() {
        let entry = entry.unwrap();
        let name = entry.data.get_name_str();
        println!("{:?} name: {}", entry.data, name);
      }
    },
    EXT4_1M_IMG,
  )
}

#[test]
fn read_file() {
  call_with_fs(
    |fs| {
      let root_dir = fs.root_dir();
      let entry = root_dir.find_entry("test0").unwrap();
      assert_eq!(entry.data.get_name_str(), "test0");
      println!("{:?}", entry.data);

      let file = entry.to_file();
      let mut buf = vec![0u8; 1024];
      let read_bytes = file.read(0, &mut buf).unwrap();
      println!("read_bytes: {}", read_bytes);
      println!("file size: {}", file.inode.get_size());
      println!("{:?}", &buf[..read_bytes]);
      let str = std::str::from_utf8(&buf[..read_bytes]).unwrap();
      println!("{}", str);
    },
    EXT4_1M_IMG,
  )
}

#[test]
fn create_dir() {
  call_with_fs(
    |fs| {
      let mut root_dir = fs.root_dir();
      let file_perm = InodeFilePerm::default_dir_perm();
      let time = get_current_time();
      let new_dir = root_dir
        .create_dir("created_dir_in_test", 0, 0, file_perm, time)
        .unwrap();
      check_dirblock_checksum(&new_dir);
      println!("{:?} num: {}", new_dir.inode, new_dir.ino);
      println!("{:?}", new_dir.inode.get_file_type());
      println!("{:?}", new_dir.inode.get_file_perm());
      println!("{:?}", new_dir.inode.get_flags());
      for entry in root_dir.iter() {
        let entry = entry.unwrap();
        let name = entry.data.get_name_str();
        println!("{:?} name: {}", entry.data, name);
      }
    },
    EXT4_1M_IMG,
  )
}

#[test]
fn create_file() {
  call_with_fs(
    |fs| {
      let mut root_dir = fs.root_dir();
      let file_perm = InodeFilePerm::default_file_perm();
      let time = get_current_time();
      let new_file = root_dir
        .create_file("created_file_in_test", 0, 0, file_perm, time)
        .unwrap();
      println!("{:?} num: {}", new_file.inode, new_file.ino);
      println!("{:?}", new_file.inode.get_file_type());
      println!("{:?}", new_file.inode.get_file_perm());
      println!("{:?}", new_file.inode.get_flags());
      for entry in root_dir.iter() {
        let entry = entry.unwrap();
        let name = entry.data.get_name_str();
        println!("{:?} name: {}", entry.data, name);
      }
    },
    EXT4_1M_IMG,
  )
}
