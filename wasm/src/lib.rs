extern crate gba_emulator;

use std::collections::HashMap;

use gba_emulator::{cpu::{registers::key_input_register::KeyInputRegister, CPU}, gpu::CYCLES_PER_FRAME};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

macro_rules! console_log {
  ($($t:tt)*) => (log(&format_args!($($t)*).to_string()))
}

#[derive(PartialEq, Eq, Hash)]
#[wasm_bindgen]
pub enum ButtonEvent {
  ButtonA,
  ButtonB,
  ButtonL,
  ButtonR,
  Select,
  Start,
  Up,
  Down,
  Left,
  Right
}

#[wasm_bindgen]
pub struct WasmEmulator {
  cpu: CPU,
  key_map: HashMap<ButtonEvent, KeyInputRegister>
}

#[wasm_bindgen]
impl WasmEmulator {
  #[wasm_bindgen(constructor)]
  pub fn new() -> Self {
    let mut key_map = HashMap::new();

    key_map.insert(ButtonEvent::ButtonA, KeyInputRegister::ButtonA);
    key_map.insert(ButtonEvent::ButtonB, KeyInputRegister::ButtonB);
    key_map.insert(ButtonEvent::ButtonL, KeyInputRegister::ButtonL);
    key_map.insert(ButtonEvent::ButtonR, KeyInputRegister::ButtonR);
    key_map.insert(ButtonEvent::Select, KeyInputRegister::Select);
    key_map.insert(ButtonEvent::Start, KeyInputRegister::Start);
    key_map.insert(ButtonEvent::Up, KeyInputRegister::Up);
    key_map.insert(ButtonEvent::Down, KeyInputRegister::Down);
    key_map.insert(ButtonEvent::Left, KeyInputRegister::Left);
    key_map.insert(ButtonEvent::Right, KeyInputRegister::Right);

    WasmEmulator {
      cpu: CPU::new(),
      key_map
    }
  }

  pub fn update_buffer(&mut self, left_buffer: &mut [f32], right_buffer: &mut [f32]) {
    let mut apu = &mut self.cpu.apu;

    let mut previous_sample = 0.0;

    let mut left_index = 0;
    let mut right_index = 0;

    for i in 0..apu.buffer_index {
      let sample = apu.audio_samples[i];
      if i % 2 == 0 {
        left_buffer[left_index] = sample as f32 * 0.05;
        left_index += 1;
      } else {
        right_buffer[right_index] = sample as f32 * 0.05;
        right_index += 1;
      }

      previous_sample = sample as f32 * 0.05;
    }

    for i in apu.buffer_index..left_buffer.len() {
      left_buffer[i] = previous_sample;
      right_buffer[i] = previous_sample;
    }

    apu.buffer_index = 0;
  }

  pub fn step_frame(&mut self) {
    let mut cycles = 0;

    while cycles < CYCLES_PER_FRAME {
      cycles += self.cpu.step();
    }
  }

  pub fn get_picture_pointer(&self) -> *const u8 {
    self.cpu.gpu.picture.data.as_ptr()
  }

  pub fn load(&mut self, rom: &[u8]) {
    self.cpu.load_game(rom.to_vec(), None);
    // self.cpu.skip_bios();
  }

  pub fn load_bios(&mut self, bios: &[u8]) {
    self.cpu.load_bios(bios.to_vec());
  }

  pub fn update_input(&mut self, button_event: ButtonEvent, is_pressed: bool) {
    if let Some(button) = self.key_map.get(&button_event) {
      self.cpu.key_input.set(*button, !is_pressed);
    }
  }
}


