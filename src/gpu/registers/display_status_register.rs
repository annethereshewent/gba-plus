bitflags! {
  #[derive(Copy, Clone)]
  pub struct DisplayStatusRegister: u16 {
    const VBLANK = 0b1;
    const HBLANK = 0b1 << 1;
    const VCOUNTER = 0b1 << 2;
    const VBLANK_ENABLE = 0b1 << 3;
    const HBLANK_ENABLE = 0b1 << 4;
    const VCOUNTER_ENABLE = 0b1 << 5;
  }
}

impl DisplayStatusRegister {
  pub fn vcount_setting(&self) -> u16 {
    (self.bits() >> 8) & 0xff
  }
}