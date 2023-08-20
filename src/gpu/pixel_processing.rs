use std::cmp;

use super::{GPU, registers::{display_control_register::DisplayControlRegister, color_effects_register::ColorEffect, window_out_register::WindowOutRegister, window_in_register::WindowInRegister}, SCREEN_WIDTH, WindowType};


impl GPU {
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
        self.process_pixel(x, &mut color, bottom_layer);
      }

      self.picture.set_pixel(x, y as usize, self.translate_to_rgb24(color));
    } else if let Some(mut color) = self.obj_lines[obj_line_index].color {
      // render object pixel
      if self.obj_lines[obj_line_index].is_transparent && bottom_layer != -1 && self.bldcnt.bg_second_pixels[bottom_layer as usize] {
        if let Some(color2) = self.bg_lines[bottom_layer as usize][x as usize] {
          color = self.blend_colors(color, color2, self.bldalpha.eva as u16, self.bldalpha.evb as u16);
        }
      }
      self.picture.set_pixel(x, y as usize, self.translate_to_rgb24(color));
    }
    else {
      self.picture.set_pixel(x, y as usize, self.translate_to_rgb24(default_color.unwrap()));
    }
  }

  fn process_pixel(&mut self, x: usize, color: &mut (u8, u8, u8), bottom_layer: isize) {
    match self.bldcnt.color_effect {
      ColorEffect::AlphaBlending => {
        let blend_layer = if self.is_bottom_layer_blended(bottom_layer)  {
          bottom_layer
        } else {
          -1
        };

        if blend_layer != -1 {
          let color2 = if blend_layer < 4 {
            self.bg_lines[blend_layer as usize][x]
          } else {
            let obj_index: usize = x + self.vcount as usize * SCREEN_WIDTH as usize;
            self.obj_lines[obj_index].color
          };

          if let Some(color2) = color2 {
            *color = self.blend_colors(*color, color2, self.bldalpha.eva as u16, self.bldalpha.evb as u16);
          }
        }
      }
      ColorEffect::Darken => {
        // blending with black
        let color2: (u8, u8, u8) = (0, 0, 0);

        *color = self.blend_colors(*color, color2, (16 - self.bldy.evy) as u16, self.bldy.evy as u16);
      }
      ColorEffect::Brighten => {
        // blending with white
        let color2: (u8, u8, u8) = (31, 31, 31);

        *color = self.blend_colors(*color, color2, (16 - self.bldy.evy) as u16, self.bldy.evy as u16);
      }
      ColorEffect::None => ()
    }
  }

  fn is_bottom_layer_blended(&self, bottom_layer: isize) -> bool {
    (bottom_layer < 4 && bottom_layer >= 0 && self.bldcnt.bg_second_pixels[bottom_layer as usize]) || (bottom_layer == 4 && self.bldcnt.obj_second_pixel)
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

  pub fn render_scanline(&mut self) {
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

  fn bg_enabled(&self, bg_index: usize) -> bool {
    match bg_index {
      0 => self.dispcnt.contains(DisplayControlRegister::DISPLAY_BG0),
      1 => self.dispcnt.contains(DisplayControlRegister::DISPLAY_BG1),
      2 => self.dispcnt.contains(DisplayControlRegister::DISPLAY_BG2),
      3 => self.dispcnt.contains(DisplayControlRegister::DISPLAY_BG3),
      _ => panic!("shouldn't happen")
    }
  }
}