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

pub const CYCLES_PER_SCANLINE: u32 = HDRAW_CYCLES + HBLANK_CYCLES;
pub const SCANLINES_PER_FRAME: u32 = VISIBLE_LINES as u32 + VBLANK_LINES as u32;
pub const CYCLES_PER_FRAME: u32 = CYCLES_PER_SCANLINE * SCANLINES_PER_FRAME;

const VRAM_OBJECT_START_TILE: u32 = 0x1_0000;
const VRAM_OBJECT_START_BITMAP: u32 = 0x1_4000;
const COLOR_TRANSPARENT: u16 = 0x8000;

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

    if self.vcount >= VISIBLE_LINES {
      // entering vblank
      self.dispstat.insert(DisplayStatusRegister::VBLANK);

      // latch bg2/bg3 internal coordinates
      for bg_prop in &mut self.bg_props {
        bg_prop.internal_x = bg_prop.x;
        bg_prop.internal_y = bg_prop.y;
      }

      if self.dispstat.contains(DisplayStatusRegister::VBLANK_ENABLE) {
        // send vblank interrupt
      }

      // notify dma that vblank has started

      // reset object buffer
    } else {
      // render scanline here
      self.render_scanline();

      // update reference points at end of scanline
      for bg_prop in &mut self.bg_props {
        bg_prop.internal_x += bg_prop.dmx as i32;
        bg_prop.internal_y += bg_prop.dmy as i32;
      }
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


  /* to convert to rgb888
    r_8 = (r << 3) | (r >> 2)
    g_8 = (g << 2) | (g >> 4)
    b_8 = (b << 3) | (b >> 2)
  */
  // todo: add offsets
  fn get_palette_color(&self, index: u32) -> Option<(u8, u8, u8)> {
    let value = if index == 0 {
      COLOR_TRANSPARENT
    } else {
      let lower = self.palette_ram[index as usize];
      let upper = self.palette_ram[(index + 1) as usize];

      ((lower as u16) | (upper as u16) << 8) & 0x7fff
    };

    // now turn this into an rgb format that sdl can use
    let mut r = (value & 0b11111) as u8;
    let mut g = ((value >> 5) & 0b11111) as u8;
    let mut b = ((value >> 10) & 0b11111) as u8;

    r = (r << 3) | (r >> 2);
    g = (g << 3) | (g >> 2);
    b = (b << 3) | (b >> 2);

    if value == COLOR_TRANSPARENT { None } else {Some((r, g, b)) }
  }

  fn render_objects(&mut self) {

  }

  fn render_mode4(&mut self) {
    let bg2_index = 2;

    let page: u32 = if self.dispcnt.contains(DisplayControlRegister::DISPLAY_FRAME_SELECT) {
      0xa000
    } else {
      0
    };

    let y = self.vcount;
    let (ref_x, ref_y) = (self.bg_props[bg2_index-2].internal_x, self.bg_props[bg2_index-2].internal_y);
    let dx = self.bg_props[bg2_index-2].dx;
    let dy = self.bg_props[bg2_index-2].dy;

    for x in 0..SCREEN_WIDTH {
      let (mut transformed_x, mut transformed_y) = self.bg_transform(ref_x, ref_y, x as i32, dx as i32, dy as i32);

      if transformed_x < 0 || transformed_x >= SCREEN_WIDTH as i32 || transformed_y < 0 || transformed_y >= SCREEN_HEIGHT as i32 {
        if self.bgcnt[bg2_index].contains(BgControlRegister::DISPLAY_AREA_OVERFLOW) {
          transformed_x %= SCREEN_WIDTH as i32;
          transformed_y %= SCREEN_HEIGHT as i32;
        } else {
          continue;
        }
      }

      let vram_index = ((transformed_x as u32 + transformed_y as u32 * SCREEN_WIDTH as u32) + page as u32) as usize;

      let color_index = self.vram[vram_index];

      if let Some(color) = self.get_palette_color(color_index as u32) {
        self.picture.set_pixel(x as usize, y as usize, color);
      }
    }

  }

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
      _ => { println!("mode not yet supported: {}", self.dispcnt.bg_mode()) }
    }
  }

  fn bg_transform(&self, ref_x: i32, ref_y: i32, screen_x: i32, dx: i32, dy: i32) -> (i32, i32) {
    (((ref_x + screen_x * dx) >> 8), ((ref_y + screen_x * dy) >> 8))
  }
}