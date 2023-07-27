
// thumb state SP maps onto ARM r13
// thumb state LR maps onto ARM r14

pub mod arm_opcodes;
pub mod thumb_opcodes;

pub const PC_REGISTER: u8 = 15;
pub const SP_REGISTER: u8 = 13;

pub struct CPU {
  r: [u32; 15],
  pc: u32,
  r8_banks: [u32; 2],
  r9_banks: [u32; 2],
  r10_banks: [u32; 2],
  r11_banks: [u32; 2],
  r12_banks: [u32; 2],
  r13_banks: [u32; 6],
  r14_banks: [u32; 6],
  spsr: PSRRegister,
  cpsr: PSRRegister,
  spsr_banks: [u32; 5],
  thumb_lut: Vec<fn(&mut CPU, instruction: u16)>
}


pub enum OperatingMode {
  User,
  FIQ,
  IRQ,
  Supervisor,
  Abort,
  Undefined,
  System
}


bitflags! {
  pub struct PSRRegister: u32 {
    const STATE_BIT = 0b1 << 5;
    const FIQ_DISABLE = 0b1 << 6;
    const IRQ_DISABLE = 0b1 << 7;
    const OVERFLOW = 0b1 << 28;
    const CARRY = 0b1 << 29;
    const ZERO = 0b1 << 30;
    const NEGATIVE = 0b1 << 31;
  }
}

impl PSRRegister {
  pub fn new() -> Self {
    Self::from_bits_truncate(0)
  }

  pub fn mode(&self) -> OperatingMode {
    match self.bits() & 0b11111 {
      0b10000 => OperatingMode::User,
      0b10001 => OperatingMode::FIQ,
      0b10010 => OperatingMode::IRQ,
      0b10011 => OperatingMode::Supervisor,
      0b10111 => OperatingMode::Abort,
      0b11011 => OperatingMode::Undefined,
      0b11111 => OperatingMode::System,
      _ => panic!("unknown mode specified: {:b}", self.bits())
    }
  }
}

impl CPU {
  pub fn new() -> Self {
    Self {
      r: [0; 15],
      pc: 0,
      r8_banks: [0; 2],
      r9_banks: [0; 2],
      r10_banks: [0; 2],
      r11_banks: [0; 2],
      r12_banks: [0; 2],
      r13_banks: [0; 6],
      r14_banks: [0; 6],
      spsr: PSRRegister::new(),
      cpsr: PSRRegister::new(),
      spsr_banks: [0; 5],
      thumb_lut: Vec::new()
    }
  }

  pub fn execute(&mut self, instr: u16) {
    let handler_fn = self.thumb_lut[(instr >> 8) as usize];

    handler_fn(self, instr);
  }

  pub fn mem_read_32(&mut self, address: u32) -> u32 {
    0
  }

  pub fn mem_read_16(&mut self, address: u32) -> u16 {
    0
  }

  pub fn mem_read_8(&mut self, address: u32) -> u8 {
    0
  }

  pub fn mem_write_32(&mut self, address: u32, val: u32) {

  }

  pub fn mem_write_16(&mut self, address: u32, val: u16) {

  }

  pub fn mem_write_8(&mut self, address: u32, val: u8) {

  }
}