use serde::{Deserialize, Serialize};

use crate::{cartridge::BackupMedia, cpu::{MemoryAccess, CPU}, scheduler::{EventType, Scheduler}};

use self::registers::dma_control_register::DmaControlRegister;

pub mod registers;

const FIFO_REGISTER_A: u32 = 0x400_00a0;
const FIFO_REGISTER_B: u32 = 0x400_00a4;

pub struct DmaParams {
  pub source_adjust: i32,
  pub destination_adjust: i32,
  pub count: u32,
  pub internal_source_address: u32,
  pub internal_destination_address: u32,
  pub word_size: u32,
  pub fifo_mode: bool,
  pub should_trigger_irq: bool
}

#[derive(Copy, Clone, Deserialize, Serialize)]
pub struct DmaChannel {
  pub id: usize,
  pub source_address: u32,
  pub destination_address: u32,
  pub internal_source_address: u32,
  pub internal_destination_address: u32,
  pub internal_count: u16,
  pub dma_control: DmaControlRegister,
  pub word_count: u16,
  pub pending: bool,
  pub running: bool,
  pub fifo_mode: bool,
  cycles: u32,
  cycles_to_transfer: u32
}

impl DmaChannel {
  pub fn new(id: usize) -> Self {
    Self {
      source_address: 0,
      destination_address: 0,
      word_count: 0,
      dma_control: DmaControlRegister::from_bits_retain(0),
      pending: false,
      internal_count: 0,
      internal_destination_address: 0,
      internal_source_address: 0,
      running: false,
      fifo_mode: false,
      id,
      cycles: 0,
      cycles_to_transfer: 0
    }
  }

  pub fn transfer(&mut self, cpu: &mut CPU) -> bool {
    let mut should_trigger_irq = false;

    let word_size = if self.dma_control.contains(DmaControlRegister::DMA_TRANSFER_TYPE) {
      4 // 32 bit
    } else {
      2 // 16 bit
    };

    let count = match self.internal_count {
      0 => if self.id == 3 { 0x1_0000 } else { 0x4000 },
      _ => self.internal_count as u32
    };

    let destination_adjust = match self.dma_control.dest_addr_control() {
      0 | 3 => word_size,
      1 => -word_size,
      2 => 0,
      _ => unreachable!("can't be")
    };

    let source_adjust = match self.dma_control.source_addr_control() {
      0 => word_size,
      1 => -word_size,
      2 => 0,
      _ => panic!("illegal value specified for source address control")
    };

    if self.id == 3 && word_size == 2 {
      if let BackupMedia::Eeprom(eeprom_controller) = &mut cpu.cartridge.backup {
        eeprom_controller.handle_dma(self.internal_destination_address, self.internal_source_address, self.internal_count.into());
      }
    }

    let mut access = MemoryAccess::NonSequential;

    if self.fifo_mode {
      for _ in 0..4 {
        let value = cpu.load_32(self.internal_source_address & !(0b11), access);
        cpu.store_32(self.internal_destination_address & !(0b11), value, access);
        access = MemoryAccess::Sequential;
        self.internal_source_address += 4;
      }
    } else if word_size == 4 {
      for _ in 0..count {
        let word = cpu.load_32(self.internal_source_address & !(0b11), access);
        cpu.store_32(self.internal_destination_address & !(0b11), word, access);
        access = MemoryAccess::Sequential;
        self.internal_source_address = (self.internal_source_address as i32).wrapping_add(source_adjust) as u32;
        self.internal_destination_address = (self.internal_destination_address as i32).wrapping_add(destination_adjust) as u32;
      }
    } else {
      for _ in 0..count {
        let half_word = cpu.load_16(self.internal_source_address & !(0b1), access);
        cpu.store_16(self.internal_destination_address & !(0b1), half_word, access);
        access = MemoryAccess::Sequential;
        self.internal_source_address = (self.internal_source_address as i32).wrapping_add(source_adjust) as u32;
        self.internal_destination_address = (self.internal_destination_address as i32).wrapping_add(destination_adjust) as u32;
      }
    }


    if self.dma_control.contains(DmaControlRegister::IRQ_ENABLE) {
      should_trigger_irq = true;
    }

    if self.dma_control.contains(DmaControlRegister::DMA_REPEAT) {
      if self.dma_control.dest_addr_control() == 3 {
        self.internal_destination_address = self.destination_address;
      }
    } else {
      self.running = false;
      self.dma_control.remove(DmaControlRegister::DMA_ENABLE);
    }

    should_trigger_irq
  }

  pub fn write_control(&mut self, value: u16) {
    let dma_control = DmaControlRegister::from_bits_retain(value);

    if dma_control.contains(DmaControlRegister::DMA_ENABLE) && !self.dma_control.contains(DmaControlRegister::DMA_ENABLE) {
      self.internal_destination_address = self.destination_address;
      self.internal_source_address = self.source_address;
      self.internal_count = self.word_count;

      self.running = true;

      let timing = dma_control.dma_start_timing();

      if timing == 0 {
        self.pending = true;
      } else {
        self.pending = false;
      }

      self.fifo_mode = timing == 3
        && dma_control.contains(DmaControlRegister::DMA_REPEAT)
        && (self.id == 1) || (self.id == 2)
        && (self.destination_address == FIFO_REGISTER_A || self.destination_address == FIFO_REGISTER_B);
    }

    if !dma_control.contains(DmaControlRegister::DMA_ENABLE) {
      self.running = false;
    }

    self.dma_control = dma_control;
  }
}