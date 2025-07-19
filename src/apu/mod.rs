use std::sync::Arc;
use ringbuf::{storage::Heap, traits::Producer, wrap::caching::Caching, SharedRb};
use serde::{Deserialize, Serialize};

use crate::{cpu::{dma::dma_channels::DmaChannels, CPU_CLOCK_SPEED}, scheduler::{EventType, Scheduler}};

use self::{registers::{sound_control_dma::SoundControlDma, sound_control_enable::SoundControlEnable}, dma_fifo::DmaFifo};

pub mod registers;
pub mod dma_fifo;

pub const GBA_SAMPLE_RATE: u32 = 32768;
pub const NUM_SAMPLES: usize = 8192*2;

const FIFO_REGISTER_A: u32 = 0x400_00a0;
const FIFO_REGISTER_B: u32 = 0x400_00a4;

#[derive(Serialize, Deserialize)]
pub struct APU {
  pub fifo_a: DmaFifo,
  pub fifo_b: DmaFifo,
  pub fifo_enable: bool,
  pub soundcnt_h: SoundControlDma,
  pub soundcnt_l: SoundControlEnable,
  pub cycles_per_sample: u32,
  pub sample_rate: u32,
  pub sound_bias: u16,
  pub buffer_index: usize,
  pub previous_value: f32,
  #[serde(skip_serializing, skip_deserializing)]
  producer: Option<Caching<Arc<SharedRb<Heap<f32>>>, true, false>>,
  phase: f32,
  in_frequency: f32,
  out_frequency: f32,
  last_sample: [f32; 2]
}

impl APU {
  pub fn new(producer: Caching<Arc<SharedRb<Heap<f32>>>, true, false>) -> Self {
    Self {
      fifo_a: DmaFifo::new(),
      fifo_b: DmaFifo::new(),
      fifo_enable: false,
      soundcnt_h: SoundControlDma::from_bits_retain(0),
      soundcnt_l: SoundControlEnable::new(),
      sample_rate: GBA_SAMPLE_RATE,
      cycles_per_sample: CPU_CLOCK_SPEED / GBA_SAMPLE_RATE,
      sound_bias: 0x200,
      buffer_index: 0,
      previous_value: 0.0,
      in_frequency: GBA_SAMPLE_RATE as f32,
      out_frequency: 44100 as f32,
      last_sample: [0.0; 2],
      phase: 0.0,
      producer: Some(producer)
    }
  }

  pub fn on_soundcnt_h_write(&mut self) {
    if self.soundcnt_h.contains(SoundControlDma::DMA_SOUND_A_RESET) {
      self.fifo_a.reset();
    }
    if self.soundcnt_h.contains(SoundControlDma::DMA_SOUND_B_RESET) {
      self.fifo_b.reset();
    }
  }

  pub fn schedule_samples(&self, scheduler: &mut Scheduler) {
    scheduler.schedule(EventType::SampleAudio, self.cycles_per_sample as usize);
  }

  pub fn sample_audio(&mut self, scheduler: &mut Scheduler) {
    scheduler.schedule(EventType::SampleAudio, self.cycles_per_sample as usize);

    let mut left_sample: i16 = 0;
    let mut right_sample: i16 = 0;

    if self.soundcnt_h.contains(SoundControlDma::DMA_SOUND_A_ENABLE_LEFT) {
      self.update_sample(self.fifo_a.value as i16, &mut left_sample, SoundControlDma::DMA_SOUND_A_VOLUME);
    }
    if self.soundcnt_h.contains(SoundControlDma::DMA_SOUND_A_ENABLE_RIGHT) {
      self.update_sample(self.fifo_a.value as i16, &mut right_sample, SoundControlDma::DMA_SOUND_A_VOLUME);
    }
    if self.soundcnt_h.contains(SoundControlDma::DMA_SOUND_B_ENABLE_LEFT) {
      self.update_sample(self.fifo_b.value as i16, &mut left_sample, SoundControlDma::DMA_SOUND_B_VOLUME);
    }
    if self.soundcnt_h.contains(SoundControlDma::DMA_SOUND_B_ENABLE_RIGHT) {
      self.update_sample(self.fifo_b.value as i16, &mut right_sample, SoundControlDma::DMA_SOUND_B_VOLUME);
    }

    let mut sample = [left_sample as i32 as f32, right_sample as i32 as f32];
    self.resample(&mut sample);
  }

  pub fn update_sample(&mut self, value: i16, sample: &mut i16, stereo_channel: SoundControlDma) {
    let volume_shift = if self.soundcnt_h.contains(stereo_channel) { 1 } else { 0 };

    *sample += value * (2 << volume_shift);

    self.apply_bias(sample);
  }

  fn push_sample(&mut self, sample: f32) {

  }

  fn resample(&mut self, sample: &mut [f32; 2]) {
    if let Some(producer) = &mut self.producer {
      while self.phase < 1.0 {
        producer.try_push(sample[0]);
        producer.try_push(sample[1]);
        self.phase += self.in_frequency / self.out_frequency;
      }
      self.phase -= 1.0;
      self.last_sample = *sample;
    }
  }

  pub fn apply_bias(&mut self, sample: &mut i16) {
    let level = self.sound_bias & 0b1111111111;

    *sample += level as i16;

    if *sample > 0x3ff {
      *sample = 0x3ff;
    } else if *sample < 0 {
      *sample = 0;
    }
    *sample -= level as i16;
  }

  pub fn write_sound_bias(&mut self, val: u16) {
    self.sound_bias = val & 0xc3fe;

    let sample_shift = (val >> 14) & 0b11;

    self.sample_rate = GBA_SAMPLE_RATE << sample_shift;
    self.cycles_per_sample = CPU_CLOCK_SPEED / self.sample_rate;

    self.in_frequency = self.sample_rate as f32;
  }

  pub fn handle_timer_overflow(&mut self, timer_id: usize, dma: &mut DmaChannels) {
    if !self.fifo_enable {
      return;
    }

    let fifo_a_timer_id = if self.soundcnt_h.contains(SoundControlDma::DMA_SOUND_A_TIMER_SELECT) { 1 } else { 0 };
    let fifo_b_timer_id = if self.soundcnt_h.contains(SoundControlDma::DMA_SOUND_B_TIMER_SELECT) { 1 } else { 0 };

    if fifo_a_timer_id == timer_id {
      self.fifo_a.value = self.fifo_a.read();

      if self.fifo_a.count <= 16 {
        dma.notify_apu_event(FIFO_REGISTER_A);
      }
    }
    if fifo_b_timer_id == timer_id {
      self.fifo_b.value = self.fifo_b.read();

      if self.fifo_b.count <= 16 {
        dma.notify_apu_event(FIFO_REGISTER_B);
      }
    }
  }
}