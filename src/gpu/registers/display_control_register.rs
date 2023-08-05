bitflags! {
  #[derive(Copy, Clone)]
  pub struct DisplayControlRegister: u16 {
    const DISPLAY_FRAME_SELECT = 0b1 << 4;
    const HBLANK_INTERVAL_FREE = 0b1 << 5;
    const OBJ_CHARACTER_MAPPING = 0b1 << 6;
    const FORCED_BLANK = 0b1 << 7;
    const DISPLAY_BG0 = 0b1 << 8;
    const DISPLAY_BG1 = 0b1 << 9;
    const DISPLAY_BG2 = 0b1 << 10;
    const DISPLAY_BG3 = 0b1 << 11;
    const DISPLAY_OBJ = 0b1 << 12;
    const DISPLAY_WINDOW_0 = 0b1 << 13;
    const DISPLAY_WINDOW_1 = 0b1 << 14;
    const DISPLAY_OBJ_WINDOW = 0b1 << 15;
  }
}

impl DisplayControlRegister {
  pub fn bg_mode(&self) -> u16 {
    self.bits() & 0b111
  }
}