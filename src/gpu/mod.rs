use std::{rc::Rc, cell::Cell, time::{SystemTime, UNIX_EPOCH, Duration}, thread::sleep};

use crate::{cpu::{dma::dma_channels::{DmaChannels, HBLANK_TIMING, VBLANK_TIMING}, registers::{interrupt_enable_register::{FLAG_HBLANK, FLAG_VBLANK, FLAG_VCOUNTER_MATCH}, interrupt_request_register::InterruptRequestRegister}, CPU}, scheduler::{EventType, Scheduler}};

use self::{registers::{display_status_register::DisplayStatusRegister, display_control_register::DisplayControlRegister, bg_control_register::BgControlRegister, color_effects_register::ColorEffectsRegister, alpha_blend_register::AlphaBlendRegister, brightness_register::BrightnessRegister, window_horizontal_register::WindowHorizontalRegister, window_vertical_register::WindowVerticalRegister, window_in_register::WindowInRegister, window_out_register::WindowOutRegister}, picture::Picture};

pub mod registers;
pub mod picture;
pub mod rendering;
pub mod pixel_processing;

pub const HDRAW_CYCLES: u32 = 960;
pub const HBLANK_CYCLES: u32 = 272;

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
  pub is_window: bool,
  pub is_transparent: bool
}

impl ObjectPixel {
  pub fn new() -> Self {
    Self {
      priority: 4,
      color: None,
      is_window: false,
      is_transparent: false
    }
  }
}

pub struct GPU {
  pub vcount: u16,
  pub dispstat: DisplayStatusRegister,
  pub dispcnt: DisplayControlRegister,
  pub picture: Picture,
  pub vram: Box<[u8]>,
  pub palette_ram: Box<[u8]>,
  pub oam_ram: Box<[u8]>,
  pub bgcnt: [BgControlRegister; 4],
  pub bgxofs: [u16; 4],
  pub bgyofs: [u16; 4],
  pub bg_props: [BgProps; 2],
  interrupt_request: Rc<Cell<InterruptRequestRegister>>,
  vram_obj_start: u32,
  bg_lines: [[Option<(u8, u8, u8)>; SCREEN_WIDTH as usize]; 4],
  obj_lines: Box<[ObjectPixel]>,
  dma_channels: Rc<Cell<DmaChannels>>,
  previous_time: u128,
  pub bldcnt: ColorEffectsRegister,
  pub bldalpha: AlphaBlendRegister,
  pub bldy: BrightnessRegister,
  pub winh: [WindowHorizontalRegister; 2],
  pub winv: [WindowVerticalRegister; 2],
  pub winin: WindowInRegister,
  pub winout: WindowOutRegister,
  pub frame_finished: bool
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
  pub fn new(
    interrupt_request: Rc<Cell<InterruptRequestRegister>>,
    dma_channels: Rc<Cell<DmaChannels>>,
  ) -> Self {
    Self {
      vcount: 0,
      bg_props: [BgProps::new(); 2],
      dispstat: DisplayStatusRegister::from_bits_retain(0),
      dispcnt: DisplayControlRegister::from_bits_retain(0x80),
      vram: vec![0; VRAM_SIZE].into_boxed_slice(),
      palette_ram: vec![0; PALETTE_RAM_SIZE].into_boxed_slice(),
      oam_ram: vec![0; OAM_RAM_SIZE].into_boxed_slice(),
      picture: Picture::new(),
      bgcnt: [BgControlRegister::from_bits_retain(0); 4],
      interrupt_request,
      vram_obj_start: 0x1_0000,
      bg_lines: [[None; SCREEN_WIDTH as usize]; 4],
      bgxofs: [0; 4],
      bgyofs: [0; 4],
      dma_channels,
      obj_lines: vec![ObjectPixel::new(); (SCREEN_WIDTH * SCREEN_HEIGHT) as usize].into_boxed_slice(),
      previous_time: 0,
      bldcnt: ColorEffectsRegister::new(),
      bldalpha: AlphaBlendRegister::new(),
      bldy: BrightnessRegister::new(),
      winh: [WindowHorizontalRegister::new(); 2],
      winv: [WindowVerticalRegister::new(); 2],
      winin: WindowInRegister::from_bits_retain(0),
      winout: WindowOutRegister::from_bits_retain(0),
      frame_finished: false
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
    for x in &mut self.obj_lines.iter_mut() {
      *x = ObjectPixel::new();
    }
  }

  pub fn handle_hblank(&mut self, scheduler: &mut Scheduler) {
    scheduler.schedule(EventType::Hdraw, HDRAW_CYCLES as usize);
    self.update_vcount(self.vcount + 1);

    self.dispstat.remove(DisplayStatusRegister::HBLANK);

    if self.vcount >= VISIBLE_LINES {
      if self.vcount == VISIBLE_LINES {
        self.frame_finished = true;
        // entering vblank
        self.dispstat.insert(DisplayStatusRegister::VBLANK);
      }

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
    if self.vcount == VISIBLE_LINES + VBLANK_LINES {
      self.update_vcount(0);

      self.dispstat.remove(DisplayStatusRegister::VBLANK);
    }
  }

  pub fn handle_hdraw(&mut self, scheduler: &mut Scheduler) {
    scheduler.schedule(EventType::Hblank, HBLANK_CYCLES as usize);
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
}