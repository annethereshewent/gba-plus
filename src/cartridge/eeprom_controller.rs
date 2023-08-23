use std::{fs, path::PathBuf};

use super::backup_file::BackupFile;

pub struct EepromController {
  pub chip: EepromChip,
  detected: bool
}

impl EepromController {
  pub fn new(file_path: Option<PathBuf>) -> Self {
    let mut eeprom_type = EepromType::Eeprom512;

    let mut detected = false;

    if let Some(file_path) = &file_path {
      if let Ok(metadata) = fs::metadata(&file_path) {
        eeprom_type = match metadata.len() {
          512 => EepromType::Eeprom512,
          8192 => EepromType::Eeprom8k,
          _ => panic!("invalid eeprom type detected")
        };

        detected = true;
      }
    }

    let size = eeprom_type.size();
    Self {
      chip: EepromChip::new(eeprom_type, BackupFile::new(size, file_path)),
      detected
    }
  }

  pub fn read(&mut self, address: u32) -> u16 {
    if self.detected {
      self.chip.clock_data_out(address) as u16
    } else {
      0
    }
  }

  pub fn write(&mut self, address: u32, val: u16) {
    if self.detected {
      self.chip.clock_data_in(address, val as u8);
    }
  }

  pub fn handle_dma(&mut self, destination: u32, source: u32, count: u32) {
    if !self.detected {
      match (destination, source) {
        (0xd00_0000..=0xdff_ffff, _) => {
          let eeprom_type = match count {
            9 => EepromType::Eeprom512,
            17 => EepromType::Eeprom8k,
            73 => EepromType::Eeprom512,
            81 => EepromType::Eeprom8k,
            _ => panic!("invalid count sent to eeprom dma")
          };

          self.detected = true;
          self.chip.set_type(eeprom_type);
        }
        (_, 0xd00_0000..=0xdff_ffff) => {
          panic!("reading from eeprom when size has not been detected yet!")
        }
        _ => ()
      }
    } else {
      if !self.chip.transmitting() && matches!(destination, 0xd00_0000..=0xdff_ffff) {
        self.chip.state = SpiState::RxInstruction;
        self.chip.reset_rx_buffer();
        self.chip.reset_tx_buffer();
      }
    }
  }
}

pub struct EepromChip {
  pub memory: BackupFile,
  address_bits: EepromAddressBits,

  state: SpiState,
  rx_count: usize,
  rx_buffer: u64,

  tx_count: usize,
  tx_buffer: u64,

  address: usize,

  is_ready: bool
}

impl EepromChip {
  pub fn new(eeprom_type: EepromType, memory: BackupFile) -> Self {
    Self {
      memory,
      address_bits: eeprom_type.bits(),
      state: SpiState::RxInstruction,
      is_ready: false,

      rx_count: 0,
      tx_count: 0,

      rx_buffer: 0,
      tx_buffer: 0,

      address: 0
    }
  }

  fn set_type(&mut self, eeprom_type: EepromType) {
    self.address_bits = eeprom_type.bits();
    self.memory.resize(eeprom_type.size())
  }

  fn fill_tx_buffer(&mut self) {
    let mut tx_buffer = 0;
    for i in 0..8 {
      tx_buffer <<= 8;
      tx_buffer |= self.memory.read(self.address + i) as u64
    }

    self.tx_buffer = tx_buffer;
    self.tx_count = 0;
  }

  fn reset_tx_buffer(&mut self) {
    self.tx_buffer = 0;
    self.tx_count = 0;
  }

  fn reset_rx_buffer(&mut self) {
    self.rx_buffer = 0;
    self.rx_count = 0;
  }

  pub fn clock_data_out(&mut self, _address: u32) -> u8 {
    let result = match self.state {
      SpiState::TxData => {
        let result = ((self.tx_buffer >> 63) & 0b1) as u8;
        self.tx_buffer = self.tx_buffer.wrapping_shl(1);
        self.tx_count += 1;

        if self.tx_count == 64 {
          self.reset_tx_buffer();
          self.reset_rx_buffer();
          self.state = SpiState::RxInstruction;
        }
        result
      }
      SpiState::TxDummy => {
        self.tx_count += 1;
        if self.tx_count == 4 {
          self.state = SpiState::TxData;
          self.fill_tx_buffer();
        }
        0
      }
      _ => {
        if self.is_ready {
          1
        } else {
          0
        }
      }
    };

    result
  }

  fn get_instruction(&self, value: u64) -> SpiInstruction {
    match value {
      0b11 =>  SpiInstruction::Read,
      0b10 => SpiInstruction::Write,
      _ => panic!("invalid value specified")
    }
  }

  pub fn transmitting(&self) -> bool {
    match self.state {
      SpiState::TxData | SpiState::TxDummy => true,
      _ => false
    }
  }

  pub fn clock_data_in(&mut self, _address: u32, val: u8) {
    self.rx_buffer = (self.rx_buffer << 1) | (val & 0b1) as u64;

    self.rx_count += 1;

    match self.state {
      SpiState::RxInstruction => {
        if self.rx_count >= 2 {
          let instruction = self.get_instruction(self.rx_buffer);

          self.state = SpiState::RxAddress(instruction);
          self.reset_rx_buffer();
        }
      }
      SpiState::RxAddress(instruction) => {
        if self.rx_count == self.address_bits.value() {
          self.address = (self.rx_buffer as usize) * 8;


          match instruction {
            SpiInstruction::Read => {
              self.state = SpiState::StopBit(instruction);
            }
            SpiInstruction::Write => {
              self.state = SpiState::RxData;
              self.is_ready = false;
              self.reset_rx_buffer();
            }
          }
        }
      }
      SpiState::RxData => {
        if self.rx_count == 64 {
          let mut data = self.rx_buffer;
          for i in 0..8 {
            self.memory.write(self.address + (7 - i), (data & 0xff) as u8);
            data >>= 8;
          }

          self.state = SpiState::StopBit(SpiInstruction::Write);
          self.reset_rx_buffer();
        }
      }
      SpiState::StopBit(SpiInstruction::Read) => {
        self.state = SpiState::TxDummy;
        self.reset_rx_buffer();
        self.reset_tx_buffer();
      }
      SpiState::StopBit(SpiInstruction::Write) => {
        self.is_ready = true;
        self.state = SpiState::RxInstruction;
        self.reset_rx_buffer();
        self.reset_tx_buffer();
      }
      _ => ()
    }
  }
}

#[derive(Copy, Clone)]
enum SpiInstruction {
  Read = 0b11,
  Write = 0b10,
}

#[derive(Copy, Clone)]
enum SpiState {
  RxInstruction,
  RxAddress(SpiInstruction),
  StopBit(SpiInstruction),
  TxDummy,
  TxData,
  RxData,
}

pub enum EepromType {
  Eeprom512,
  Eeprom8k
}

impl EepromType {
  pub fn size(&self) -> usize {
    match self {
      Self::Eeprom512 => 0x200,
      Self::Eeprom8k => 0x2000
    }
  }
  fn bits(&self) -> EepromAddressBits {
    match self {
      EepromType::Eeprom512 => EepromAddressBits::Eeprom6bit,
      EepromType::Eeprom8k => EepromAddressBits::Eeprom14bit,
    }
  }
}

enum EepromAddressBits {
  Eeprom6bit,
  Eeprom14bit,
}

impl EepromAddressBits {
  pub fn value(&self) -> usize {
    match self {
      EepromAddressBits::Eeprom14bit => 14,
      EepromAddressBits::Eeprom6bit => 6
    }
  }
}