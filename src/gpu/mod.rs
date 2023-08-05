use self::registers::display_status_register::DisplayStatusRegister;

pub mod registers;

const HDRAW_CYCLES: u32 = 960;
const HBLANK_CYCLES: u32 = 272;

const VISIBLE_LINES: u16 = 160;
const VBLANK_LINES: u16 = 68;

const SCREEN_WIDTH: u16 = 240;
const SCREEN_HEIGHT: u16 = 160;

const VRAM_SIZE: usize = 96 * 1024;
const PALETTE_RAM_SIZE: usize = 1024;
const OAM_RAM_SIZE: usize = 1024;

pub struct GPU {
  cycles: u32,
  mode: GpuMode,
  pub vcount: u16,
  pub dispstat: DisplayStatusRegister,
  vram: [u8; VRAM_SIZE],
  palette_ram: [u8; PALETTE_RAM_SIZE],
  oam_ram_size: [u8; OAM_RAM_SIZE]
}

enum GpuMode {
  Hblank,
  Hdraw
}

impl GPU {
  pub fn new() -> Self {
    Self {
      cycles: 0,
      vcount: 0,
      mode: GpuMode::Hdraw,
      dispstat: DisplayStatusRegister::from_bits_retain(0),
      vram: [0; VRAM_SIZE],
      palette_ram: [0; PALETTE_RAM_SIZE],
      oam_ram_size: [0; OAM_RAM_SIZE]
    }
  }

  fn update_vcount(&mut self, count: u16) {
    self.vcount = count;

    self.dispstat.set(DisplayStatusRegister::VCOUNTER, self.dispstat.vcount_setting() == self.vcount);

    if self.dispstat.contains(DisplayStatusRegister::VCOUNTER_ENABLE) && self.dispstat.contains(DisplayStatusRegister::VCOUNTER) {
      // trigger interrupt here
    }
  }

  fn handle_visible_hblank(&mut self) {
    self.update_vcount(self.vcount + 1);

    self.dispstat.remove(DisplayStatusRegister::HBLANK);

    if self.vcount > VISIBLE_LINES {
      // entering vblank
      self.dispstat.insert(DisplayStatusRegister::VBLANK);

      // latch bg2/bg3 stuff here

      if self.dispstat.contains(DisplayStatusRegister::VBLANK_ENABLE) {
        // send vblank interrupt
      }

      // notify dma that vblank has started

      // reset object buffer
    } else {
      // render scanline here
      self.render_scanline();

      // update stuff with bg2/bg3 here
    }
  }

  fn handle_vblank_hblank(&mut self) {
    self.dispstat.remove(DisplayStatusRegister::HBLANK);
    if self.vcount < VISIBLE_LINES + VBLANK_LINES - 1 {
      self.update_vcount(self.vcount + 1);
    } else {
      self.update_vcount(0);

      self.render_scanline();
      self.dispstat.remove(DisplayStatusRegister::VBLANK);

    }
  }

  fn handle_hdraw(&mut self) {
    self.dispstat.insert(DisplayStatusRegister::HBLANK);

    if self.dispstat.contains(DisplayStatusRegister::HBLANK_ENABLE) {
      // send hblank interrupt
    }

    if self.vcount <= VISIBLE_LINES {
      // notify dma that hblank has started
    }
    self.mode = GpuMode::Hblank;
  }

  pub fn tick(&mut self, cycles: u32) {
    self.cycles += cycles;
    match self.mode {
      GpuMode::Hdraw => {
        if self.cycles >= HDRAW_CYCLES {
          self.cycles -= HDRAW_CYCLES;
          self.handle_hdraw();
        }
      }
      GpuMode::Hblank => {
        if self.cycles >= HBLANK_CYCLES {
          self.cycles -= HBLANK_CYCLES;
          if self.vcount <= VISIBLE_LINES {
            // hblank within visible lines
            self.handle_visible_hblank();
          } else {
            // hblank within vblank
            self.handle_vblank_hblank();
          }
          self.mode = GpuMode::Hdraw;
        }
      }
    }
  }

  fn render_scanline(&mut self) {

  }
}