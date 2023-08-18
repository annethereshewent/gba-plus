bitflags! {
  pub struct SweepControlTone: u16 {
    const SWEEP_FREQUENCY_DIRECTION = 0b1 << 3;
  }
}

impl SweepControlTone {
  pub fn sweep_shift(&self) -> u16 {
    self.bits() & 0b111
  }

  pub fn sweep_time(&self) -> u16 {
    (self.bits() >> 4) & 0b111
  }
}

