extern crate gba_emulator;
extern crate console_error_panic_hook;

use std::{collections::HashMap, panic, sync::Arc};

use gba_emulator::{apu::NUM_SAMPLES, cartridge::BackupMedia, cpu::{registers::key_input_register::KeyInputRegister, CPU}, gpu::CYCLES_PER_FRAME};
use ringbuf::{storage::Heap, traits::{Consumer, Split}, wrap::caching::Caching, HeapRb, SharedRb};
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
  key_map: HashMap<ButtonEvent, KeyInputRegister>,
  state_len: usize,
  consumer: Caching<Arc<SharedRb<Heap<f32>>>, false, true>,
  audio_buffer: Vec<f32>
}

#[wasm_bindgen]
impl WasmEmulator {
  #[wasm_bindgen(constructor)]
  pub fn new() -> Self {
    panic::set_hook(Box::new(console_error_panic_hook::hook));
    let mut key_map = HashMap::new();

    let ringbuffer = HeapRb::<f32>::new(NUM_SAMPLES);
    let (producer, consumer) = ringbuffer.split();

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
      cpu: CPU::new(producer),
      key_map,
      state_len: 0,
      consumer,
      audio_buffer: Vec::new()
    }
  }

  pub fn create_save_state(&mut self) -> *const u8 {
    let buf = self.cpu.create_save_state();

    self.state_len = buf.len();

    buf.as_ptr()
  }

  pub fn load_save_state(&mut self, data: &[u8]) {
    self.cpu.load_save_state(&data);

    let ringbuffer = HeapRb::<f32>::new(NUM_SAMPLES);
    let (producer, consumer) = ringbuffer.split();

    self.consumer = consumer;
    self.cpu.apu.producer = Some(producer);

    // repopulate arm and thumb luts
    self.cpu.populate_arm_lut();
    self.cpu.populate_thumb_lut();
  }

  pub fn save_state_length(&self) -> usize {
    self.state_len
  }

  pub fn set_pause(&mut self, val: bool) {
    self.cpu.paused = val;
  }

  pub fn reload_rom(&mut self, rom: &[u8]) {
    self.cpu.reload_game(rom.to_vec());
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
    let mut left_index = 0;
    let mut right_index = 0;

    let mut is_left = true;

    for sample in self.consumer.pop_iter() {
      if left_index >= left_buffer.len() {
        break;
      }
      if is_left {
        left_buffer[left_index] = sample * 0.0005;
        left_index += 1;
      } else {
        right_buffer[right_index] = sample * 0.0005;
        right_index += 1;
      }

      is_left = !is_left;
    }
  }

  pub fn step_frame(&mut self) {
    while !self.cpu.gpu.frame_finished {
      if self.cpu.paused {
        break;
      }
      self.cpu.step();
    }

    if self.cpu.scheduler.cycles >= 0xfff0_0000  {
      let to_subtract = self.cpu.scheduler.rebase_cycles();
      self.cpu.cycles -= to_subtract;
    }

    self.cpu.gpu.frame_finished = false;
  }

  pub fn get_picture_pointer(&self) -> *const u8 {
    self.cpu.gpu.picture.data.as_ptr()
  }

  pub fn load(&mut self, rom: &[u8]) {
    self.cpu.load_game(rom.to_vec(), None);
    self.cpu.skip_bios();
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


