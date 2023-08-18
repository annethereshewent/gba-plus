const FIFO_CAPACITY: usize = 32; // 32 signed bytes

pub struct DmaFifo {
  pub data: [i8; FIFO_CAPACITY],
  pub value: i8, // current value read from buffer
  write_pointer: usize,
  read_pointer: usize,
  pub count: u32
}

impl DmaFifo {
  pub fn new() -> Self {
    Self {
      data: [0; FIFO_CAPACITY],
      read_pointer: 0,
      write_pointer: 0,
      value: 0,
      count: 0
    }
  }

  pub fn reset(&mut self) {
    self.write_pointer = 0;
    self.read_pointer = 0;
    self.count = 0;
  }

  pub fn write(&mut self, val: i8) {
    if self.count < FIFO_CAPACITY as u32 {
      self.data[self.write_pointer] = val;
      self.count += 1;

      self.write_pointer = (self.write_pointer + 1) % FIFO_CAPACITY;
    }
  }

  pub fn read(&mut self) -> i8 {
    if self.count == 0 {
      return 0;
    }

    let val = self.data[self.read_pointer];
    self.read_pointer = (self.read_pointer + 1) % FIFO_CAPACITY;
    self.count -= 1;

    val
  }
}