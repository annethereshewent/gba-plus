const LUT_SIZE: usize = 0x100;

const BOARD_RAM_PAGE: usize = 0x2;
const OAM_RAM_PAGE: usize = 0x7;
const VRAM_PAGE: usize = 0x6;
const PALRAM_PAGE: usize = 0x5;

pub struct CycleLookupTables {
  pub n_cycles_32: [u32; LUT_SIZE],
  pub s_cycles_32: [u32; LUT_SIZE],
  pub n_cycles_16: [u32; LUT_SIZE],
  pub s_cycles_16: [u32; LUT_SIZE]
}

impl CycleLookupTables {
  pub fn new() -> Self {
    Self {
      n_cycles_32: [1; LUT_SIZE],
      s_cycles_32: [1; LUT_SIZE],
      n_cycles_16: [1; LUT_SIZE],
      s_cycles_16: [1; LUT_SIZE]
    }
  }

  pub fn init(&mut self) {
    self.n_cycles_32[BOARD_RAM_PAGE] = 6;
    self.s_cycles_32[BOARD_RAM_PAGE] = 6;
    self.n_cycles_16[BOARD_RAM_PAGE] = 3;
    self.s_cycles_16[BOARD_RAM_PAGE] = 3;

    self.n_cycles_32[OAM_RAM_PAGE] = 2;
    self.s_cycles_32[OAM_RAM_PAGE] = 2;
    self.n_cycles_16[OAM_RAM_PAGE] = 1;
    self.s_cycles_16[OAM_RAM_PAGE] = 1;

    self.n_cycles_32[VRAM_PAGE] = 2;
    self.s_cycles_32[VRAM_PAGE] = 2;
    self.n_cycles_16[VRAM_PAGE] = 1;
    self.s_cycles_16[VRAM_PAGE] = 1;

    self.n_cycles_32[PALRAM_PAGE] = 2;
    self.s_cycles_32[PALRAM_PAGE] = 2;
    self.n_cycles_16[PALRAM_PAGE] = 1;
    self.s_cycles_16[PALRAM_PAGE] = 1;
  }
}