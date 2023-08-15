use std::{fs::{File, self}, io::{Read, SeekFrom, Seek, Write}, path::Path};

pub struct BackupFile {
  size: usize,
  file: File,
  buffer: Vec<u8>
}

impl BackupFile {
  pub fn new(size: usize, file_path: &String) -> Self {
    if !Path::new(file_path).is_file() {
      let mut file = File::create(file_path).unwrap();
      file.write_all(&vec![0xff; size]).unwrap();
    }

    let mut file = fs::OpenOptions::new()
      .read(true)
      .write(true)
      .open(file_path)
      .unwrap();

    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).unwrap();
    buffer.resize(size, 0xff);

    Self {
      file,
      buffer,
      size
    }
  }

  pub fn read(&self, offset: usize) -> u8 {
    self.buffer[offset]
  }

  pub fn write(&mut self, offset: usize, value: u8) {
    self.buffer[offset] = value;

    self.file.seek(SeekFrom::Start(offset as u64)).unwrap();
    self.file.write_all(&[value]).unwrap();
  }

  pub fn resize(&mut self, new_size: usize) {
    self.size = new_size;
    self.buffer.resize(new_size, 0xff);
    self.flush();
  }

  pub fn flush(&mut self) {
    self.file.seek(SeekFrom::Start(0)).unwrap();
    self.file.write_all(&self.buffer).unwrap();
  }
}