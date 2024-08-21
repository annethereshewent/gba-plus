use std::{rc::Rc, cell::Cell};

use crate::{apu::APU, scheduler::Scheduler};

use self::timer::{Timer, TimerControl};

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

  pub fn handle_overflow(&mut self, timer_id: usize, dma: &mut DmaChannels, scheduler: &mut Scheduler, cycles_left: usize) {
    if timer_id != 3 {
      let next_timer_id = timer_id + 1;

      let next_timer = &mut self.t[next_timer_id];

      if next_timer.timer_ctl.contains(TimerControl::COUNT_UP_TIMING) && next_timer.count_up_timer(scheduler, cycles_left) {
        self.handle_overflow(next_timer_id, dma, scheduler, cycles_left);
      }
    }
  }
}