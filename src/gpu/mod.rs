use std::{rc::Rc, cell::Cell, time::{SystemTime, UNIX_EPOCH, Duration}, thread::sleep, cmp};

use crate::cpu::{registers::{interrupt_request_register::InterruptRequestRegister, interrupt_enable_register::{FLAG_VBLANK, FLAG_VCOUNTER_MATCH, FLAG_HBLANK}}, CPU, dma::dma_channels::{DmaChannels, VBLANK_TIMING, HBLANK_TIMING}};

use self::{registers::{display_status_register::DisplayStatusRegister, display_control_register::DisplayControlRegister, bg_control_register::BgControlRegister, color_effects_register::{ColorEffectsRegister, ColorEffect}, alpha_blend_register::AlphaBlendRegister, brightness_register::BrightnessRegister, window_horizontal_register::WindowHorizontalRegister, window_vertical_register::WindowVerticalRegister, window_in_register::WindowInRegister, window_out_register::WindowOutRegister}, picture::Picture};

pub mod registers;
pub mod picture;
pub mod rendering;

const HDRAW_CYCLES: u32 = 960;
const HBLANK_CYCLES: u32 = 272;

const VISIBLE_LINES: u16 = 160;
const VBLANK_LINES: u16 = 68;

pub const SCREEN_WIDTH: u16 = 240;
pub const SCREEN_HEIGHT: u16 = 160;

pub const MODE5_WIDTH: u16 = 160;
pub const MODE5_HEIGHT: u16 = 128;

pub const VRAM_SIZE: usize = 128 * 1024;
pub const PALETTE_RAM_SIZE: usize = 1024;
pub const OAM_RAM_SIZE: usize = 1024;

pub const CYCLES_PER_SCANLINE: u32 = HDRAW_CYCLES + HBLANK_CYCLES;
pub const SCANLINES_PER_FRAME: u32 = VISIBLE_LINES as u32 + VBLANK_LINES as u32;
pub const CYCLES_PER_FRAME: u32 = CYCLES_PER_SCANLINE * SCANLINES_PER_FRAME;

const VRAM_OBJECT_START_TILE: u32 = 0x1_0000;
const VRAM_OBJECT_START_BITMAP: u32 = 0x1_4000;
const COLOR_TRANSPARENT: u16 = 0x8000;

const FPS_INTERVAL: u32 = 1000 / 60;

enum WindowType {
  Zero = 0,
  One = 1,
  Obj = 2,
  Out = 3,
  None = 4
}

#[derive(Copy, Clone)]
pub struct ObjectPixel {
  pub priority: u16,
  pub color: Option<(u8, u8, u8)>,
  pub is_window: bool
}

