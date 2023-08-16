use std::{rc::Rc, cell::Cell};

use self::timer::{Timer, TimerControl, CYCLE_LUT};

use super::{dma::dma_channels::DmaChannels, registers::interrupt_request_register::InterruptRequestRegister};

pub mod timer;

pub struct Timers {
  pub t: [Timer; 4],
}

impl Timers {
  pub fn new(interrupt_request: Rc<Cell<InterruptRequestRegister>>) -> Self {
    Self {
      t: [Timer::new(0, interrupt_request.clone()), Timer::new(1, interrupt_request.clone()), Timer::new(2, interrupt_request.clone()), Timer::new(3, interrupt_request.clone())],
    }
  }

  pub fn tick(&mut self, cycles: u32, dma: &mut DmaChannels) {
    for i in 0..self.t.len() {
      let timer = &mut self.t[i];

      let timer_overflowed = timer.tick(cycles);

      let timer_id = timer.id;
      if timer_overflowed {
        self.handle_overflow(timer_id, dma);
      }
    }
  }

  pub fn handle_overflow(&mut self, timer_id: usize, dma: &mut DmaChannels) {
    if timer_id != 3 {
      let next_timer_id = timer_id + 1;

      let next_timer = &mut self.t[next_timer_id];

      if next_timer.timer_ctl.contains(TimerControl::COUNT_UP_TIMING) && next_timer.count_up_timer() {
        self.handle_overflow(next_timer_id, dma);
      }
    }

    if timer_id == 0 || timer_id == 1 {
      // do apu related timer stuff
    }
  }

  pub fn write_timer_control(&mut self, timer_id: usize, value: u16) {
    let new_ctl = TimerControl::from_bits_retain(value);
    let mut timer = &mut self.t[timer_id];

    timer.prescalar_frequency = CYCLE_LUT[new_ctl.prescalar_selection() as usize];

    if new_ctl.contains(TimerControl::ENABLED) && !timer.timer_ctl.contains(TimerControl::ENABLED) {
      timer.value = timer.reload_value;
      timer.cycles = 0;
      timer.running = true;
    } else if !new_ctl.contains(TimerControl::ENABLED) {
      timer.running = false;
      timer.cycles = 0;
    }

    timer.timer_ctl = new_ctl;
  }
}