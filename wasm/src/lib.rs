extern crate gba_emulator;

use std::collections::HashMap;

use gba_emulator::{cpu::{registers::key_input_register::KeyInputRegister, CPU}, gpu::CYCLES_PER_FRAME, cartridge::BackupMedia};
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

  pub fn load_save(&mut self, data: &[u8]) {
    match &mut self.cpu.cartridge.backup {
      BackupMedia::Eeprom(eeprom) => {
        eeprom.chip.memory.buffer = data.to_vec();
      }
      BackupMedia::Flash(flash) => {
        flash.memory.buffer = data.to_vec();
      }
      BackupMedia::Sram(file) => {
        file.buffer = data.to_vec();
      }
      _ => ()
    }
  }

  pub fn backup_file_pointer(&self) -> *const u8 {
    match &self.cpu.cartridge.backup {
      BackupMedia::Eeprom(eeprom) => {
        eeprom.chip.memory.buffer.as_ptr()
      }
      BackupMedia::Flash(flash) => {
        flash.memory.buffer.as_ptr()
      }
      BackupMedia::Sram(file) => {
        file.buffer.as_ptr()
      }
      _ => [].as_ptr()
    }
  }

  pub fn backup_file_size(&self) -> usize {
    match &self.cpu.cartridge.backup {
      BackupMedia::Eeprom(eeprom) => {
        eeprom.chip.memory.size
      }
      BackupMedia::Flash(flash) => {
        flash.memory.size
      }
      BackupMedia::Sram(file) => {
        file.size
      }
      _ => 0
    }
  }

  pub fn has_saved(&self) -> bool {
    match &self.cpu.cartridge.backup {
      BackupMedia::Eeprom(eeprom) => {
        eeprom.chip.memory.has_saved
      }
      BackupMedia::Flash(flash) => {
        flash.memory.has_saved
      }
      BackupMedia::Sram(file) => {
        file.has_saved
      }
      _ => false
    }
  }

  pub fn set_saved(&mut self, val: bool) {
    match &mut self.cpu.cartridge.backup {
      BackupMedia::Eeprom(eeprom) => {
        eeprom.chip.memory.has_saved = val;
      }
      BackupMedia::Flash(flash) => {
        flash.memory.has_saved = val;
      }
      BackupMedia::Sram(file) => {
        file.has_saved = val;
      }
      _ => ()
    }
  }

  pub fn update_buffer(&mut self, left_buffer: &mut [f32], right_buffer: &mut [f32]) {
    let mut apu = &mut self.cpu.apu;

    let mut previous_sample = 0.0;

    let mut left_index = 0;
    let mut right_index = 0;

    for i in 0..apu.buffer_index {
      let sample = (apu.audio_samples[i] as f32) * 0.0005;
      if i % 2 == 0 {
        left_buffer[left_index] = sample;
        left_index += 1;
      } else {
        right_buffer[right_index] = sample;
        right_index += 1;
      }

      previous_sample = sample;
    }

    for i in apu.buffer_index..left_buffer.len() {
      left_buffer[i] = previous_sample;
      right_buffer[i] = previous_sample;
    }

    apu.buffer_index = 0;
  }

  pub fn step_frame(&mut self) {
    while !self.cpu.gpu.frame_finished {
      self.cpu.step();
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


