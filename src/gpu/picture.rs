use serde::{Deserialize, Serialize};

use super::{SCREEN_WIDTH, SCREEN_HEIGHT};

#[derive(Serialize, Deserialize)]
pub struct Picture {
  pub data: Vec<u8>
}

impl Picture {
  pub fn set_pixel(&mut self, x: usize, y: usize, rgb: (u8, u8, u8)) {
    // let i: usize = x * 3 + y * SCREEN_WIDTH as usize * 3;
    let i: usize = 4 * (x + y * SCREEN_WIDTH as usize);

    self.data[i] = rgb.0;
    self.data[i+1] = rgb.1;
    self.data[i+2] = rgb.2;
    self.data[i+3] = 0xff;
  }

  pub fn new() -> Self {
    Picture {
      data: vec![0; (4 * SCREEN_WIDTH as u32 * SCREEN_HEIGHT as u32) as usize]
    }
  }
}