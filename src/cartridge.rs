use self::{eeprom_controller::EepromController, flash::Flash, backup_file::BackupFile};

pub mod eeprom_controller;
pub mod flash;
pub mod backup_file;

pub struct Cartridge {
  pub rom: Vec<u8>,
  pub backup: BackupMedia,
  pub file_path: String
}

// pub enum BackupType {
//   Eeprom = 0,
//   Sram = 1,
//   Flash = 2,
//   Flash512 = 3,
//   Flash1024 = 4,
//   Undetected = 5
// }

pub enum BackupMedia {
  Eeprom(EepromController),
  Flash(Flash),
  Sram(BackupFile),
  Undetected
}


impl Cartridge {
  pub fn detect_backup_media(&mut self) {
    const BACKUP_MEDIA: &[&str] = &["EEPROM", "SRAM", "FLASH_", "FLASH512_", "FLASH1M_"];

    for i in 0..5 {
      let needle = BACKUP_MEDIA[i].as_bytes();

      if let Some(_) = self.rom.windows(needle.len()).position(|window| window == needle) {
        self.backup = self.detect_backup(i);
        break;
      }
    }
  }

  fn detect_backup(&self, index: usize) -> BackupMedia {
    match index {
      0 => BackupMedia::Eeprom(EepromController::new(&self.file_path)),
      // TODO
      // 1 => BackupMedia::Sram(BackupFile::new(self.file_path)),
      // 2 => BackupMedia::Flash(Flash::new(self.file_path)), // regular flash
      // 3 => BackupMedia::Flash(Flash::new(self.file_path)), // flash 512
      // 4 => BackupMedia::Flash(Flash::new(self.file_path)), // flash 1024
      _ => BackupMedia::Undetected
    }
  }
}