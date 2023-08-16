use std::path::Path;

use self::{eeprom_controller::EepromController, flash::Flash, backup_file::BackupFile};

pub mod eeprom_controller;
pub mod flash;
pub mod backup_file;

pub struct Cartridge {
  pub rom: Vec<u8>,
  pub backup: BackupMedia,
  pub file_path: String
}

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
    let backup_path = Path::new(&self.file_path).with_extension("sav");
    match index {
      0 => BackupMedia::Eeprom(EepromController::new(backup_path)),
      1 => BackupMedia::Sram(BackupFile::new(32 * 1024, backup_path)),
      // TODO
      // 2 => BackupMedia::Flash(Flash::new(self.file_path)), // regular flash
      // 3 => BackupMedia::Flash(Flash::new(self.file_path)), // flash 512
      // 4 => BackupMedia::Flash(Flash::new(self.file_path)), // flash 1024
      _ => BackupMedia::Undetected
    }
  }
}