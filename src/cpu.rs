
// thumb state SP maps onto ARM r13
// thumb state LR maps onto ARM r14

// in THUMB mode, bit 0 of PC is 0
// in ARM mode, bits 0-1 of PC are 0

pub mod arm_opcodes;
pub mod thumb_opcodes;

pub const PC_REGISTER: usize = 15;
pub const LR_REGISTER: usize = 14;
pub const SP_REGISTER: usize = 13;

pub const SOFTWARE_INTERRUPT_VECTOR: u32 = 0x8;

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
  spsr_banks: [PSRRegister; 6],
  thumb_lut: Vec<fn(&mut CPU, instruction: u16)>,
  arm_lut: Vec<fn(&mut CPU, instruction: u32)>,
  pipeline: [u32; 2],
  rom: Vec<u8>,
  is_init: bool
}


#[derive(Clone, Copy)]
pub enum OperatingMode {
  User = 0b10000,
  FIQ = 0b10001,
  IRQ = 0b10010,
  Supervisor = 0b10011,
  Abort = 0b10111,
  Undefined = 0b11011,
  System = 0b11111
}

impl OperatingMode {
  pub fn bank_index(&self) -> usize {
    match self {
      OperatingMode::User | OperatingMode::System => 0,
      OperatingMode::FIQ => 1,
      OperatingMode::IRQ => 2,
      OperatingMode::Supervisor => 3,
      OperatingMode::Abort => 4,
      OperatingMode::Undefined => 5,
    }
  }
}

