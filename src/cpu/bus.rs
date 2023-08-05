use crate::gpu::{registers::display_status_register::DisplayStatusRegister, VRAM_SIZE};

use super::CPU;

impl CPU {
  pub fn mem_read_32(&mut self, address: u32) -> u32 {
    self.mem_read_16(address) as u32 | ((self.mem_read_16(address + 2) as u32) << 16)
  }

  pub fn mem_read_16(&mut self, address: u32) -> u16 {
    match address {
      0x400_0000..=0x400_03fe => self.io_read_16(address),
      _ => self.mem_read_8(address) as u16 | ((self.mem_read_8(address + 1) as u16) << 8)
    }
  }

  pub fn mem_read_8(&mut self, address: u32) -> u8 {
    match address {
      0..=0x3fff => self.bios[address as usize],
      0x200_0000..=0x2ff_ffff => self.board_wram[(address & 0x3_ffff) as usize],
      0x300_0000..=0x3ff_ffff => self.chip_wram[(address & 0x7fff) as usize],
      0x400_0000..=0x400_03fe => self.io_read_8(address),
      0x800_0000..=0xdff_ffff => {
        let offset = address & 0x01ff_ffff;
        if offset >= self.rom.len() as u32 {
          let x = (address / 2) & 0xffff;
          if address & 1 != 0 {
              (x >> 8) as u8
          } else {
              x as u8
          }
        } else {
          self.rom[(address & 0x01ff_ffff) as usize]
        }
      }
      0x10_000_000..=0xff_fff_fff => panic!("unused memory"),
      _ => {
        println!("reading from unsupported address: {:X}", address);
        0
      }
    }
  }

  fn io_read_16(&mut self, address: u32) -> u16 {
    let address = if address & 0xfffe == 0x8000 {
      0x400_0800
    } else {
      address
    };

    match address {
      0x400_0004 => self.gpu.dispstat.bits(),
      0x400_0006 => self.gpu.vcount,
      0x400_0088 => 0x200,
      0x400_0300 => self.post_flag,
      _ => {
        println!("io register not implemented: {:X}", address);
        0
      }
    }
  }

  fn io_read_8(&mut self, address: u32) -> u8 {
    let val = self.io_read_16(address & !(0b1));

    if address & 0b1 == 1 {
      (val >> 8) as u8
    } else {
      (val & 0xff) as u8
    }
  }

  pub fn mem_write_32(&mut self, address: u32, val: u32) {
    let upper = (val >> 16) as u16;
    let lower = (val & 0xffff) as u16;

    self.mem_write_16(address, lower);
    self.mem_write_16(address + 2, upper);
  }

  pub fn mem_write_16(&mut self, address: u32, val: u16) {
    let upper = (val >> 8) as u8;
    let lower = (val & 0xff) as u8;

    match address {
      0x400_0000..=0x400_03ff => self.io_write_16(address, val),
      0x500_0000..=0x500_03ff => {
        let base_address = address & 0x3fe;
        self.gpu.palette_ram[base_address as usize] = lower;
        self.gpu.palette_ram[(base_address + 1) as usize] = upper;
      }
      0x600_0000..=0x601_7fff => {
        let mut offset = address % VRAM_SIZE as u32;

        if offset > 0x18000 {
          offset -= 0x8000
        }

        self.gpu.vram[offset as usize] = lower;
        self.gpu.vram[(offset + 1) as usize] = upper;
      }
      0x700_0000..=0x700_03ff => {
        let base_address = address & 0x3fe;
        self.gpu.oam_ram[base_address as usize] = lower;
        self.gpu.oam_ram[(base_address+ 1) as usize] = upper;
      }
      _ => {
        self.mem_write_8(address, lower);
        self.mem_write_8(address + 1, upper);
      }
    }
  }

  pub fn mem_write_8(&mut self, address: u32, val: u8) {
    match address {
      0x200_0000..=0x203_ffff => self.board_wram[(address & 0x3_ffff) as usize] = val,
      0x300_0000..=0x300_7fff => self.chip_wram[(address & & 0x7fff) as usize] = val,
      0x400_0000..=0x400_03ff => self.io_write_8(address, val),
      0x500_0000..=0x500_03ff => self.mem_write_16(address & 0x3fe, (val as u16) * 0x101),
      0x600_0000..=0x601_7fff => {
        let mut offset = address % VRAM_SIZE as u32;

        if offset > 0x18000 {
          offset -= 0x8000
        }

        self.mem_write_16(offset, (val as u16) * 0x101);
      }
      _ => ()
    }
  }

  pub fn io_write_16(&mut self, address: u32, value: u16) {
    let address = if address & 0xfffe == 0x8000 {
      0x400_0800
    } else {
      address
    };

    match address {
      0x400_0004 => self.gpu.dispstat = DisplayStatusRegister::from_bits_retain(value),
      0x400_0006 => (),
      0x400_0088 => (),
      0x400_0300 => self.post_flag = if value > 0 { 1 } else { 0 },
      _ => println!("io register not implemented: {:X}", address)
    }
  }

  pub fn io_write_8(&mut self, address: u32, _value: u8) {
    let _address = if address & 0xffff == 0x8000 {
      0x400_0800
    } else {
      address
    };

    // todo: implement sound
  }
}