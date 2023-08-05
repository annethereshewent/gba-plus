use super::SCREEN_WIDTH;

pub struct Picture {
  pub data: Vec<u8>
}

impl Picture {
  pub fn set_pixel(&mut self, x: usize, y: usize, rgb: (u8, u8, u8)) {
    let i: usize = x * 3 + y * SCREEN_WIDTH as usize * 3;

    self.data[i] = rgb.0;
    self.data[i+1] = rgb.1;
    self.data[i+2] = rgb.2;
  }

  pub fn new() -> Self {
    Picture {
      data: vec![0; (3 * 240 * 256) as usize]
    }
  }
}