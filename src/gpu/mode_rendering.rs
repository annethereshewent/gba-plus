use super::{GPU, SCREEN_WIDTH, SCREEN_HEIGHT, registers::{bg_control_register::BgControlRegister, display_control_register::DisplayControlRegister}, MODE5_WIDTH, MODE5_HEIGHT};

// 2 bytes per tile
const SCREEN_BLOCK_SIZE: u32 = 32 * 32 * 2;

impl GPU {
  pub fn render_normal_background(&mut self, background_id: usize) {
    let (x_offset, y_offset) = (self.bgxofs[background_id], self.bgyofs[background_id]);

    let tilemap_base = (self.bgcnt[background_id].screen_base_block() as u32) * 2048;
    let tile_base = (self.bgcnt[background_id].character_base_block() as u32) * 16 * 1024;

    let (background_width, background_height) = self.bgcnt[background_id].get_screen_dimensions();

    let mut x = 0;
    let y = self.vcount;

    let x_in_bg = (x + x_offset) % background_width;
    let y_in_bg = (y + y_offset) % background_height;

    let mut screen_index = match self.bgcnt[background_id].screen_size() {
      0 => 0,
      1 => x_in_bg / 256, // 512 x 256
      2 => y_in_bg / 256, // 256 x 512
      3 => (x_in_bg / 256) + ((y_in_bg / 256) * 2), // 512 x 512
      _ => unreachable!("not possible")
    };

    let tile_size: u32 = if self.bgcnt[background_id].contains(BgControlRegister::PALETTES) {
      64
    } else {
      32
    };

    // 32 x 32 tilemap
    let mut tile_num_horizontal = (x_in_bg / 8) % 32;
    let tile_num_vertical = (y_in_bg / 8 ) % 32;

    // initial x pos in tile
    let mut x_pos_in_tile = x_in_bg % 8;
    let tile_y = y_in_bg % 8;

    // finally render the background
    while x < SCREEN_WIDTH {
      let tile_number = tile_num_horizontal as u32 + (tile_num_vertical as u32) * 32;
      let mut tilemap_address = tilemap_base + SCREEN_BLOCK_SIZE * screen_index as u32  + 2 * tile_number;

      'outer: for _ in tile_num_horizontal..32 {
        let attributes = (self.vram[tilemap_address as usize] as u16) | (self.vram[(tilemap_address + 1) as usize] as u16) << 8;

        let x_flip = (attributes >> 10) & 0b1 == 1;
        let y_flip =  (attributes >> 11) & 0b1 == 1;
        let palette_number = (attributes >> 12) & 0b1111;
        let tile_number = attributes & 0b1111111111;

        let tile_address = tile_base + tile_number as u32 * tile_size as u32;

        for tile_x in x_pos_in_tile..8 {
          let palette_index = if tile_size == 64 {
            self.get_pixel_index_bpp8(tile_address, tile_x, tile_y, x_flip, y_flip)
          } else {
            self.get_pixel_index_bpp4(tile_address, tile_x, tile_y, x_flip, y_flip)
          };

          let palette_bank = if tile_size == 64 {
            0
          } else {
            palette_number
          };

          self.bg_lines[background_id][x as usize] = self.get_palette_color(palette_index as u32, palette_bank as usize);

          x += 1;

          if x == SCREEN_WIDTH {
            break 'outer;
          }
        }
        x_pos_in_tile = 0;
        tilemap_address += 2;
      }
      tile_num_horizontal = 0;
      if background_width == 512 {
        screen_index ^= 1;
      }
    }

  }

  pub fn render_affine_background(&mut self, background_id: usize) {
    let texture_size = 128 << self.bgcnt[background_id].screen_size();

    let (ref_x, ref_y) = (self.bg_props[background_id - 2].internal_x, self.bg_props[background_id - 2].internal_y);

    let dx = self.bg_props[background_id - 2].dx;
    let dy = self.bg_props[background_id - 2].dy;

    let screen_base = self.bgcnt[background_id].screen_base_block() * 2048;
    let character_base = self.bgcnt[background_id].character_base_block() * 16 * 1024;

    for x in 0..SCREEN_WIDTH {
      let (mut transformed_x, mut transformed_y) = self.bg_transform(ref_x, ref_y, x as i32, dx as i32, dy as i32);

      if transformed_x < 0 || transformed_x >= texture_size as i32 || transformed_y < 0 || transformed_y >= texture_size as i32 {
        if self.bgcnt[background_id].contains(BgControlRegister::DISPLAY_AREA_OVERFLOW) {
          transformed_x = transformed_x.rem_euclid(texture_size.into());
          transformed_y = transformed_y.rem_euclid(texture_size.into());
        } else {
          // -1 means transparent
          self.bg_lines[background_id][x as usize] = None;
          continue;
        }
      }

      // tiles are 8 * 8 pixels
      let vram_index = screen_base as usize + (transformed_x as usize / 8) + (transformed_y as usize / 8) * (texture_size as usize / 8);

      let tile_index = self.vram[vram_index] as u32;
      let tile_address_base = character_base as u32 + tile_index * 0x40;

      let x_pos_in_tile = transformed_x % 8;
      let y_pos_in_tile = transformed_y % 8;

      let tile_address = (tile_address_base + x_pos_in_tile as u32 + ((y_pos_in_tile as u32) * 8)) as usize;

      let palette_index = self.vram[tile_address];

      self.bg_lines[background_id][x as usize] = self.get_palette_color(palette_index as u32, 0);
    }
  }

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

      if let Some(color) = self.get_palette_color(color_index as u32, 0) {
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
          transformed_x %= MODE5_WIDTH as i32;
          transformed_y %= MODE5_HEIGHT as i32;
        } else {
          continue;
        }
      }

      let vram_index = 2 * (transformed_x as usize + transformed_y as usize * MODE5_WIDTH as usize) + page as usize;

      let color_val = (self.vram[vram_index] as u16) | (self.vram[vram_index + 1] as u16) << 8;

      if let Some(color) = self.translate_to_rgb(color_val) {
        self.picture.set_pixel(x as usize, y as usize, color);
      }
    }
  }
}