extern crate gba_emulator;

use std::collections::HashMap;

use gba_emulator::{cpu::{CPU, registers::key_input_register::KeyInputRegister}, gpu::CYCLES_PER_FRAME};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

macro_rules! console_log {
  // Note that this is using the `log` function imported above during
  // `bare_bones`
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

macro_rules! console_log {
  // Note that this is using the `log` function imported above during
  // `bare_bones`
  ($($t:tt)*) => (log(&format_args!($($t)*).to_string()))
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
  }

  pub fn update_input(&mut self, button_event: ButtonEvent, is_pressed: bool) {
    if let Some(button) = self.key_map.get(&button_event) {
      self.cpu.key_input.set(*button, is_pressed);
    }
  }
}


