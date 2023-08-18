pub struct SoundControlEnable {
  pub value: u16,
  pub sound_master_volume_right: u16,
  pub sound_master_volume_left: u16,
  pub sound_master_enable_left: u16,
  pub sound_master_enable_right: u16
}

impl SoundControlEnable {
  pub fn new() -> Self {
    Self {
      value: 0,
      sound_master_enable_left: 0,
      sound_master_enable_right: 0,
      sound_master_volume_left: 0,
      sound_master_volume_right: 0
    }
  }

  pub fn write(&mut self, val: u16) {
    self.value = val;

    self.sound_master_volume_right = val & 0b111;
    self.sound_master_volume_left = (val >> 4) & 0b111;
    self.sound_master_enable_right = (val >> 8) & 0b1111;
    self.sound_master_enable_left = (val >> 12) & 0b1111;
  }
}