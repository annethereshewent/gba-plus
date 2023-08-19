bitflags! {
  pub struct SoundControlEnvelope: u16 {
    const ENVELOPE_DIRECTION = 0b1 << 11;
  }
}

impl SoundControlEnvelope {
  pub fn sound_length(&self) -> u16 {
    self.bits() & 0b111111
  }

  pub fn wave_pattern_duty(&self) -> u16 {
    (self.bits() >> 6) & 0b11
  }

  pub fn envelope_step_time(&self) -> u16 {
    (self.bits() >> 8) & 0b111
  }

  pub fn initial_volume(&self) -> u16 {
    (self.bits() >> 12) & 0b1111
  }
}