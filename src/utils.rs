#[inline]
pub fn combine_u64(lo: u32, hi: u32) -> u64 {
  ((hi as u64) << 32) | (lo as u64)
}
