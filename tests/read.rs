use log::*;
use std::fs;

use ext4fs::io::StdIoWrapper;
use fscommon::BufStream;

const EXT41M_IMG: &str = "imgs/ext4_1m.img";

type FileSystem = ext4fs::fs::FileSystem<StdIoWrapper<BufStream<fs::File>>>;

fn call_with_fs<F: Fn(FileSystem)>(f: F, filename: &str) {
  let _ = env_logger::builder().is_test(true).try_init();
  let file = fs::File::open(filename).unwrap();
  let buf_file = BufStream::new(file);
  let fs = FileSystem::new(buf_file).unwrap();
  f(fs);
}

fn test_root_dir(fs: FileSystem) {
  error!("{:?}", fs.super_block);
  // let root_dir = fs.root_dir();
  // let entries = root_dir.iter().map(|r| r.unwrap()).collect::<Vec<_>>();
  // let short_names = entries.iter().map(|e| e.short_file_name()).collect::<Vec<String>>();
  // assert_eq!(short_names, ["LONG.TXT", "SHORT.TXT", "VERY", "VERY-L~1"]);
  // let names = entries.iter().map(|e| e.file_name()).collect::<Vec<String>>();
  // assert_eq!(names, ["long.txt", "short.txt", "very", "very-long-dir-name"]);
  // // Try read again
  // let names2 = root_dir.iter().map(|r| r.unwrap().file_name()).collect::<Vec<String>>();
  // assert_eq!(names2, names);
}

#[test]
fn test_root_dir_1m() {
  call_with_fs(test_root_dir, EXT41M_IMG)
}
