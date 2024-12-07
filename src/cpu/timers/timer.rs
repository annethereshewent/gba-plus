use std::{rc::Rc, cell::Cell};

use serde::{Deserialize, Serialize};

use crate::{cpu::registers::interrupt_request_register::InterruptRequestRegister, scheduler::{EventType, Scheduler}};

pub const CYCLE_LUT: [u32; 4] = [1, 64, 256, 1024];

#[derive(Clone, Serialize, Deserialize)]
pub struct Timer {
  pub id: usize,
  pub reload_value: u16,
  pub value: u16,
  pub timer_ctl: TimerControl,
  pub prescalar_frequency: u32,
  pub running: bool,
  pub cycles: u32,
  interrupt_request: Rc<Cell<InterruptRequestRegister>>,
  start_cycles: usize
}

impl Timer {
  pub fn new(id: usize, interrupt_request: Rc<Cell<InterruptRequestRegister>>) -> Self {
    Self {
      reload_value: 0,
      value: 0,
      timer_ctl: TimerControl::from_bits_retain(0),
      prescalar_frequency: 0,
      running: false,
      cycles: 0,
      id,
      interrupt_request,
      start_cycles: 0
    }
  }

  pub fn count_up_timer(&mut self, scheduler: &mut Scheduler, cycles_left: usize) -> bool {
    let mut return_val = false;

    if self.running {
      let temp = self.value.wrapping_add(1);

      // overflow has happened
      if temp < self.value {
        self.handle_overflow(scheduler, cycles_left);

        return_val = true;
      } else {
        self.value = temp;
      }
    }

    return_val
  }

  pub fn handle_overflow(&mut self, scheduler: &mut Scheduler, cycles_left: usize) {
    if !self.timer_ctl.contains(TimerControl::COUNT_UP_TIMING) {
      let event_type = EventType::Timer(self.id);

      self.value = self.reload_value;
      let cycles_till_overflow = self.prescalar_frequency * (0x1_0000 - self.value as u32);
      scheduler.schedule(event_type, cycles_till_overflow as usize - cycles_left);
      self.start_cycles = scheduler.cycles;
    }

    if self.timer_ctl.contains(TimerControl::IRQ_ENABLE) {
      let mut interrupt_request = self.interrupt_request.get();
      // trigger irq
      interrupt_request.request_timer(self.id);

      self.interrupt_request.set(interrupt_request);
    }
  }

  pub fn reload_timer_value(&mut self, value: u16) {
    self.reload_value = value;
  }

  pub fn write_timer_control(&mut self, value: u16, scheduler: &mut Scheduler) {
    let new_ctl = TimerControl::from_bits_retain(value);

    self.prescalar_frequency = CYCLE_LUT[new_ctl.prescalar_selection() as usize];

    if new_ctl.contains(TimerControl::ENABLED) && !self.timer_ctl.contains(TimerControl::ENABLED) {
      self.value = self.reload_value;

      let cycles_till_overflow = self.prescalar_frequency * (0x1_0000 - self.value as u32);

      scheduler.schedule(EventType::Timer(self.id), cycles_till_overflow as usize);

      self.running = true;
    } else if !new_ctl.contains(TimerControl::ENABLED) || new_ctl.contains(TimerControl::COUNT_UP_TIMING) {
      scheduler.remove(EventType::Timer(self.id));
    }

    self.timer_ctl = new_ctl;
  }
}

bitflags! {
  #[derive(Copy, Clone, Serialize, Deserialize)]
  #[serde(transparent)]
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