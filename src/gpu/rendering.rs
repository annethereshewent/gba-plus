use super::{GPU, SCREEN_WIDTH, SCREEN_HEIGHT, registers::{bg_control_register::BgControlRegister, display_control_register::DisplayControlRegister}, MODE5_WIDTH, MODE5_HEIGHT, ObjectPixel, COLOR_TRANSPARENT};

// 2 bytes per tile
const SCREEN_BLOCK_SIZE: u32 = 32 * 32 * 2;
const ATTRIBUTE_SIZE: usize = 8; // 6 bytes (3 16 bit attributes) + 2 empty bytes in between
const AFFINE_SIZE: u16 = 3 * 2;

struct OamAttributes {
  x_coordinate: u16,
  y_coordinate: u16,
  rotation_flag: bool,
  double_sized_flag: bool,
  obj_disable: bool,
  obj_mode: u16,
  obj_mosaic: bool,
  palette_flag: bool,
  obj_shape: u16,
  obj_size: u16,
  rotation_param_selection: u16,
  horizontal_flip: bool,
  vertical_flip: bool,
  tile_number: u16,
  priority: u16,
  palette_number: u16
}

impl OamAttributes {
  pub fn get_object_dimensions(&self) -> (u32, u32) {
    match (self.obj_size, self.obj_shape) {
      (0, 0) => (8, 8),
      (1, 0) => (16, 16),
      (2, 0) => (32, 32),
      (3, 0) => (64, 64),
      (0, 1) => (16, 8),
      (1, 1) => (32, 8),
      (2, 1) => (32, 16),
      (3, 1) => (64, 32),
      (0, 2) => (8, 16),
      (1, 2) => (8, 32),
      (2, 2) => (16, 32),
      (3, 2) => (32, 64),
      _ => panic!("object shape of 3 not allowed")
    }
  }
}

impl GPU {
  fn bg_transform(&self, ref_x: i32, ref_y: i32, screen_x: i32, dx: i32, dy: i32) -> (i32, i32) {
    (((ref_x + screen_x * dx) >> 8), ((ref_y + screen_x * dy) >> 8))
  }

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

          self.bg_lines[background_id][x as usize] = self.get_palette_color(palette_index as usize, palette_bank as usize, 0);

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