bitflags! {
  #[derive(Copy, Clone)]
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
    let mut cpu = Self {
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
      spsr_banks: [PSRRegister::new(); 6],
      thumb_lut: Vec::new(),
      arm_lut: Vec::new(),
      pipeline: [0; 2],
      rom: Vec::new(),
      is_init: true
    };

    cpu.populate_thumb_lut();
    cpu.populate_arm_lut();

    cpu
  }

  pub fn set_mode(&mut self, new_mode: OperatingMode) {
    let old_mode = self.cpsr.mode();

    let new_index = new_mode.bank_index();
    let old_index = old_mode.bank_index();

    if new_index == old_index {
      return;
    }

    // save contents of cpsr and banked registers
    self.spsr_banks[old_index] = self.spsr;
    self.r13_banks[old_index] = self.r[13];
    self.r14_banks[old_index] = self.r[14];

    let new_cpsr = (self.cpsr.bits() & !(0b11111)) | (new_mode as u32);

    self.spsr = self.spsr_banks[new_index];
    self.r[13] = self.r13_banks[new_index];
    self.r[14] = self.r14_banks[new_index];

    if matches!(new_mode, OperatingMode::FIQ) {
      self.r8_banks[0] = self.r[8];
      self.r9_banks[0] = self.r[9];
      self.r10_banks[0] = self.r[10];
      self.r11_banks[0] = self.r[11];
      self.r12_banks[0] = self.r[12];

      self.r[8] = self.r8_banks[1];
      self.r[9] = self.r9_banks[1];
      self.r[10] = self.r10_banks[1];
      self.r[11] = self.r11_banks[1];
      self.r[12] = self.r12_banks[1];
    } else if matches!(old_mode, OperatingMode::FIQ) {
      self.r8_banks[1] = self.r[8];
      self.r9_banks[1] = self.r[9];
      self.r10_banks[1] = self.r[10];
      self.r11_banks[1] = self.r[11];
      self.r12_banks[1] = self.r[12];

      self.r[8] = self.r8_banks[0];
      self.r[9] = self.r9_banks[0];
      self.r[10] = self.r10_banks[0];
      self.r[11] = self.r11_banks[0];
      self.r[12] = self.r12_banks[0];
    }

    self.cpsr = PSRRegister::from_bits_truncate(new_cpsr);
  }

  pub fn skip_bios(&mut self) {
    self.r13_banks[0] = 0x0300_7f00; // USR/SYS
    self.r13_banks[1] = 0x0300_7f00; // FIQ
    self.r13_banks[2] = 0x0300_7fa0; // IRQ
    self.r13_banks[3] = 0x0300_7fe0; // SVC
    self.r13_banks[4] = 0x0300_7f00; // ABT
    self.r13_banks[5] = 0x0300_7f00; // UND
    self.r[13] = 0x0300_7f00;
    self.pc = 0x0800_0000;
    self.cpsr = PSRRegister::from_bits_truncate(0x5f);
  }

  pub fn load_game(&mut self, rom: Vec<u8>) {
    self.rom = rom;
  }

  pub fn execute_thumb(&mut self, instr: u16) {
    let handler_fn = self.thumb_lut[(instr >> 8) as usize];

    println!("executing an instruction!");

    handler_fn(self, instr);
  }

  pub fn execute_arm(&mut self, instr: u32) {
    let handler_fn = self.arm_lut[(((instr >> 16) & 0xff0) | ((instr >> 4) & 0xf)) as usize];

    handler_fn(self, instr);
  }

  pub fn step_arm(&mut self) {
    let pc = self.pc & !(0b11);

    let next_instruction = if self.is_init {
      self.pipeline[0] = self.mem_read_32(pc) as u32;
      self.pipeline[1] = self.mem_read_32(pc + 4);

      None
    } else {
      Some(self.mem_read_32(pc))
    };

    let instruction = self.pipeline[0];
    self.pipeline[0] = self.pipeline[1];

    if let Some(instr) = next_instruction {
      self.pipeline[1] = instr;
    }

    let condition = (instruction >> 28) as u8;

    if self.arm_condition_met(condition) {
      self.execute_arm(instruction);
    }

    self.pc = self.pc.wrapping_add(4);
  }

  fn arm_condition_met(&self, condition: u8) -> bool {
    match condition {
      0 => self.cpsr.contains(PSRRegister::ZERO),
      1 => !self.cpsr.contains(PSRRegister::ZERO),
      2 => self.cpsr.contains(PSRRegister::CARRY),
      3 => !self.cpsr.contains(PSRRegister::CARRY),
      4 => self.cpsr.contains(PSRRegister::NEGATIVE),
      5 => !self.cpsr.contains(PSRRegister::NEGATIVE),
      6 => self.cpsr.contains(PSRRegister::OVERFLOW),
      7 => !self.cpsr.contains(PSRRegister::OVERFLOW),
      8 => self.cpsr.contains(PSRRegister::CARRY) && !self.cpsr.contains(PSRRegister::ZERO),
      9 => !self.cpsr.contains(PSRRegister::CARRY) || self.cpsr.contains(PSRRegister::ZERO),
      10 => self.cpsr.contains(PSRRegister::NEGATIVE) == self.cpsr.contains(PSRRegister::OVERFLOW),
      11 => self.cpsr.contains(PSRRegister::NEGATIVE) != self.cpsr.contains(PSRRegister::OVERFLOW),
      12 => !self.cpsr.contains(PSRRegister::ZERO) && self.cpsr.contains(PSRRegister::NEGATIVE) == self.cpsr.contains(PSRRegister::OVERFLOW),
      13 => self.cpsr.contains(PSRRegister::ZERO) || self.cpsr.contains(PSRRegister::NEGATIVE) != self.cpsr.contains(PSRRegister::OVERFLOW),
      14 => true,
      _ => panic!("shouldn't happen")
    }
  }

  pub fn step_thumb(&mut self) {
    let pc = self.pc & !(0b1);

    let next_instruction = if self.is_init {
      self.pipeline[0] = self.mem_read_16(pc) as u32;
      self.pipeline[1] = self.mem_read_16(pc + 2) as u32;
      self.is_init = false;

      None
    } else {
      Some(self.mem_read_16(pc) as u32)
    };

    let instruction = self.pipeline[0];
    self.pipeline[0] = self.pipeline[1];

    if let Some(instr) = next_instruction {
      self.pipeline[1] = instr;
    }

    self.execute_thumb(instruction as u16);

    self.pc = self.pc.wrapping_add(2);
  }

  pub fn mem_read_32(&mut self, address: u32) -> u32 {
    self.mem_read_16(address) as u32 | ((self.mem_read_16(address + 2) as u32) << 16)
  }

  pub fn mem_read_16(&mut self, address: u32) -> u16 {
    self.mem_read_8(address) as u16 | ((self.mem_read_8(address + 1) as u16) << 8)
  }

  pub fn mem_read_8(&mut self, address: u32) -> u8 {
    match address {
      0x8_000_000..=0xD_FFF_FFF => self.rom[(address - 0x8_000_000) as usize],
      _ => 0
    }
  }

  pub fn mem_write_32(&mut self, address: u32, val: u32) {

  }

  pub fn mem_write_16(&mut self, address: u32, val: u16) {

  }

  pub fn mem_write_8(&mut self, address: u32, val: u8) {

  }

  pub fn reload_pipeline16(&mut self) {
    self.pipeline[0] = self.mem_read_16(self.pc) as u32;

    self.pc += 2;

    self.pipeline[1] = self.mem_read_16(self.pc) as u32;

    self.pc += 2;
  }

  pub fn push(&mut self, val: u32) {
    self.r[SP_REGISTER] -= 4;

    self.mem_write_32(self.r[SP_REGISTER], val);
  }

  pub fn pop(&mut self) -> u32 {
    let val = self.mem_read_32(self.r[SP_REGISTER]);

    self.r[SP_REGISTER] += 4;

    val
  }
}