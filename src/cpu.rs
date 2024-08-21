// general comments

// per the ARM7tdmi manual,
// in ARM state, bits [1:0] of
// R15 are zero and bits [31:2] contain the PC. In THUMB state,
// bit [0] is zero and bits [31:1] contain the PC.

use std::{
  rc::Rc,
  cell::Cell
};

use crate::{
  apu::APU,
  cartridge::{
    BackupMedia,
    Cartridge
  },
  gpu::{
    GPU,
    HDRAW_CYCLES
  },
  scheduler::{
    EventType,
    Scheduler
  }
};

use self::{
  cycle_lookup_tables::CycleLookupTables,
  registers::{
    interrupt_enable_register::InterruptEnableRegister,
    interrupt_request_register::InterruptRequestRegister,
    key_input_register::KeyInputRegister,
    waitstate_control_register::WaitstateControlRegister
  },
  dma::dma_channels::DmaChannels,
  timers::Timers
};

pub mod arm_instructions;
pub mod thumb_instructions;
pub mod cycle_lookup_tables;
pub mod bus;
pub mod rotations_shifts;
pub mod registers;
pub mod dma;
pub mod timers;

pub const PC_REGISTER: usize = 15;
pub const LR_REGISTER: usize = 14;
pub const SP_REGISTER: usize = 13;

pub const SOFTWARE_INTERRUPT_VECTOR: u32 = 0x8;
pub const IRQ_VECTOR: u32 = 0x18;

