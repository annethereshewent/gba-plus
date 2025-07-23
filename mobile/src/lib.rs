use std::sync::Arc;

use gba_emulator::{apu::NUM_SAMPLES, cartridge::BackupMedia, cpu::{registers::key_input_register::KeyInputRegister, CPU}};
use ringbuf::{storage::Heap, traits::{Consumer, Split}, wrap::caching::Caching, HeapRb, SharedRb};

extern crate gba_emulator;

const BUTTON_B: usize = 0;
const BUTTON_A: usize = 1;
const BUTTON_Y: usize = 2;
const BUTTON_X: usize = 3;
const SELECT: usize = 4;
const START: usize = 6;
const BUTTON_L: usize = 9;
const BUTTON_R: usize = 10;
const UP: usize = 12;
const DOWN: usize = 13;
const LEFT: usize = 14;
const RIGHT: usize = 15;

#[swift_bridge::bridge]
mod ffi {
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

    #[swift_bridge(swift_name = "reloadRom")]
    fn reload_rom(&mut self, rom: &[u8]);

    #[swift_bridge(swift_name = "loadBios")]
    fn load_bios(&mut self, bios: &[u8]);

    #[swift_bridge(swift_name = "updateInput")]
    fn update_input(&mut self, index: usize, is_pressed: bool);

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

    #[swift_bridge(swift_name="setPausedAudio")]
    fn set_paused_audio(&mut self, value: bool);
  }
}

pub struct GBAEmulator {
  cpu: CPU,
  compressed_len: usize,
  consumer: Caching<Arc<SharedRb<Heap<f32>>>, false, true>,
  audio_buffer: Vec<f32>
}

impl GBAEmulator {

  pub fn new() -> Self {
    let ringbuffer = HeapRb::<f32>::new(NUM_SAMPLES);
    let (producer, consumer) = ringbuffer.split();


    GBAEmulator {
      cpu: CPU::new(producer),
      compressed_len: 0,
      consumer,
      audio_buffer: Vec::new()
    }
  }

  fn set_paused_audio(&mut self, value: bool) {
    self.cpu.apu.audio_paused = value;
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

    let ringbuffer = HeapRb::<f32>::new(NUM_SAMPLES);

    let (producer, consumer) = ringbuffer.split();

    self.consumer = consumer;
    self.cpu.apu.producer = Some(producer);

    // repopulate arm and thumb luts
    self.cpu.populate_arm_lut();
    self.cpu.populate_thumb_lut();
  }

  pub fn audio_buffer_ptr(&mut self) -> *const f32 {
    self.audio_buffer = Vec::new();

    for sample in self.consumer.pop_iter() {
      self.audio_buffer.push(sample * 0.0005);
    }

    self.audio_buffer.as_ptr()
  }

  pub fn audio_buffer_length(&self) -> usize {
    self.audio_buffer.len()
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

  pub fn reload_rom(&mut self, rom: &[u8]) {
    self.cpu.reload_game(rom.to_vec());
  }

  pub fn load_bios(&mut self, bios: &[u8]) {
    self.cpu.load_bios(bios.to_vec());
  }

  pub fn set_paused(&mut self, paused: bool) {
    self.cpu.paused = paused;
  }

  pub fn update_input(&mut self, index: usize, is_pressed: bool) {
    match index {
      BUTTON_A => self.cpu.key_input.set(KeyInputRegister::ButtonA, !is_pressed),
      BUTTON_B => self.cpu.key_input.set(KeyInputRegister::ButtonB, !is_pressed),
      START => self.cpu.key_input.set(KeyInputRegister::Start, !is_pressed),
      SELECT => self.cpu.key_input.set(KeyInputRegister::Select, !is_pressed),
      UP => self.cpu.key_input.set(KeyInputRegister::Up, !is_pressed),
      DOWN => self.cpu.key_input.set(KeyInputRegister::Down, !is_pressed),
      LEFT => self.cpu.key_input.set(KeyInputRegister::Left, !is_pressed),
      RIGHT => self.cpu.key_input.set(KeyInputRegister::Right, !is_pressed),
      BUTTON_L => self.cpu.key_input.set(KeyInputRegister::ButtonL, !is_pressed),
      BUTTON_R => self.cpu.key_input.set(KeyInputRegister::ButtonR, !is_pressed),
      _ => ()
    }
  }
}