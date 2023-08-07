// general comments

// per the ARM7tdmi manual,
// in ARM state, bits [1:0] of
// R15 are zero and bits [31:2] contain the PC. In THUMB state,
// bit [0] is zero and bits [31:1] contain the PC.

use crate::gpu::GPU;

use self::cycle_lookup_tables::CycleLookupTables;

pub mod arm_opcodes;
pub mod thumb_opcodes;
pub mod cycle_lookup_tables;
pub mod bus;

pub const PC_REGISTER: usize = 15;
pub const LR_REGISTER: usize = 14;
pub const SP_REGISTER: usize = 13;

pub const SOFTWARE_INTERRUPT_VECTOR: u32 = 0x8;

#[derive(Clone, Copy)]
pub enum MemoryAccess {
  Sequential,
  NonSequential
}

enum MemoryWidth {
  Width8,
  Width16,
  Width32
}

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
  post_flag: u16,
  spsr: PSRRegister,
  cpsr: PSRRegister,
  spsr_banks: [PSRRegister; 6],
  thumb_lut: Vec<fn(&mut CPU, instruction: u16) -> Option<MemoryAccess>>,
  arm_lut: Vec<fn(&mut CPU, instruction: u32) -> Option<MemoryAccess>>,
  pipeline: [u32; 2],
  bios: Vec<u8>,
  board_wram: [u8; 256 * 1024],
  chip_wram: [u8; 32 * 1024],
  rom: Vec<u8>,
  next_fetch: MemoryAccess,
  cycle_luts: CycleLookupTables,
  pub gpu: GPU,
  cycles: u32
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
    Self::from_bits_retain(0)
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
      _ => panic!("unknown mode specified: {:b}", self.bits() & 0b11111)
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
      spsr: PSRRegister::from_bits_retain(0),
      cpsr: PSRRegister::from_bits_retain(0xd3),
      spsr_banks: [PSRRegister::new(); 6],
      thumb_lut: Vec::new(),
      arm_lut: Vec::new(),
      pipeline: [0; 2],
      rom: Vec::new(),
      bios: Vec::new(),
      next_fetch: MemoryAccess::NonSequential,
      board_wram: [0; 256 * 1024],
      chip_wram: [0; 32 * 1024],
      post_flag: 0,
      gpu: GPU::new(),
      cycle_luts: CycleLookupTables::new(),
      cycles: 0
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

    self.cpsr = PSRRegister::from_bits_retain(new_cpsr);
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
    self.cpsr = PSRRegister::from_bits_retain(0x5f);
  }

  pub fn load_game(&mut self, rom: Vec<u8>) {
    self.rom = rom;
  }

  pub fn execute_thumb(&mut self, instr: u16) -> Option<MemoryAccess> {
    let handler_fn = self.thumb_lut[(instr >> 8) as usize];

    handler_fn(self, instr)
  }

  pub fn execute_arm(&mut self, instr: u32) -> Option<MemoryAccess> {
    let handler_fn = self.arm_lut[(((instr >> 16) & 0xff0) | ((instr >> 4) & 0xf)) as usize];

    handler_fn(self, instr)
  }

  fn step_arm(&mut self) {
    let pc = self.pc & !(0b11);

    let next_instruction = self.load_32(pc, self.next_fetch);

    let instruction = self.pipeline[0];
    self.pipeline[0] = self.pipeline[1];
    self.pipeline[1] = next_instruction;

    let condition = (instruction >> 28) as u8;

    println!("attempting to execute instruction {:032b} at address {:X}", instruction, pc.wrapping_sub(8));

    if self.arm_condition_met(condition) {
      if let Some(access) = self.execute_arm(instruction) {
        self.next_fetch = access;
      }
    } else {
      self.pc = self.pc.wrapping_add(4);
      self.next_fetch = MemoryAccess::NonSequential;
    }
  }

  fn arm_condition_met(&self, condition: u8) -> bool {
    println!("condition is {condition}");
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

  pub fn step(&mut self) -> u32 {
    println!("r0 = {:X}, r3 = {:X}", self.r[0], self.r[3]);
    if self.cpsr.contains(PSRRegister::STATE_BIT) {
      self.step_thumb();
    } else {
      self.step_arm();
    }

    let return_cycles = self.cycles;
    self.cycles = 0;

    return_cycles
  }

  fn step_thumb(&mut self) {
    let pc = self.pc & !(0b1);

    let next_instruction = self.load_16(pc, self.next_fetch) as u32;

    let instruction = self.pipeline[0];
    self.pipeline[0] = self.pipeline[1];
    self.pipeline[1] = next_instruction;

    println!("executing instruction {:016b} at address {:X}", instruction, pc.wrapping_sub(4));

    if let Some(fetch) = self.execute_thumb(instruction as u16) {
      self.next_fetch = fetch;
    }
  }

  fn get_register(&self, r: usize) -> u32 {
    if r == PC_REGISTER {
      self.pc
    } else {
      self.r[r]
    }
  }

  pub fn load_32(&mut self, address: u32, access: MemoryAccess) -> u32 {
    self.update_cycles(address, access, MemoryWidth::Width16);
    self.mem_read_32(address)
  }

  pub fn load_16(&mut self, address: u32, access: MemoryAccess) -> u16 {
    self.update_cycles(address, access, MemoryWidth::Width32);
    self.mem_read_16(address)
  }

  pub fn load_8(&mut self, address: u32, access: MemoryAccess) -> u8 {
    self.update_cycles(address, access, MemoryWidth::Width8);
    self.mem_read_8(address)
  }

  pub fn store_8(&mut self, address: u32, value: u8, access: MemoryAccess) {
    self.update_cycles(address, access, MemoryWidth::Width8);
    self.mem_write_8(address, value);
  }

  pub fn store_16(&mut self, address: u32, value: u16, access: MemoryAccess) {
    self.update_cycles(address, access, MemoryWidth::Width8);
    self.mem_write_16(address, value);
  }

  pub fn store_32(&mut self, address: u32, value: u32, access: MemoryAccess) {
    self.update_cycles(address, access, MemoryWidth::Width8);
    self.mem_write_32(address, value);
  }

  fn update_cycles(&mut self, address: u32,  access: MemoryAccess, width: MemoryWidth) {
    let page = ((address >> 24) & 0xf) as usize;
    let cycles = match width {
      MemoryWidth::Width8 | MemoryWidth::Width16 => match access {
        MemoryAccess::NonSequential => self.cycle_luts.n_cycles_16[page],
        MemoryAccess::Sequential => self.cycle_luts.s_cycles_16[page]
      }
      MemoryWidth::Width32 => match access {
        MemoryAccess::NonSequential => self.cycle_luts.n_cycles_32[page],
        MemoryAccess::Sequential => self.cycle_luts.s_cycles_32[page],
      }
    };

    self.add_cycles(cycles);
  }

  fn add_cycles(&mut self, cycles: u32) {
    self.cycles += cycles;
    self.gpu.tick(cycles);
  }

  pub fn reload_pipeline16(&mut self) {
    self.pc = self.pc & !(0b1);
    self.pipeline[0] = self.load_16(self.pc, MemoryAccess::NonSequential) as u32;

    self.pc = self.pc.wrapping_add(2);

    self.pipeline[1] = self.load_16(self.pc, MemoryAccess::Sequential) as u32;

    self.pc = self.pc.wrapping_add(2);
  }

  pub fn reload_pipeline32(&mut self) {
    self.pc = self.pc & !(0b11);
    self.pipeline[0] = self.load_32(self.pc, MemoryAccess::NonSequential);

    self.pc = self.pc.wrapping_add(4);

    self.pipeline[1] = self.load_32(self.pc, MemoryAccess::Sequential);

    self.pc = self.pc.wrapping_add(4);
  }

  pub fn interrupt(&mut self) {
    let supervisor_bank = OperatingMode::Supervisor.bank_index();

    self.r14_banks[supervisor_bank] = if self.cpsr.contains(PSRRegister::STATE_BIT) { self.pc - 2 } else { self.pc - 4 };
    self.spsr_banks[supervisor_bank] = self.cpsr;

    self.set_mode( OperatingMode::Supervisor);

    // change to ARM state
    self.cpsr.remove(PSRRegister::STATE_BIT);

    self.cpsr.insert(PSRRegister::IRQ_DISABLE);

    self.pc = SOFTWARE_INTERRUPT_VECTOR;

    // reload pipeline
    self.reload_pipeline32();

  }

  pub fn push(&mut self, val: u32, access: MemoryAccess) {
    self.r[SP_REGISTER] -= 4;

    println!("pushing {val} to address {:X}", self.r[SP_REGISTER] & !(0b11));

    self.store_32(self.r[SP_REGISTER] & !(0b11), val, access);
  }

  pub fn pop(&mut self, access: MemoryAccess) -> u32 {
    let val = self.load_32(self.r[SP_REGISTER] & !(0b11), access);

    println!("popping {val} from address {:X}", self.r[SP_REGISTER] & !(0b11));

    self.r[SP_REGISTER] += 4;

    val
  }

  pub fn ldr_halfword(&mut self, address: u32) -> u16 {
    if address & 0b1 != 0 {
      let rotation = (address & 0b1) << 3;

      let value = self.load_16(address & !(0b1), MemoryAccess::NonSequential);

      let mut carry = self.cpsr.contains(PSRRegister::CARRY);
      let return_val = self.ror(value as u32, rotation as u8, &mut carry) as u16;

      self.cpsr.set(PSRRegister::CARRY, carry);

      return_val
    } else {
      self.load_16(address, MemoryAccess::NonSequential)
    }
  }

  fn ldr_word(&mut self, address: u32) -> u32 {
    if address & (0b11) != 0 {
      let rotation = (address & 0b11) << 3;

      let value = self.load_32(address & !(0b11), MemoryAccess::NonSequential);

      let mut carry = self.cpsr.contains(PSRRegister::CARRY);

      let return_val = self.ror(value, rotation as u8, &mut carry);

      self.cpsr.set(PSRRegister::CARRY, carry);

      return_val
    } else {
      self.load_32(address, MemoryAccess::NonSequential)
    }
  }

  pub fn load_bios(&mut self, bytes: Vec<u8>) {
    self.bios = bytes;
  }

  pub fn ror(&mut self, immediate: u32, amount: u8, carry: &mut bool) -> u32 {
    if amount != 0 {
      let amount = amount % 32;

      let result = immediate.rotate_right(amount as u32);

      *carry = (result >> 31) & 0b1 == 1;

      println!("carry is {carry}");

      result
    } else {
      immediate
    }
  }

  pub fn get_multiplier_cycles(&self, operand: u32) -> u32 {
    if operand & 0xff == operand {
      1
    } else if operand & 0xffff == operand {
      2
    } else if operand & 0xffffff == operand {
      3
    } else {
      4
    }
  }

}