use std::path::PathBuf;

use crate::number::Number;

use super::backup_file::BackupFile;

const BANK_SIZE: usize = 0x10000;
const SECTOR_SIZE: usize = 4 * 1024;

pub struct Flash {
  pub memory: BackupFile,
  size: usize,
  bank: usize,
  mode: FlashMode,
  chip_id: u16,
  state: FlashState
}

pub enum FlashMode {
  Initial,
  ChipId,
  Erase,
  WriteByte,
  BankSwitch
}

pub enum FlashState {
  Initial,
  Initial2,
  Command,
  Argument
}

impl Flash {
  pub fn new(file_path: Option<PathBuf>, flash_size: FlashSize) -> Self {
    let size = flash_size as usize;
    let chip_id = flash_size.chip_id();
    Self {
      memory: BackupFile::new(size, file_path),
      size,
      bank: 0,
      mode: FlashMode::Initial,
      chip_id,
      state: FlashState::Initial
    }
  }

  pub fn read<T: Number>(&self, address: u32) -> T {
    let offset = address & 0xffff;
    if matches!(self.mode, FlashMode::ChipId) {
      match offset {
        0 => num::cast::<u16, T>(self.chip_id & 0xff).unwrap(),
        1 => num::cast::<u16, T>((self.chip_id >> 8)).unwrap(),
        _ => panic!("invalid offset specified for chip id mode")
      }
    } else {
      let mem_address = self.flash_offset(offset as usize);

      self.memory.read(mem_address)
    }
  }

  pub fn flash_offset(&self, offset: usize) -> usize {
    offset + self.bank * BANK_SIZE
  }

  pub fn command(&mut self, address: u32, val: u8) {
    const COMMAND_ADDRESS: u32 = 0xe00_5555;
    match (address, val) {
      (COMMAND_ADDRESS, 0x90) => {
        self.mode = FlashMode::ChipId;
        self.reset_state();
      }
      (COMMAND_ADDRESS, 0x80) => {
        self.mode = FlashMode::Erase;
        self.reset_state();
      }
      (COMMAND_ADDRESS, 0xa0) => {
        self.mode = FlashMode::WriteByte;
        self.state = FlashState::Argument;
      },
      (COMMAND_ADDRESS, 0xb0) => {
        self.mode = FlashMode::BankSwitch;
        self.state = FlashState::Argument;
      },
      (COMMAND_ADDRESS, 0xf0) => {
        if matches!(self.mode, FlashMode::ChipId) {
          self.mode = FlashMode::Initial;
        }
        self.reset_state();
      }
      (COMMAND_ADDRESS, 0x10) => {
        if matches!(self.mode, FlashMode::Erase) {
          // erase the entire chip
          for i in 0..self.size {
            self.memory.write(i, 0xff);
          }
        }
        self.reset_state();
        self.mode = FlashMode::Initial;
      }
      (sector_address, 0x30) => {
        if matches!(self.mode, FlashMode::Erase) {
          let sector = sector_address & 0xf000;

          let offset = self.flash_offset(sector as usize);

          for i in 0..SECTOR_SIZE {
            self.memory.write(i + offset, 0xff);
          }
        }
        self.reset_state();
        self.mode = FlashMode::Initial;
      }
      _ => panic!("invalid command or address specified to flash chip")
    }
  }

  pub fn reset_state(&mut self) {
    self.state = FlashState::Initial;
  }

  pub fn write(&mut self, address: u32, val: u8) {
    match self.state {
      FlashState::Initial => {
        if address == 0xe00_5555 && val == 0xaa {
          self.state = FlashState::Initial2
        }
      }
      FlashState::Initial2 => {
        if address == 0xe00_2aaa && val == 0x55 {
          self.state = FlashState::Command
        }
      }
      FlashState::Command => {
        self.command(address, val);
      }
      FlashState::Argument => {
        match self.mode {
          FlashMode::BankSwitch => {
            if address == 0xe00_0000 {
              self.bank = val as usize;
            }
          }
          FlashMode::WriteByte => {
            let offset = address & 0xffff;
            let mem_offset = self.flash_offset(offset as usize);

            self.memory.write(mem_offset, val);
          }
          _ => panic!("invalid mode specified for argument state")
        }

        self.reset_state();
        self.mode = FlashMode::Initial;

      }
    }
  }
}

#[derive(Copy, Clone)]
pub enum FlashSize {
  Flash128k = 1024 * 128,
  Flash64k = 64 * 1024
}

const MACRONIX_64K_CHIP_ID: u16 = 0x1CC2;
const MACRONIX_128K_CHIP_ID: u16 = 0x09c2;

impl FlashSize {
  // macronix chips should suffice for now
  pub fn chip_id(&self) -> u16 {
    match self {
      Self::Flash128k => MACRONIX_128K_CHIP_ID,
      Self::Flash64k => MACRONIX_64K_CHIP_ID
    }
  }
}