impl ObjectPixel {
  pub fn new() -> Self {
    Self {
      priority: 4,
      color: None,
      is_window: false
    }
  }
}

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
  pub bgxofs: [u16; 4],
  pub bgyofs: [u16; 4],
  pub bg_props: [BgProps; 2],
  interrupt_request: Rc<Cell<InterruptRequestRegister>>,
  vram_obj_start: u32,
  bg_lines: [[Option<(u8, u8, u8)>; SCREEN_WIDTH as usize]; 4],
  obj_lines: [ObjectPixel; (SCREEN_WIDTH * SCREEN_HEIGHT) as usize],
  dma_channels: Rc<Cell<DmaChannels>>,
  previous_time: u128,
  pub bldcnt: ColorEffectsRegister,
  pub bldalpha: AlphaBlendRegister,
  pub bldy: BrightnessRegister,
  pub winh: [WindowHorizontalRegister; 2],
  pub winv: [WindowVerticalRegister; 2],
  pub winin: WindowInRegister,
  pub winout: WindowOutRegister
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
  pub fn new(interrupt_request: Rc<Cell<InterruptRequestRegister>>, dma_channels: Rc<Cell<DmaChannels>>) -> Self {
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
      interrupt_request,
      vram_obj_start: 0x1_0000,
      bg_lines: [[None; SCREEN_WIDTH as usize]; 4],
      bgxofs: [0; 4],
      bgyofs: [0; 4],
      dma_channels,
      obj_lines: [ObjectPixel::new(); (SCREEN_WIDTH * SCREEN_HEIGHT) as usize],
      previous_time: 0,
      bldcnt: ColorEffectsRegister::new(),
      bldalpha: AlphaBlendRegister::new(),
      bldy: BrightnessRegister::new(),
      winh: [WindowHorizontalRegister::new(); 2],
      winv: [WindowVerticalRegister::new(); 2],
      winin: WindowInRegister::from_bits_retain(0),
      winout: WindowOutRegister::from_bits_retain(0)
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
      CPU::trigger_interrupt(&self.interrupt_request, FLAG_VCOUNTER_MATCH);
    }
  }

  fn clear_obj_lines(&mut self) {
    for x in &mut self.obj_lines {
      *x = ObjectPixel::new();
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
        CPU::trigger_interrupt(&self.interrupt_request, FLAG_VBLANK)
      }

      // notify dma that vblank has started
      let mut dma = self.dma_channels.get();

      dma.notify_gpu_event(VBLANK_TIMING);

      self.dma_channels.set(dma);

      self.clear_obj_lines();
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
      CPU::trigger_interrupt(&self.interrupt_request, FLAG_HBLANK);
    }

    if self.vcount <= VISIBLE_LINES {
      // notify dma that hblank has started
      let mut dma = self.dma_channels.get();

      dma.notify_gpu_event(HBLANK_TIMING);

      self.dma_channels.set(dma);
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

  fn get_palette_color(&self, index: usize, palette_bank: usize, offset: usize) -> Option<(u8, u8, u8)> {
    let value = if index == 0 || (palette_bank != 0 && index % 16 == 0) {
      COLOR_TRANSPARENT
    } else {
      let index = offset + 2 * index + 32 * palette_bank;

      let lower = self.palette_ram[index];
      let upper = self.palette_ram[index + 1];

      ((lower as u16) | (upper as u16) << 8) & 0x7fff
    };

    self.get_rgb(value)
  }

  // TODO: refactor this and get rid of x_flip and y_flip
  fn get_pixel_index_bpp8(&self, address: u32, tile_x: u16, tile_y: u16, x_flip: bool, y_flip: bool) -> u8 {
    let tile_x = if x_flip { 7 - tile_x } else { tile_x };
    let tile_y = if y_flip { 7 - tile_y } else { tile_y };

    self.vram[(address + tile_x as u32 + (tile_y as u32) * 8) as usize]
  }

  fn get_pixel_index_bpp4(&self, address: u32, tile_x: u16, tile_y: u16, x_flip: bool, y_flip: bool) -> u8 {
    let tile_x = if x_flip { 7 - tile_x } else { tile_x };
    let tile_y = if y_flip { 7 - tile_y } else { tile_y };

    let address = address + (tile_x / 2) as u32 + (tile_y as u32) * 4;

    let byte = self.vram[address as usize];

    if tile_x & 0b1 == 1 {
      byte >> 4
    } else {
      byte & 0xf
    }
  }

  /* to convert to rgb888
    r_8 = (r << 3) | (r >> 2)
    g_8 = (g << 2) | (g >> 4)
    b_8 = (b << 3) | (b >> 2)
  */
  fn translate_to_rgb24(&self, value: (u8, u8, u8)) -> (u8, u8, u8) {
    let (mut r, mut g, mut b) = value;

    r = (r << 3) | (r >> 2);
    g = (g << 3) | (g >> 2);
    b = (b << 3) | (b >> 2);

   (r, g, b)
  }

  fn get_rgb(&self, value: u16) -> Option<(u8, u8, u8)> {
    let r = (value & 0b11111) as u8;
    let g = ((value >> 5) & 0b11111) as u8;
    let b = ((value >> 10) & 0b11111) as u8;

    if value != COLOR_TRANSPARENT { Some((r,g,b))} else { None }
  }

  fn bg_enabled(&self, bg_index: usize) -> bool {
    match bg_index {
      0 => self.dispcnt.contains(DisplayControlRegister::DISPLAY_BG0),
      1 => self.dispcnt.contains(DisplayControlRegister::DISPLAY_BG1),
      2 => self.dispcnt.contains(DisplayControlRegister::DISPLAY_BG2),
      3 => self.dispcnt.contains(DisplayControlRegister::DISPLAY_BG3),
      _ => panic!("shouldn't happen")
    }
  }

  pub fn cap_fps(&mut self) {
    let current_time = SystemTime::now()
      .duration_since(UNIX_EPOCH)
      .expect("an error occurred")
      .as_millis();

    if self.previous_time != 0 {
      let diff = current_time - self.previous_time;
      if diff < FPS_INTERVAL as u128 {
        // sleep for the missing time
        sleep(Duration::from_millis((FPS_INTERVAL - diff as u32) as u64));
      }
    }

    self.previous_time = SystemTime::now()
      .duration_since(UNIX_EPOCH)
      .expect("an error occurred")
      .as_millis();
  }

  fn finalize_scanline(&mut self, start: usize, end: usize) {
    let mut sorted: Vec<usize> = Vec::new();

    for i in start..=end {
      if self.bg_enabled(i) {
        sorted.push(i);
      }
    }

    sorted.sort_by_key(|key| (self.bgcnt[*key].bg_priority(), *key));


    let mut occupied = [false; SCREEN_WIDTH as usize];

    let y = self.vcount;

    if self.dispcnt.windows_enabled() {
      if self.dispcnt.contains(DisplayControlRegister::DISPLAY_WINDOW_0) {
        if y >= self.winv[0].y1 && y < self.winv[0].y2 {
          let mut window_sorted: Vec<usize> = Vec::new();

          for bg in &sorted {
            if (self.winin.window0_bg_enable() >> bg) & 0b1 == 1 {
              window_sorted.push(*bg);
            }
          }

          for x in self.winh[0].x1..self.winh[0].x2 {
            if !occupied[x as usize] {
              occupied[x as usize] = true;
              self.finalize_pixel(x as usize, &window_sorted, WindowType::Zero);
            }
          }
        }
      }
      if self.dispcnt.contains(DisplayControlRegister::DISPLAY_WINDOW_1) {
        if y >= self.winv[1].y1 && y < self.winv[1].y2 {
          let mut window_sorted: Vec<usize> = Vec::new();

          for bg in &sorted {
            if (self.winin.window1_bg_enable() >> bg) & 0b1 == 1 {
              window_sorted.push(*bg);
            }
          }
          for x in self.winh[0].x1..self.winh[0].x2 {
            if !occupied[x as usize] {
              occupied[x as usize] = true;
              self.finalize_pixel(x as usize, &window_sorted, WindowType::One);
            }
          }
        }
      }


      let mut outside_sorted: Vec<usize> = Vec::new();

      for bg in &sorted {
        if (self.winout.outside_window_background_enable_bits() >> bg) & 0b1 == 1 {
          outside_sorted.push(*bg);
        }
      }

      if self.dispcnt.contains(DisplayControlRegister::DISPLAY_OBJ_WINDOW) {
        for x in 0..SCREEN_WIDTH {
          if !occupied[x as usize] {
            occupied[x as usize] = true;
            let obj_index = (x + y * SCREEN_WIDTH) as usize;

            if self.obj_lines[obj_index].is_window {
              self.finalize_pixel(x as usize, &outside_sorted, WindowType::Obj);
            } else {
              self.finalize_pixel(x as usize, &outside_sorted, WindowType::Out);
            }
          }
        }
      }

      // finally render pixels outside of window
      for x in 0..SCREEN_WIDTH {
        if !occupied[x as usize] {
          self.finalize_pixel(x as usize, &outside_sorted, WindowType::Out);
        }
      }
    } else {
      // no windows enabled, just render as normal
      for x in 0..SCREEN_WIDTH {
        self.finalize_pixel(x as usize, &sorted, WindowType::None);
      }
    }
  }

  fn is_window_obj_enabled(&self, window_type: &WindowType) -> bool {
    match window_type {
      WindowType::Zero => {
        self.winin.contains(WindowInRegister::Window0ObjEnable)
      }
      WindowType::One => {
        self.winin.contains(WindowInRegister::Window1ObjEnable)
      }
      WindowType::Obj => {
        self.winout.contains(WindowOutRegister::ObjWindowObjEnable)
      }
      WindowType::Out => {
        self.winout.contains(WindowOutRegister::OutsideWindowObjEnable)
      }
      WindowType::None => true
    }
  }

  fn window_apply_effects(&self, window_type: &WindowType) -> bool {
    match window_type {
      WindowType::Zero => {
        self.winin.contains(WindowInRegister::Window0ColorEffect)
      }
      WindowType::One => {
        self.winin.contains(WindowInRegister::Window1ColorEffect)
      }
      WindowType::Obj => {
        self.winout.contains(WindowOutRegister::ObjWIndowColorEffect)
      }
      WindowType::Out => {
        self.winout.contains(WindowOutRegister::OutsideWindowColorEffect)
      }
      WindowType::None => true
    }
  }

  fn finalize_pixel(&mut self, x: usize, sorted: &Vec<usize>, window_type: WindowType) {
    let default_color = self.get_rgb((self.palette_ram[0] as u16) | (self.palette_ram[1] as u16) << 8);

    // disregard blending effects for now so we can just draw the top layer.
    let mut top_layer: isize = -1;
    let mut top_layer_priority: isize = -1;

    let mut bottom_layer: isize = -1;
    let mut bottom_layer_priority: isize = -1;

    let y = self.vcount;

    let obj_line_index = x + y as usize * SCREEN_WIDTH as usize;

    for index in sorted {
      // if the pixel isn't transparent
      if let Some(_) = self.bg_lines[*index][x] {
        if top_layer == -1 {
          top_layer = *index as isize;
          top_layer_priority = self.bgcnt[*index].bg_priority() as isize;
        } else {
          bottom_layer = *index as isize;
          bottom_layer_priority = self.bgcnt[*index].bg_priority() as isize;

          break;
        }
      }
    }

    // check to see if object layer has higher priority
    if self.dispcnt.contains(DisplayControlRegister::DISPLAY_OBJ) && self.is_window_obj_enabled(&window_type) {
      if top_layer_priority == -1 || (self.obj_lines[obj_line_index].priority <= top_layer_priority as u16) {
        bottom_layer = top_layer;
        top_layer = 4;
      } else if bottom_layer_priority == -1 ||  (self.obj_lines[obj_line_index].priority <= bottom_layer_priority as u16) {
        bottom_layer = 4;
      }
    }

    if top_layer < 4 && top_layer >= 0 {
      // safe to unwrap at this point since we have verified above the color exists
      let mut color = self.bg_lines[top_layer as usize][x as usize].unwrap();

      if self.bldcnt.bg_first_pixels[top_layer as usize] && self.window_apply_effects(&window_type) {
        match self.bldcnt.color_effect {
          ColorEffect::AlphaBlending => {
            let mut blend_layer: isize = -1;
            for i in 0..self.bldcnt.bg_second_pixels.len() {
              if self.bldcnt.bg_second_pixels[i] {
                blend_layer = i as isize;
                break;
              }
            }

            if blend_layer != -1 {
              if let Some(color2) = self.bg_lines[blend_layer as usize][x as usize] {
                // do alpha blending here
                color = self.blend_colors(color, color2, self.bldalpha.eva as u16, self.bldalpha.evb as u16);
              }
            }
          }
          ColorEffect::Darken => {
            // blending with black
            let color2: (u8, u8, u8) = (0, 0, 0);

            color = self.blend_colors(color, color2, (16 - self.bldy.evy) as u16, self.bldy.evy as u16);
          }
          ColorEffect::Brighten => {
            // blending with white
            let color2: (u8, u8, u8) = (31, 31, 31);

            color = self.blend_colors(color, color2, (16 - self.bldy.evy) as u16, self.bldy.evy as u16);
          }
          ColorEffect::None => ()
        }
      }

      self.picture.set_pixel(x, y as usize, self.translate_to_rgb24(color));
    } else if let Some(color) = self.obj_lines[obj_line_index].color {
      // render object pixel
      self.picture.set_pixel(x, y as usize, self.translate_to_rgb24(color));
    } else if bottom_layer != -1 {
      let color = self.bg_lines[bottom_layer as usize][x as usize].unwrap();
      self.picture.set_pixel(x, y as usize, self.translate_to_rgb24(color));
    }
    else {
      self.picture.set_pixel(x, y as usize, self.translate_to_rgb24(default_color.unwrap()));
    }
  }

  fn darken_color(&self, color: (u8, u8, u8)) -> (u8, u8, u8) {
    let r = cmp::min(31, color.0 - color.0 * self.bldy.evy);
    let g = cmp::min(31, color.1 - color.1 * self.bldy.evy);
    let b = cmp::min(31, color.2 - color.2 * self.bldy.evy);

    (r, g, b)
  }

  fn lighten_color(&self, color: (u8, u8, u8)) -> (u8, u8, u8) {
    let r = cmp::min(31, color.0 + (31 - self.bldy.evy) * color.0);
    let g = cmp::min(31, color.1 + (31 - self.bldy.evy) * color.1);
    let b = cmp::min(31, color.2 + (31 - self.bldy.evy) * color.2);

    (r, g, b)
  }

  fn blend_colors(&self, color: (u8, u8, u8), color2: (u8, u8, u8), eva: u16, evb: u16) -> (u8, u8, u8) {
    let r = cmp::min(31, (color.0 as u16 * eva + color2.0 as u16 * evb) >> 4) as u8;
    let g = cmp::min(31, (color.1 as u16 * eva + color2.1 as u16 * evb) >> 4) as u8;
    let b = cmp::min(31, (color.2 as u16 * eva + color2.2 as u16 * evb) >> 4) as u8;

    (r, g, b)
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
      0 => {
        for i in 0..4 {
          if self.bg_enabled(i) {
            self.render_normal_background(i);
          }
        }

        self.finalize_scanline(0, 3);
      }
      1 => {
        if self.dispcnt.contains(DisplayControlRegister::DISPLAY_BG2) {
          self.render_affine_background(2);
        }
        if self.dispcnt.contains(DisplayControlRegister::DISPLAY_BG1) {
          self.render_normal_background(1);
        }
        if self.dispcnt.contains(DisplayControlRegister::DISPLAY_BG0) {
          self.render_normal_background(0);
        }
        self.finalize_scanline(0, 2);
      }
      2 => {
        if self.dispcnt.contains(DisplayControlRegister::DISPLAY_BG3) {
          self.render_affine_background(3);
        }
        if self.dispcnt.contains(DisplayControlRegister::DISPLAY_BG2) {
          self.render_affine_background(2);
        }
        self.finalize_scanline(2, 3);
      }
      3=> {
        self.render_mode3();
        self.finalize_scanline(2, 2);
      }
      4 => {
        self.render_mode4();
        self.finalize_scanline(2, 2);
      }
      5 => {
        self.render_mode5();
        self.finalize_scanline(2, 2);
      }
      _ => {
        // println!("mode not implemented: {}", self.dispcnt.bg_mode())
      }
    }
  }

  fn bg_transform(&self, ref_x: i32, ref_y: i32, screen_x: i32, dx: i32, dy: i32) -> (i32, i32) {
    (((ref_x + screen_x * dx) >> 8), ((ref_y + screen_x * dy) >> 8))
  }
}