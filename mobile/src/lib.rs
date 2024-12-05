use ffi::GBAButtonEvent;
use gba_emulator::{cartridge::BackupMedia, cpu::{registers::key_input_register::KeyInputRegister, CPU}};

extern crate gba_emulator;

#[swift_bridge::bridge]
mod ffi {
  enum GBAButtonEvent {
    ButtonA,
    ButtonB,
    ButtonL,
    ButtonR,
    Select,
    Start,
    Up,
    Down,
    Left,
    Right,
    ButtonHome
  }

  extern "Rust" {
    type GBAEmulator;

    #[swift_bridge(init)]
    fn new() -> GBAEmulator;

    #[swift_bridge(swift_name = "stepFrame")]
    fn step_frame(&mut self);

    #[swift_bridge(swift_name = "getPicturePointer")]
    fn get_picture_pointer(&self) -> *const u8;

    #[swift_bridge(swift_name = "loadSave")]
    fn load_save(&mut self, data: &[u8]);

    #[swift_bridge(swift_name = "backupFilePointer")]
    fn backup_file_pointer(&self) -> *const u8;

    #[swift_bridge(swift_name = "backupFileSize")]
    fn backup_file_size(&self) -> usize;

    #[swift_bridge(swift_name = "hasSaved")]
    fn has_saved(&self) -> bool;

    #[swift_bridge(swift_name = "setSaved")]
    fn set_saved(&mut self, val: bool);

    #[swift_bridge(swift_name = "audioBufferPtr")]
    fn audio_buffer_ptr(&mut self) -> *const f32;

    fn load(&mut self, rom: &[u8]);

    #[swift_bridge(swift_name = "loadBios")]
    fn load_bios(&mut self, bios: &[u8]);

    #[swift_bridge(swift_name = "updateInput")]
    fn update_input(&mut self, button_event: GBAButtonEvent, is_pressed: bool);

    #[swift_bridge(swift_name = "audioBufferLength")]
    fn audio_buffer_length(&self) -> usize;

    #[swift_bridge(swift_name="setPaused")]
    fn set_paused(&mut self, paused: bool);
  }
}

pub struct GBAEmulator {
  cpu: CPU
}

impl GBAEmulator {
  pub fn new() -> Self {
    GBAEmulator {
      cpu: CPU::new()
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

  pub fn audio_buffer_ptr(&mut self) -> *const f32 {
    let audio_buffer = self.cpu
      .apu
      .audio_samples;

    let mut vec = Vec::new();

    for i in 0..self.cpu.apu.buffer_index {
      let sample = audio_buffer[i]  * 0.0005;

      vec.push(sample);
    }

    self.cpu.apu.buffer_index = 0;

    vec.as_ptr()
  }

  pub fn audio_buffer_length(&self) -> usize {
    self.cpu.apu.buffer_index
  }

  pub fn step_frame(&mut self) {
    while !self.cpu.gpu.frame_finished {
      if !self.cpu.paused {
        self.cpu.step();
      } else {
        break;
      }
    }

    self.cpu.gpu.cap_fps();

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

  pub fn set_paused(&mut self, paused: bool) {
    self.cpu.paused = paused;
  }

  pub fn update_input(&mut self, button_event: GBAButtonEvent, is_pressed: bool) {
    use self::GBAButtonEvent::*;
    match button_event {
      ButtonA => self.cpu.key_input.set(KeyInputRegister::ButtonA, !is_pressed),
      ButtonB => self.cpu.key_input.set(KeyInputRegister::ButtonB, !is_pressed),
      Start => self.cpu.key_input.set(KeyInputRegister::Start, !is_pressed),
      Select => self.cpu.key_input.set(KeyInputRegister::Select, !is_pressed),
      Up => self.cpu.key_input.set(KeyInputRegister::Up, !is_pressed),
      Down => self.cpu.key_input.set(KeyInputRegister::Down, !is_pressed),
      Left => self.cpu.key_input.set(KeyInputRegister::Left, !is_pressed),
      Right => self.cpu.key_input.set(KeyInputRegister::Right, !is_pressed),
      ButtonL => self.cpu.key_input.set(KeyInputRegister::ButtonL, !is_pressed),
      ButtonR => self.cpu.key_input.set(KeyInputRegister::ButtonR, !is_pressed),
      _ => ()
    }
  }
}