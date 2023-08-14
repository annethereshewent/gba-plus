use std::{rc::Rc, cell::Cell};

use crate::cpu::{dma::dma_channels::DmaChannels, registers::interrupt_request_register::InterruptRequestRegister};

pub const SHIFT_LUT: [usize; 4] = [0, 6, 8, 10];

#[derive(Clone)]
pub struct Timer {
  pub id: usize,
  pub reload_value: u16,
  pub value: u16,
  pub timer_ctl: TimerControl,
  pub prescalar_shift: usize,
  pub running: bool,
  pub cycles_to_overflow: u32,
  cycles: u32,
  interrupt_request: Rc<Cell<InterruptRequestRegister>>
}

impl Timer {
  pub fn new(id: usize, interrupt_request: Rc<Cell<InterruptRequestRegister>>) -> Self {
    Self {
      reload_value: 0,
      value: 0,
      timer_ctl: TimerControl::from_bits_retain(0),
      prescalar_shift: 0,
      running: false,
      cycles_to_overflow: 0,
      cycles: 0,
      id,
      interrupt_request
    }
  }

  pub fn tick(&mut self, cycles: u32, dma: &mut DmaChannels) -> bool {
    if self.cycles_to_overflow > 0 {
      self.cycles += cycles;

      if self.cycles >= self.cycles_to_overflow {
        self.cycles -= self.cycles_to_overflow;
        self.handle_overflow();

        self.cycles_to_overflow = self.ticks_for_overflow() << self.prescalar_shift;

        return true;
      }
    }

    false
  }

  fn handle_overflow(&mut self) {
    self.value = self.reload_value;
    if self.timer_ctl.contains(TimerControl::IRQ_ENABLE) {
      // trigger irq
      let mut interrupt_request = self.interrupt_request.get();

      interrupt_request.request_timer(self.id);

      self.interrupt_request.set(interrupt_request);
    }
  }

  pub fn reload_timer(&mut self, value: u16) {
    self.reload_value = value;
    self.value = value;
  }

  pub fn ticks_for_overflow(&mut self) -> u32 {
    0x1_0000 - (self.value as u32)
  }
}

bitflags! {
  #[derive(Copy, Clone)]
  pub struct TimerControl: u16 {
    const COUNT_UP_TIMING = 0b1 << 2;
    const IRQ_ENABLE = 0b1 << 6;
    const ENABLED = 0b1 << 7;
  }
}

impl TimerControl {
  pub fn prescalar_selection(&self) -> u16 {
    self.bits() & 0b11
  }
}