      self.bg_lines[background_id][x as usize] = self.get_palette_color(palette_index as usize, 0, 0);
    }
  }

  pub fn render_mode3(&mut self) {
    let bg2_index = 2;

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
          self.bg_lines[bg2_index][x as usize] = None;
          continue;
        }
      }

      let vram_index = 2 * (transformed_x as usize + transformed_y as usize * SCREEN_WIDTH as usize);

      let color_val = (self.vram[vram_index] as u16) | (self.vram[vram_index + 1] as u16) << 8;

      self.bg_lines[bg2_index][x as usize] = self.get_rgb(color_val);
    }
  }

  pub fn render_mode4(&mut self) {
    let bg2_index = 2;

    let page: u32 = if self.dispcnt.contains(DisplayControlRegister::DISPLAY_FRAME_SELECT) {
      0xa000
    } else {
      0
    };

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

      self.bg_lines[bg2_index][x as usize] = self.get_palette_color(color_index as usize, 0, 0);
    }
  }

  pub fn render_mode5(&mut self) {
    let bg2_index = 2;

    let page: u32 = if self.dispcnt.contains(DisplayControlRegister::DISPLAY_FRAME_SELECT) {
      0xa000
    } else {
      0
    };

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

      self.bg_lines[bg2_index][x as usize] = self.get_rgb(color_val);
    }
  }

  pub fn render_objects(&mut self) {
    for i in 0..128 {
      let obj_attributes = self.get_attributes(i);

      if obj_attributes.obj_disable {
        continue;
      }
      if obj_attributes.rotation_flag {
        self.render_affine_object(obj_attributes);
      } else {
        // render object normally
        self.render_normal_object(obj_attributes);
      }
    }
  }

  fn render_normal_object(&mut self, obj_attributes: OamAttributes) {
    let y = self.vcount;

    let (obj_width, obj_height) = obj_attributes.get_object_dimensions();

    let (x_coordinate, y_coordinate) = self.get_obj_coordinates(obj_attributes.x_coordinate, obj_attributes.y_coordinate);

    let y_pos_in_sprite: i16 = y as i16 - y_coordinate;

    if y_pos_in_sprite < 0 || y_pos_in_sprite as u32 >= obj_height || obj_attributes.obj_mode == 3 {
      return;
    }

    let tile_number = obj_attributes.tile_number;
    let tile_base: u32 = 0x1_0000 + tile_number as u32 * 32;

    if tile_base < self.vram_obj_start {
      return;
    }

    let tile_size = if obj_attributes.palette_flag {
      64
    } else {
      32
    };

    let tile_width = if self.dispcnt.contains(DisplayControlRegister::OBJ_CHARACTER_MAPPING) {
      obj_width / 8
    } else {
      if obj_attributes.palette_flag {
        16
      } else {
        32
      }
    };

    let palette_bank = if !obj_attributes.palette_flag {
      obj_attributes.palette_number
    } else {
      0
    };

    for x in 0..obj_width {
      let screen_x = x as i16 + x_coordinate;

      if screen_x < 0 {
        continue;
      }

      if screen_x >= SCREEN_WIDTH as i16 {
        break;
      }

      let obj_line_index = (screen_x as u16 + y * SCREEN_WIDTH) as usize;

      if self.obj_lines[obj_line_index].priority <= obj_attributes.priority && obj_attributes.obj_mode != 2 {
        continue;
      }

      let x_pos_in_sprite = if obj_attributes.horizontal_flip {
        obj_width - x - 1
      } else {
        x
      };

      let y_pos_in_sprite = if obj_attributes.vertical_flip {
        (obj_height as i16 - y_pos_in_sprite - 1) as u16
      } else {
        y_pos_in_sprite as u16
      };

      let x_pos_in_tile = x_pos_in_sprite % 8;
      let y_pos_in_tile = y_pos_in_sprite % 8;

      let tile_address = tile_base + (x_pos_in_sprite / 8 + (y_pos_in_sprite as u32 / 8) * tile_width) * tile_size;

      let palette_index = if obj_attributes.palette_flag {
        self.get_pixel_index_bpp8(tile_address, x_pos_in_tile as u16, y_pos_in_tile, false, false)
      } else {
        self.get_pixel_index_bpp4(tile_address, x_pos_in_tile as u16, y_pos_in_tile, false, false)
      };

      if palette_index != 0 {
        self.obj_lines[obj_line_index] = ObjectPixel {
          priority: obj_attributes.priority,
          color: self.get_palette_color(palette_index as usize, palette_bank as usize, 0x200),
          is_window: obj_attributes.obj_mode == 2,
          is_transparent: obj_attributes.obj_mode == 1
        };
      }
    }
  }

  fn render_affine_object(&mut self, obj_attributes: OamAttributes) {
    let y = self.vcount;

    let (obj_width, obj_height) = obj_attributes.get_object_dimensions();

    let (x_coordinate, y_coordinate) = self.get_obj_coordinates(obj_attributes.x_coordinate, obj_attributes.y_coordinate);

    let (bbox_width, bbox_height) = if obj_attributes.double_sized_flag {
      (2 * obj_width, 2 * obj_height)
    } else {
      (obj_width, obj_height)
    };

    let y_pos_in_sprite = y as i16 - y_coordinate;

    if y_pos_in_sprite < 0 || y_pos_in_sprite as u32 >= bbox_height || obj_attributes.obj_mode == 3 {
      return;
    }

    let tile_number = obj_attributes.tile_number;
    let tile_base: u32 = 0x1_0000 + tile_number as u32 * 32;

    if tile_base < self.vram_obj_start {
      return;
    }

    let tile_size = if obj_attributes.palette_flag {
      64
    } else {
      32
    };

    let tile_width = if self.dispcnt.contains(DisplayControlRegister::OBJ_CHARACTER_MAPPING) {
      obj_width / 8
    } else {
      if obj_attributes.palette_flag {
        16
      } else {
        32
      }
    };

    let palette_bank = if !obj_attributes.palette_flag {
      obj_attributes.palette_number
    } else {
      0
    };

    // get affine matrix
    let (dx, dmx, dy, dmy) = self.get_obj_affine_params(obj_attributes.rotation_param_selection);

    let half_height = bbox_height / 2;
    let half_width: i16 = bbox_width as i16 / 2;

    let iy = y as i16 - (y_coordinate + half_height as i16);

    for ix in (-half_width)..(half_width) {
      let x = x_coordinate + half_width + ix;

      if x < 0 {
        continue;
      }

      if x as u16 >= SCREEN_WIDTH {
        break;
      }

      let obj_line_index =(x as u16 + y * SCREEN_WIDTH) as usize;

      if self.obj_lines[obj_line_index].priority <= obj_attributes.priority && obj_attributes.obj_mode != 2 {
        continue;
      }

      let transformed_x = (dx * ix + dmx * iy) >> 8;
      let transformed_y = (dy * ix + dmy * iy) >> 8;

      let texture_x = transformed_x + obj_width as i16 / 2;
      let texture_y = transformed_y + obj_height as i16 / 2;

      if texture_x >= 0 && texture_x < obj_width as i16 && texture_y >= 0 && texture_y < obj_height as i16 {
        // finally queue the pixel!

        let tile_x = texture_x % 8;
        let tile_y = texture_y % 8;

        let tile_address = tile_base + (texture_x as u32 / 8 + (texture_y as u32 / 8) * tile_width) * tile_size;

        let palette_index = if obj_attributes.palette_flag {
          self.get_pixel_index_bpp8(tile_address, tile_x as u16, tile_y as u16, false, false)
        } else {
          self.get_pixel_index_bpp4(tile_address, tile_x as u16, tile_y as u16, false, false)
        };

        let color = self.get_palette_color(palette_index as usize, palette_bank as usize, 0x200);

        if palette_index != 0 {
          self.obj_lines[obj_line_index] = ObjectPixel {
            priority: obj_attributes.priority,
            color,
            is_window: obj_attributes.obj_mode == 2,
            is_transparent: obj_attributes.obj_mode == 1
          }
        }
      }
    }

  }

  fn get_obj_coordinates(&mut self, x: u16, y: u16) -> (i16, i16) {
    let return_x: i16 = if x >= SCREEN_WIDTH {
      x as i16 - 512
    } else {
      x as i16
    };

    let return_y: i16 = if y >= SCREEN_HEIGHT {
      y as i16 - 256
    } else {
      y as i16
    };

    (return_x, return_y)
  }

  fn get_obj_affine_params(&self, affine_index: u16) -> (i16, i16, i16, i16) {
    let mut offset = affine_index * 32 + AFFINE_SIZE;

    let dx = self.oam_read_16(offset as usize) as i16;
    offset += 2 + AFFINE_SIZE;
    let dmx = self.oam_read_16(offset as usize) as i16;
    offset += 2 + AFFINE_SIZE;
    let dy = self.oam_read_16(offset as usize) as i16;
    offset += 2 + AFFINE_SIZE;
    let dmy = self.oam_read_16(offset as usize) as i16;

    (dx, dmx, dy, dmy)
  }

  fn get_attributes(&self, i: usize) -> OamAttributes {
    let oam_address = i * ATTRIBUTE_SIZE;

    let attribute1 = self.oam_read_16(oam_address);
    let attribute2 = self.oam_read_16(oam_address + 2);
    let attribute3 = self.oam_read_16(oam_address + 4);

    let y_coordinate = attribute1 & 0xff;
    let rotation_flag = (attribute1 >> 8) & 0b1 == 1;
    let double_sized_flag = rotation_flag && (attribute1 >> 9) & 0b1 == 1;
    let obj_disable = !rotation_flag && (attribute1 >> 9) & 0b1 == 1;
    let obj_mode = (attribute1 >> 10) & 0b11;
    let obj_mosaic = (attribute1 >> 12) & 0b1 == 1;
    let palette_flag = (attribute1 >> 13) & 0b1 == 1;
    let obj_shape = (attribute1 >> 14) & 0b11;

    let x_coordinate = attribute2 & 0x1ff;
    let rotation_param_selection = if rotation_flag {
      (attribute2 >> 9) & 0b11111
    } else {
      0
    };
    let horizontal_flip = !rotation_flag && (attribute2 >> 12) & 0b1 == 1;
    let vertical_flip = !rotation_flag && (attribute2 >> 13) & 0b1 == 1;
    let obj_size = (attribute2 >> 14) & 0b11;

    let tile_number = attribute3 & 0b1111111111;
    let priority = (attribute3 >> 10) & 0b11;
    let palette_number = (attribute3 >> 12) & 0xf;

    OamAttributes {
      y_coordinate,
      rotation_flag,
      double_sized_flag,
      obj_disable,
      obj_mode,
      obj_mosaic,
      palette_flag,
      obj_shape,
      x_coordinate,
      rotation_param_selection,
      horizontal_flip,
      vertical_flip,
      obj_size,
      tile_number,
      priority,
      palette_number
    }

  }

  fn oam_read_16(&self, address: usize) -> u16 {
    (self.oam_ram[address] as u16) | (self.oam_ram[address + 1] as u16) << 8
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
}