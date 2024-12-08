use std::path::Path;

use serde::{Deserialize, Serialize};

use self::{eeprom_controller::EepromController, flash::{Flash, FlashSize}, backup_file::BackupFile};

pub mod eeprom_controller;
pub mod flash;
pub mod backup_file;

#[derive(Serialize, Deserialize)]
pub struct Cartridge {
  #[serde(skip_deserializing)]
  #[serde(skip_serializing)]
  pub rom: Vec<u8>,
  pub backup: BackupMedia,
  pub file_path: Option<String>
}

#[derive(Serialize, Deserialize)]
pub enum BackupMedia {
  Eeprom(EepromController),
  Flash(Flash),
  Sram(BackupFile),
  Undetected
}

const BACKUP_MEDIA: &[&str] = &["EEPROM", "SRAM", "FLASH_", "FLASH512_", "FLASH1M_"];

impl Cartridge {
  pub fn detect_backup_media(&mut self) {
    for i in 0..5 {
      let needle = BACKUP_MEDIA[i].as_bytes();

      if let Some(_) = self.rom.windows(needle.len()).position(|window| window == needle) {
        self.backup = self.create_backup(i);
        break;
      }
    }
  }

  fn create_backup(&self, index: usize) -> BackupMedia {
    let backup_path = if let Some(file_path) = &self.file_path {
      Some(Path::new(file_path).with_extension("sav"))
    } else {
      None
    };

    match index {
      0 => BackupMedia::Eeprom(EepromController::new(backup_path)),
      1 => BackupMedia::Sram(BackupFile::new(32 * 1024, backup_path)),
      2 => BackupMedia::Flash(Flash::new(backup_path, FlashSize::Flash64k)), // regular flash
      3 => BackupMedia::Flash(Flash::new(backup_path, FlashSize::Flash64k)), // flash 512
      4 => BackupMedia::Flash(Flash::new(backup_path, FlashSize::Flash128k)), // flash 1024
      _ => BackupMedia::Undetected
    }
  }
}