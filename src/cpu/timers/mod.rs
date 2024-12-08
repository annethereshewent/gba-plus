use serde::{Deserialize, Serialize};

use crate::{apu::APU, scheduler::Scheduler};

use self::timer::{Timer, TimerControl};

use super::{dma::dma_channels::DmaChannels, registers::interrupt_request_register::InterruptRequestRegister};

pub mod timer;

#[derive(Serialize, Deserialize)]
pub struct Timers {
  pub t: [Timer; 4],
}

impl Timers {
  pub fn new() -> Self {
    Self {
      t: [Timer::new(0), Timer::new(1), Timer::new(2), Timer::new(3)],
    }
  }

  pub fn handle_overflow(
    &mut self,
    timer_id: usize,
    dma: &mut DmaChannels,
    scheduler: &mut Scheduler,
    apu: &mut APU,
    interrupt_request: &mut InterruptRequestRegister,
    cycles_left: usize
  ) {
    if timer_id != 3 {
      let next_timer_id = timer_id + 1;

      let next_timer = &mut self.t[next_timer_id];

      if next_timer.timer_ctl.contains(TimerControl::COUNT_UP_TIMING) && next_timer.count_up_timer(scheduler, interrupt_request, cycles_left) {
        self.handle_overflow(next_timer_id, dma, scheduler, apu, interrupt_request, cycles_left);
      }
      if timer_id == 0 || timer_id == 1 {
        apu.handle_timer_overflow(timer_id, dma);
      }
    }
  }
}