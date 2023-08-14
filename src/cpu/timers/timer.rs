use std::{rc::Rc, cell::Cell};

use crate::cpu::registers::interrupt_request_register::InterruptRequestRegister;

pub const SHIFT_LUT: [usize; 4] = [0, 6, 8, 10];

#[derive(Clone)]
pub struct Timer {
  pub id: usize,
  pub reload_value: u16,
  pub value: u16,
  pub initial_value: u16,
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
      initial_value: 0,
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

  pub fn tick(&mut self, cycles: u32) -> bool {
    if self.cycles_to_overflow > 0 {
      self.cycles += cycles;

      self.value = self.initial_value + (self.cycles >> self.prescalar_shift) as u16;
      if self.cycles >= self.cycles_to_overflow {
        self.cycles -= self.cycles_to_overflow;
        self.handle_overflow();

        self.cycles_to_overflow = self.ticks_to_overflow() << self.prescalar_shift;
        self.initial_value = self.value;

        return true;
      }
    }

    false
  }

  pub fn update(&mut self) -> bool {
    let mut return_val = false;
    let mut ticks = 1;

    if ticks >= self.ticks_to_overflow() {
      self.handle_overflow();
      ticks -= self.ticks_to_overflow();

      return_val = true;
    }

    self.value += ticks as u16;

    return_val
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

  pub fn ticks_to_overflow(&mut self) -> u32 {
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