bitflags! {
  pub struct SweepControlFrequency: u16 {
    const LENGTH_FLAG = 0b1 << 14;
    const RESTART = 0b1 << 15;
  }
}

impl SweepControlFrequency {
  pub fn frequency(&self) -> u16 {
    self.bits() & 0b11111111111
  }
}