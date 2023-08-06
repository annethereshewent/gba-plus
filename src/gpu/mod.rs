use self::{registers::{display_status_register::DisplayStatusRegister, display_control_register::DisplayControlRegister, bg_control_register::BgControlRegister}, picture::Picture};

pub mod registers;
pub mod picture;

const HDRAW_CYCLES: u32 = 960;
const HBLANK_CYCLES: u32 = 272;

const VISIBLE_LINES: u16 = 160;
const VBLANK_LINES: u16 = 68;

pub const SCREEN_WIDTH: u16 = 240;
pub const SCREEN_HEIGHT: u16 = 160;

pub const VRAM_SIZE: usize = 96 * 1024;
pub const PALETTE_RAM_SIZE: usize = 1024;
pub const OAM_RAM_SIZE: usize = 1024;

const VRAM_OBJECT_START_TILE: u32 = 0x1_0000;
const VRAM_OBJECT_START_BITMAP: u32 = 0x1_4000;

pub struct GPU {
  cycles: u32,
  mode: GpuMode,
  pub vcount: u16,
  pub dispstat: DisplayStatusRegister,
  pub dispcnt: DisplayControlRegister,
  pub picture: Picture,
  pub vram: [u8; VRAM_SIZE],
  pub palette_ram: [u8; PALETTE_RAM_SIZE],
  pub oam_ram: [u8; OAM_RAM_SIZE],
  pub bgcnt: [BgControlRegister; 4],
  pub bg_props: [BgProps; 2],
  vram_obj_start: u32
}

enum GpuMode {
  Hblank,
  Hdraw
}

#[derive(Copy, Clone)]
pub struct BgProps {
  pub x: i32,
  pub y: i32,
  pub dx: i16,
  pub dmx: i16,
  pub dy: i16,
  pub dmy: i16,
  pub internal_x: i32,
  pub internal_y: i32
}

impl BgProps {
  pub fn new() -> Self {
    Self {
      x: 0,
      y: 0,
      dx: 0,
      dmx: 0,
      dy: 0,
      dmy: 0,
      internal_x: 0,
      internal_y: 0
    }
  }
}

impl GPU {
  pub fn new() -> Self {
    Self {
      cycles: 0,
      vcount: 0,
      mode: GpuMode::Hdraw,
      bg_props: [BgProps::new(); 2],
      dispstat: DisplayStatusRegister::from_bits_retain(0),
      dispcnt: DisplayControlRegister::from_bits_retain(0x80),
      vram: [0; VRAM_SIZE],
      palette_ram: [0; PALETTE_RAM_SIZE],
      oam_ram: [0; OAM_RAM_SIZE],
      picture: Picture::new(),
      bgcnt: [BgControlRegister::from_bits_retain(0); 4],
      vram_obj_start: 0x1_0000
    }
  }

  pub fn write_dispcnt(&mut self, value: u16) {
    let mode = self.dispcnt.bg_mode();
    self.dispcnt = DisplayControlRegister::from_bits_retain(value);

    // if mode has changed
    if mode != self.dispcnt.bg_mode() {
      // change where the obj tiles are fetched from
      self.vram_obj_start = if self.dispcnt.bg_mode() < 3 {
        VRAM_OBJECT_START_TILE
      } else {
        VRAM_OBJECT_START_BITMAP
      };
    }
  }

  fn update_vcount(&mut self, count: u16) {
    self.vcount = count;

    self.dispstat.set(DisplayStatusRegister::VCOUNTER, self.dispstat.vcount_setting() == self.vcount);

    if self.dispstat.contains(DisplayStatusRegister::VCOUNTER_ENABLE) && self.dispstat.contains(DisplayStatusRegister::VCOUNTER) {
      // trigger vcounter interrupt here
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

  fn render_objects(&mut self) {

  }

  fn render_mode4(&mut self) {

  }

  /* to convert to rgb888
    r_8 = (r << 3) | (r >> 2)
    g_8 = (g << 2) | (g >> 4)
    b_8 = (b << 3) | (b >> 2)
  */

  fn render_scanline(&mut self) {
    if self.dispcnt.contains(DisplayControlRegister::FORCED_BLANK) {
      for i in 0..SCREEN_WIDTH {
        self.picture.set_pixel(i as usize, self.vcount as usize, (0xf8, 0xf8, 0xf8));
      }

      return;
    }

    if self.dispcnt.contains(DisplayControlRegister::DISPLAY_OBJ) {
      self.render_objects();
    }

    match self.dispcnt.bg_mode() {
      4 => {
        self.render_mode4();
      }
      _ => ()
    }
  }
}