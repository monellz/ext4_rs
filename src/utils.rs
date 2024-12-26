#[inline]
pub fn combine_u64(lo: u32, hi: u32) -> u64 {
  ((hi as u64) << 32) | (lo as u64)
}

pub fn split_path(path: &str) -> (&str, Option<&str>) {
  let trimmed_path = path.trim_matches('/');
  trimmed_path.find('/').map_or((trimmed_path, None), |n| {
    (&trimmed_path[..n], Some(&trimmed_path[n + 1..]))
  })
}
