use ffi::GBAButtonEvent;
use gba_emulator::{apu::NUM_SAMPLES, cartridge::BackupMedia, cpu::{registers::key_input_register::KeyInputRegister, CPU}};

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
    ButtonHome,
    GameMenu,
    QuickSave,
    QuickLoad
  }

  extern "Rust" {
    type GBAEmulator;

    #[swift_bridge(init)]
    fn new() -> GBAEmulator;

    #[swift_bridge(swift_name = "stepFrame")]
    fn step_frame(&mut self);

    #[swift_bridge(swift_name = "getPicturePtr")]
    fn get_picture_ptr(&self) -> *const u8;

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

    #[swift_bridge(swift_name="createSaveState")]
    fn create_save_state(&mut self) -> *const u8;

    #[swift_bridge(swift_name="loadSaveState")]
    fn load_save_state(&mut self, buf: &[u8]);

    #[swift_bridge(swift_name="compressedLength")]
    fn compressed_len(&self) -> usize;
  }
}

pub struct GBAEmulator {
  cpu: CPU,
  compressed_len: usize
}

impl GBAEmulator {
  pub fn new() -> Self {
    GBAEmulator {
      cpu: CPU::new(),
      compressed_len: 0
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

  pub fn create_save_state(&mut self) -> *const u8 {
    let buf = self.cpu.create_save_state();

    let compressed = zstd::encode_all(&*buf, 9).unwrap();

    self.compressed_len  = compressed.len();

    compressed.as_ptr()
  }

  pub fn compressed_len(&self) -> usize {
    self.compressed_len
  }

  pub fn load_save_state(&mut self, data: &[u8]) {
    let buf = zstd::decode_all(&*data).unwrap();

    self.cpu.load_save_state(&buf);

    self.cpu.apu.audio_samples = vec![0.0; NUM_SAMPLES].into_boxed_slice();
    self.cpu.apu.buffer_index = 0;

    // repopulate arm and thumb luts
    self.cpu.populate_arm_lut();
    self.cpu.populate_thumb_lut();
  }

  pub fn audio_buffer_ptr(&mut self) -> *const f32 {
    let audio_buffer = &self.cpu
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

  pub fn get_picture_ptr(&self) -> *const u8 {
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