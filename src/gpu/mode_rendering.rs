use super::{GPU, SCREEN_WIDTH, SCREEN_HEIGHT, registers::{bg_control_register::BgControlRegister, display_control_register::DisplayControlRegister}, MODE5_WIDTH, MODE5_HEIGHT};

impl GPU {
  pub fn render_mode3(&mut self) {
    let bg2_index = 2;

    let y = self.vcount;
    let (ref_x, ref_y) = (self.bg_props[bg2_index-2].internal_x, self.bg_props[bg2_index - 2].internal_y);

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

      let vram_index = 2 * (transformed_x as usize + transformed_y as usize * SCREEN_WIDTH as usize);

      let color_val = (self.vram[vram_index] as u16) | (self.vram[vram_index + 1] as u16) << 8;

      if let Some(color) = self.translate_to_rgb(color_val) {
        self.picture.set_pixel(x as usize, y as usize, color);
      }
    }
  }

  pub fn render_mode4(&mut self) {
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

  pub fn render_mode5(&mut self) {
    let bg2_index = 2;

    let page: u32 = if self.dispcnt.contains(DisplayControlRegister::DISPLAY_FRAME_SELECT) {
      0xa000
    } else {
      0
    };

    let y = self.vcount;
    let (ref_x, ref_y) = (self.bg_props[bg2_index-2].internal_x, self.bg_props[bg2_index - 2].internal_y);

    let dx = self.bg_props[bg2_index-2].dx;
    let dy = self.bg_props[bg2_index-2].dy;

    for x in 0..SCREEN_WIDTH {
      let (mut transformed_x, mut transformed_y) = self.bg_transform(ref_x, ref_y, x as i32, dx as i32, dy as i32);

      if transformed_x < 0 || transformed_x >= MODE5_WIDTH as i32 || transformed_y < 0 || transformed_y >= MODE5_HEIGHT as i32 {
        if self.bgcnt[bg2_index].contains(BgControlRegister::DISPLAY_AREA_OVERFLOW) {
          transformed_x %= SCREEN_WIDTH as i32;
          transformed_y %= SCREEN_HEIGHT as i32;
        } else {
          continue;
        }
      }

      let vram_index = 2 * (transformed_x as usize + transformed_y as usize * MODE5_HEIGHT as usize) + page as usize;

      let color_val = (self.vram[vram_index] as u16) | (self.vram[vram_index + 1] as u16) << 8;

      if let Some(color) = self.translate_to_rgb(color_val) {
        self.picture.set_pixel(x as usize, y as usize, color);
      }
    }
  }
}