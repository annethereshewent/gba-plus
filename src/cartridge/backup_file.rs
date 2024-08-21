use std::{fs::{File, self}, io::{Read, SeekFrom, Seek, Write}, path::PathBuf};

use crate::number::Number;

pub struct BackupFile {
  pub size: usize,
  file: Option<File>,
  pub buffer: Vec<u8>,
  pub has_saved: bool
}

impl BackupFile {
  pub fn new(size: usize, file_path: Option<PathBuf>) -> Self {
    let mut buffer = Vec::new();
    let file = if let Some(file_path) = file_path {
      if !file_path.is_file() {
        let mut file = File::create(&file_path).unwrap();
        file.write_all(&vec![0xff; size]).unwrap();
      }

      let mut file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(file_path)
        .unwrap();

      file.read_to_end(&mut buffer).unwrap();

      Some(file)
    } else {
      None
    };

    buffer.resize(size, 0xff);

    Self {
      file,
      buffer,
      size,
      has_saved: false
    }
  }

  pub fn read<T: Number>(&self, offset: usize) -> T {
    unsafe { *(&self.buffer[offset] as *const u8 as *const T) }
  }

  pub fn write(&mut self, offset: usize, value: u8) {
    self.has_saved = true;
    self.buffer[offset] = value;

    if let Some(file) = &mut self.file {
      file.seek(SeekFrom::Start(offset as u64)).unwrap();
      file.write_all(&[value]).unwrap();
    }
  }

  pub fn resize(&mut self, new_size: usize) {
    self.size = new_size;
    self.buffer.resize(new_size, 0xff);
    self.flush();
  }

  pub fn flush(&mut self) {
    if let Some(file) = &mut self.file {
      file.seek(SeekFrom::Start(0)).unwrap();
      file.write_all(&self.buffer).unwrap();
    }
  }
}