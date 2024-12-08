use serde::{Deserialize, Serialize};

bitflags! {
  #[derive(Serialize, Deserialize)]
  #[serde(transparent)]
  pub struct SoundControlDma: u16 {
    const DMA_SOUND_A_VOLUME = 0b1 << 2;
    const DMA_SOUND_B_VOLUME = 0b1 << 3;
    const DMA_SOUND_A_ENABLE_RIGHT = 0b1 << 8;
    const DMA_SOUND_A_ENABLE_LEFT = 0b1 << 9;
    const DMA_SOUND_A_TIMER_SELECT = 0b1 << 10;
    const DMA_SOUND_A_RESET = 0b1 << 11;
    const DMA_SOUND_B_ENABLE_RIGHT = 0b1 << 12;
    const DMA_SOUND_B_ENABLE_LEFT = 0b1 << 13;
    const DMA_SOUND_B_TIMER_SELECT = 0b1 << 14;
    const DMA_SOUND_B_RESET = 0b1 << 15;
  }
}

impl SoundControlDma {
  pub fn channel_sound_volume(&self) -> f32 {
    let vol_index = self.bits() & 0b11;

    match vol_index {
      0 => 0.25,
      1 => 0.50,
      2 => 0.75,
      3 => 0.0,
      _ => unreachable!("can't happen")
    }
  }

  pub fn dma_a_sound_volume(&self) -> f32 {
    if self.contains(Self::DMA_SOUND_A_VOLUME) {
      1.0
    } else {
      0.5
    }
  }

  pub fn dma_b_sound_volume(&self) -> f32 {
    if self.contains(Self::DMA_SOUND_B_VOLUME) {
      1.0
    } else {
      0.5
    }
  }
}