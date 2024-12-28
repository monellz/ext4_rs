use std::fs;

use ext4fs::inode::InodeFilePerm;
use ext4fs::io::StdIoWrapper;
use fscommon::BufStream;
use std::time::{SystemTime, UNIX_EPOCH};

const EXT4_1M_IMG: &str = "imgs/ext4_1m.img";
const EXT4_32M_IMG: &str = "imgs/ext4_32m.img";

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
  let root_dir = fs.root_dir();
  println!("{:?}", root_dir.inode);
  println!("{:?}", root_dir.inode.get_file_type());
  println!("{:?}", root_dir.inode.get_file_perm());
  println!("{:?}", root_dir.inode.get_flags());
}

#[test]
fn metadata_1m() {
  call_with_fs(display_metadata, EXT4_1M_IMG)
}

#[test]
fn metadata_32m() {
  call_with_fs(display_metadata, EXT4_32M_IMG)
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
    },
    EXT4_1M_IMG,
  )
}

#[test]
fn inode_of_root_dir_32m() {
  call_with_fs(display_inode_of_root_dir, EXT4_32M_IMG)
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
      let file_perm = InodeFilePerm::default();
      let time = get_current_time();
      let new_dir = root_dir
        .create_dir("new_dir", 0, 0, file_perm, time, time, time)
        .unwrap();
      println!("{:?} num: {}", new_dir.inode, new_dir.ino);
      for entry in root_dir.iter() {
        let entry = entry.unwrap();
        let name = entry.data.get_name_str();
        println!("{:?} name: {}", entry.data, name);
      }
    },
    EXT4_1M_IMG,
  )
}