pub const CPU_CLOCK_SPEED: u32 = 2u32.pow(24);

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
  interrupt_master_enable: bool,
  spsr: PSRRegister,
  pub cpsr: PSRRegister,
  spsr_banks: [PSRRegister; 6],
  thumb_lut: Vec<fn(&mut CPU, instruction: u16) -> Option<MemoryAccess>>,
  arm_lut: Vec<fn(&mut CPU, instruction: u32) -> Option<MemoryAccess>>,
  pipeline: [u32; 2],
  bios: Vec<u8>,
  board_wram: Box<[u8]>,
  chip_wram: Box<[u8]>,
  pub cartridge: Cartridge,
  next_fetch: MemoryAccess,
  cycle_luts: CycleLookupTables,
  pub gpu: GPU,
  interrupt_enable: InterruptEnableRegister,
  interrupt_request: Rc<Cell<InterruptRequestRegister>>,
  waitcnt: WaitstateControlRegister,
  is_halted: bool,
  dma_channels: Rc<Cell<DmaChannels>>,
  pub key_input: KeyInputRegister,
  pub timers: Timers,
  pub apu: APU,
  pub scheduler: Scheduler,
  cycles: usize
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
    let interrupt_request = Rc::new(Cell::new(InterruptRequestRegister::from_bits_retain(0)));
    let dma_channels = Rc::new(Cell::new(DmaChannels::new()));

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
      spsr: PSRRegister::from_bits_retain(0xd3),
      cpsr: PSRRegister::from_bits_retain(0xd3),
      spsr_banks: [PSRRegister::from_bits_retain(0xd3); 6],
      thumb_lut: Vec::new(),
      arm_lut: Vec::new(),
      pipeline: [0; 2],
      cartridge: Cartridge {
        rom: Vec::new(),
        file_path: None,
        backup: BackupMedia::Undetected
      },
      bios: Vec::new(),
      next_fetch: MemoryAccess::NonSequential,
      board_wram: vec![0; 256 * 1024].into_boxed_slice(),
      chip_wram: vec![0; 32 * 1024].into_boxed_slice(),
      post_flag: 0,
      gpu: GPU::new(interrupt_request.clone(), dma_channels.clone()),
      interrupt_request: interrupt_request.clone(),
      cycle_luts: CycleLookupTables::new(),
      interrupt_master_enable: false,
      interrupt_enable: InterruptEnableRegister::from_bits_retain(0),
      dma_channels,
      is_halted: false,
      key_input: KeyInputRegister::from_bits_retain(0x3ff),
      timers: Timers::new(interrupt_request.clone()),
      waitcnt: WaitstateControlRegister::new(),
      apu: APU::new(),
      scheduler: Scheduler::new(),
      cycles: 0
    };

    cpu.populate_thumb_lut();
    cpu.populate_arm_lut();

    cpu.apu.schedule_samples(&mut cpu.scheduler);
    cpu.scheduler.schedule(EventType::Hdraw, HDRAW_CYCLES as usize);

    cpu
  }

  pub fn trigger_interrupt(interrupt_request_ref: &Rc<Cell<InterruptRequestRegister>>, flags: u16) {
    let mut interrupt_request = interrupt_request_ref.get();

    interrupt_request = InterruptRequestRegister::from_bits_retain(interrupt_request.bits() | flags);

    interrupt_request_ref.set(interrupt_request);
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

    for bg_prop in &mut self.gpu.bg_props {
      bg_prop.dx = 0x100;
      bg_prop.dmx = 0;
      bg_prop.dy = 0;
      bg_prop.dmy = 0x100;
    }
  }

  pub fn load_game(&mut self, rom: Vec<u8>, file_path: Option<String>) {
    self.cartridge.rom = rom;
    self.cartridge.file_path = file_path;
    self.cartridge.detect_backup_media();
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

    // println!("attempting to execute instruction {:032b} at address {:X}", instruction, pc.wrapping_sub(8));

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
    // println!("condition is {condition}");
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

  fn check_interrupts(&mut self) {
    if self.interrupt_master_enable && (self.interrupt_enable.bits() & self.interrupt_request.get().bits()) != 0 {
      self.trigger_irq();

      self.is_halted = false;
    }
  }

  pub fn step(&mut self) {
    let cycles = self.scheduler.get_cycles_to_next_event();

    while self.cycles < cycles {
      let mut dma = self.dma_channels.get();
       // first check interrupts
      self.check_interrupts();
      if dma.has_pending_transfers() {
        let should_trigger_irqs = dma.do_transfers(self);
        let mut interrupt_request = self.interrupt_request.get();

        for i in 0..4 {
          if should_trigger_irqs[i] {
            interrupt_request.request_dma(i);
          }
        }

        self.interrupt_request.set(interrupt_request);
        self.dma_channels.set(dma);
      } else if !self.is_halted {
        if self.cpsr.contains(PSRRegister::STATE_BIT) {
          self.step_thumb();
        } else {
          self.step_arm();
        }
      } else {
        self.cycles = cycles;

        break;
      }
    }

    self.scheduler.update_cycles(cycles);

    while let Some((event_type, cycles_left)) = self.scheduler.get_next_event() {
      match event_type {
        EventType::Hdraw => self.gpu.handle_hdraw(&mut self.scheduler),
        EventType::Hblank => self.gpu.handle_hblank(&mut self.scheduler),
        EventType::DMA(channel_id) => {
          let mut dma = self.dma_channels.get();

          dma.channels[channel_id].pending = true;

          self.dma_channels.set(dma);
        }
        EventType::Timer(timer_id) =>  {
          let mut dma = self.dma_channels.get();

          self.timers.t[timer_id].handle_overflow(&mut self.scheduler, cycles_left);
          self.timers.handle_overflow(timer_id, &mut dma, &mut self.scheduler, &mut self.apu, cycles_left);

          self.dma_channels.set(dma);
        }
        EventType::SampleAudio => self.apu.sample_audio(&mut self.scheduler)
      }
    }


  }

  fn step_thumb(&mut self) {
    let pc = self.pc & !(0b1);

    let next_instruction = self.load_16(pc, self.next_fetch) as u32;

    let instruction = self.pipeline[0];
    self.pipeline[0] = self.pipeline[1];
    self.pipeline[1] = next_instruction;

    // println!("executing instruction {:016b} at address {:X}", instruction, pc.wrapping_sub(4));

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
    self.update_cycles(address, access, MemoryWidth::Width32);
    self.mem_read_32(address)
  }

  pub fn load_16(&mut self, address: u32, access: MemoryAccess) -> u16 {
    self.update_cycles(address, access, MemoryWidth::Width16);
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
    self.cycles += cycles as usize;
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

  pub fn trigger_irq(&mut self) {
    if !self.cpsr.contains(PSRRegister::IRQ_DISABLE) {
      // println!("finally triggering irq!");
      let lr = self.get_irq_return_address();
      self.interrupt(OperatingMode::IRQ, IRQ_VECTOR, lr);

      self.cpsr.insert(PSRRegister::IRQ_DISABLE);
    }
  }

  fn get_irq_return_address(&self) -> u32 {
    let word_size = if self.cpsr.contains(PSRRegister::STATE_BIT) {
      2
    } else {
      4
    };

    self.pc + 4 - (2 * word_size)
  }

  pub fn software_interrupt(&mut self) {
    let lr = if self.cpsr.contains(PSRRegister::STATE_BIT) { self.pc - 2 } else { self.pc - 4 };
    self.interrupt(OperatingMode::Supervisor, SOFTWARE_INTERRUPT_VECTOR, lr);
    self.cpsr.insert(PSRRegister::IRQ_DISABLE);
  }

  pub fn interrupt(&mut self, mode: OperatingMode, vector: u32, lr: u32) {
    let bank = mode.bank_index();

    self.r14_banks[bank] = lr;
    self.spsr_banks[bank] = self.cpsr;

    self.set_mode(mode);

    // change to ARM state
    self.cpsr.remove(PSRRegister::STATE_BIT);

    self.pc = vector;

    self.reload_pipeline32();
  }

  pub fn push(&mut self, val: u32, access: MemoryAccess) {
    self.r[SP_REGISTER] -= 4;

    // println!("pushing {val} to address {:X}", self.r[SP_REGISTER] & !(0b11));

    self.store_32(self.r[SP_REGISTER] & !(0b11), val, access);
  }

  pub fn pop(&mut self, access: MemoryAccess) -> u32 {
    let val = self.load_32(self.r[SP_REGISTER] & !(0b11), access);

    // println!("popping {val} from address {:X}", self.r[SP_REGISTER] & !(0b11));

    self.r[SP_REGISTER] += 4;

    val
  }

  pub fn ldr_halfword(&mut self, address: u32) -> u32 {
    if address & 0b1 != 0 {
      let rotation = (address & 0b1) << 3;

      let value = self.load_16(address & !(0b1), MemoryAccess::NonSequential);

      let mut carry = self.cpsr.contains(PSRRegister::CARRY);
      let return_val = self.ror(value as u32, rotation as u8, false, false, &mut carry);

      self.cpsr.set(PSRRegister::CARRY, carry);

      return_val
    } else {
      self.load_16(address, MemoryAccess::NonSequential) as u32
    }
  }

  fn ldr_word(&mut self, address: u32) -> u32 {
    if address & (0b11) != 0 {
      let rotation = (address & 0b11) << 3;

      let value = self.load_32(address & !(0b11), MemoryAccess::NonSequential);

      let mut carry = self.cpsr.contains(PSRRegister::CARRY);

      let return_val = self.ror(value, rotation as u8, false, false, &mut carry);

      self.cpsr.set(PSRRegister::CARRY, carry);

      return_val
    } else {
      self.load_32(address, MemoryAccess::NonSequential)
    }
  }

  fn ldr_signed_halfword(&mut self, address: u32) -> u32 {
    if address & 0b1 != 0 {
      self.load_8(address, MemoryAccess::NonSequential) as i8 as i32 as u32
    } else {
      self.load_16(address, MemoryAccess::NonSequential) as i16 as i32 as u32
    }
  }

  pub fn load_bios(&mut self, bytes: Vec<u8>) {
    self.bios = bytes;
  }

  fn clear_interrupts(&mut self, value: u16) {
    let mut interrupt_request = self.interrupt_request.get();

    interrupt_request = InterruptRequestRegister::from_bits_retain(interrupt_request.bits() & !value);

    self.interrupt_request.set(interrupt_request);
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