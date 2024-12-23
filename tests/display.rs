use std::fs;

use ext4fs::io::StdIoWrapper;
use fscommon::BufStream;

const EXT4_1M_IMG: &str = "imgs/ext4_1m.img";
const EXT4_32M_IMG: &str = "imgs/ext4_32m.img";

type FileSystem = ext4fs::fs::FileSystem<StdIoWrapper<BufStream<fs::File>>>;

fn call_with_fs<F: Fn(FileSystem)>(f: F, filename: &str) {
  let _ = env_logger::builder().is_test(true).try_init();
  let file = fs::File::open(filename).unwrap();
  let buf_file = BufStream::new(file);
  let fs = FileSystem::new(buf_file).unwrap();
  f(fs);
}

fn display_metadata(fs: FileSystem) {
  println!("{:?}", fs.super_block);
  println!(
    "bgd count: {}(vec len: {})",
    fs.super_block.get_block_group_count(),
    fs.block_group_descriptors.len()
  );

  println!("{:?}", fs.super_block.get_feature_compat());
  println!("{:?}", fs.super_block.get_feature_incompat());
  println!("{:?}", fs.super_block.get_feature_ro_compat());
}

fn display_inode_of_root_dir(mut fs: FileSystem) {
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
fn inode_of_root_dir_32m() {
  call_with_fs(display_inode_of_root_dir, EXT4_32M_IMG)
}
