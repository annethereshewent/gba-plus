use crate::{gpu::{registers::{display_status_register::DisplayStatusRegister, bg_control_register::BgControlRegister}, VRAM_SIZE}, cpu::registers::{interrupt_enable_register::InterruptEnableRegister, interrupt_request_register::InterruptRequestRegister}};

use super::CPU;

impl CPU {
  pub fn mem_read_32(&mut self, address: u32) -> u32 {
    self.mem_read_16(address) as u32 | ((self.mem_read_16(address + 2) as u32) << 16)
  }

  pub fn mem_read_16(&mut self, address: u32) -> u16 {
    match address {
      0x400_0000..=0x400_03ff => self.io_read_16(address),
      _ => self.mem_read_8(address) as u16 | ((self.mem_read_8(address + 1) as u16) << 8)
    }
  }

  pub fn  mem_read_8(&mut self, address: u32) -> u8 {
    match address {
      0..=0x3fff => self.bios[address as usize],
      0x200_0000..=0x2ff_ffff => self.board_wram[(address & 0x3_ffff) as usize],
      0x300_0000..=0x3ff_ffff => self.chip_wram[(address & 0x7fff) as usize],
      0x400_0000..=0x400_03fe => self.io_read_8(address),
      0x500_0000..=0x500_03ff => self.gpu.palette_ram[(address & 0x3ff) as usize],
      0x600_0000..=0x601_7fff => {
        let mut offset = address % VRAM_SIZE as u32;

        if offset > 0x18000 {
          offset -= 0x8000
        }

        self.gpu.vram[offset as usize]
      }
      0x700_0000..=0x700_03ff => self.gpu.oam_ram[(address & 0x3ff) as usize],
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
      // 0x1000_0000..=0xffff_ffff => panic!("unused memory"),
      _ => {
        // println!("reading from unsupported address: {:X}", address);
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
      0x400_0000 => self.gpu.dispcnt.bits(),
      0x400_0004 => self.gpu.dispstat.bits(),
      0x400_0006 => self.gpu.vcount,
      0x400_0008 => self.gpu.bgcnt[0].bits(),
      0x400_000a => self.gpu.bgcnt[1].bits(),
      0x400_000c => self.gpu.bgcnt[2].bits(),
      0x400_000e => self.gpu.bgcnt[3].bits(),
      0x400_0088 => 0x200,
      0x400_0300 => self.post_flag,
      _ => {
        // println!("io register not implemented: {:X}", address);
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

    let gpu = &mut self.gpu;

    macro_rules! write_bg_reference_point {
      (low $coordinate:ident $internal:ident $i:expr) => {{
        let existing = gpu.bg_props[$i].$coordinate as u32;

        let new_value = ((existing & 0xffff0000) + (value as u32)) as i32;

        gpu.bg_props[$i].$coordinate = new_value;
        gpu.bg_props[$i].$internal = new_value;
      }};
      (high $coordinate:ident $internal:ident $i:expr) => {{
        let existing = gpu.bg_props[$i].$coordinate;

        let new_value = existing & 0xffff | ((((value & 0xfff) as i32) << 20) as i32) >> 4;

        gpu.bg_props[$i].$coordinate = new_value;
        gpu.bg_props[$i].$internal = new_value;
      }}
    }

    match address {
      0x400_0000 => self.gpu.write_dispcnt(value),
      0x400_0004 => self.gpu.dispstat = DisplayStatusRegister::from_bits_retain(value),
      0x400_0006 => (),
      0x400_0008 => self.gpu.bgcnt[0] = BgControlRegister::from_bits_retain(value),
      0x400_000a => self.gpu.bgcnt[1] = BgControlRegister::from_bits_retain(value),
      0x400_000c => self.gpu.bgcnt[2] = BgControlRegister::from_bits_retain(value),
      0x400_000e => self.gpu.bgcnt[3] = BgControlRegister::from_bits_retain(value),
      0x400_0020 => self.gpu.bg_props[0].dx = value as i16,
      0x400_0022 => self.gpu.bg_props[0].dmx = value as i16,
      0x400_0024 => self.gpu.bg_props[0].dy = value as i16,
      0x400_0026 => self.gpu.bg_props[0].dmy = value as i16,
      0x400_0028 => write_bg_reference_point!(low x internal_x 0),
      0x400_002a => write_bg_reference_point!(high x internal_x 0),
      0x400_002c => write_bg_reference_point!(low y internal_y 0),
      0x400_002e => write_bg_reference_point!(high y internal_y 0),
      0x400_0030 => self.gpu.bg_props[1].dx = value as i16,
      0x400_0032 => self.gpu.bg_props[1].dmx = value as i16,
      0x400_0034 => self.gpu.bg_props[1].dy = value as i16,
      0x400_0036 => self.gpu.bg_props[1].dmy = value as i16,
      0x400_0038 => write_bg_reference_point!(low x internal_x 1),
      0x400_003a => write_bg_reference_point!(high x internal_x 1),
      0x400_003c => write_bg_reference_point!(low y internal_y 1),
      0x400_003e => write_bg_reference_point!(high y internal_y 1),
      0x400_0088 => (),
      0x400_0200 => self.interrupt_enable = InterruptEnableRegister::from_bits_retain(value),
      0x400_0202 => self.interrupt_request = InterruptRequestRegister::from_bits_retain(value),
      0x400_0208 => self.interrupt_master_enable = value != 0,
      0x400_0300 => self.post_flag = if value > 0 { 1 } else { 0 },
      _ => { println!("io register not implemented: {:X}", address) }
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