use crate::{
  apu::registers::sound_control_dma::SoundControlDma, cartridge::BackupMedia, cpu::{
    dma::dma_channels::AddressType, registers::interrupt_enable_register::InterruptEnableRegister
  },
  gpu::{
    registers::{
      bg_control_register::BgControlRegister,
      display_status_register::DisplayStatusRegister,
      window_in_register::WindowInRegister,
      window_out_register::WindowOutRegister
    },
    VRAM_SIZE
  },
  number::Number
};

use super::CPU;

impl CPU {
  pub fn mem_read_32(&mut self, address: u32) -> u32 {
    match address {
      0x400_0000..=0x4ff_ffff => self.io_read_16(address) as u32 | (self.io_read_16(address + 2) as u32) << 16,
      _ => self.mem_read::<u32>(address)
    }
  }

  pub fn mem_read_16(&mut self, address: u32) -> u16 {
    match address {
      0x400_0000..=0x4ff_ffff => self.io_read_16(address),
      0xd00_0000..=0xdff_ffff if self.cartridge.rom.len() <= (16 * 1024 * 1024) || address >= 0xdff_ff00 => {
        if let BackupMedia::Eeprom(eeprom_controller) = &mut self.cartridge.backup {
          return eeprom_controller.read(address);
        }
        0
      },
      _ => self.mem_read::<u16>(address)
    }
  }

  pub fn mem_read<T: Number>(&mut self, address: u32) -> T {
    match address {
      0..=0x3fff => unsafe { *(&self.bios[address as usize] as *const u8 as *const T) },
      0x200_0000..=0x2ff_ffff => unsafe { *(&self.board_wram[(address & 0x3_ffff) as usize] as *const u8 as *const T) },
      0x300_0000..=0x3ff_ffff => {
        unsafe { *(&self.chip_wram[(address & 0x7fff) as usize] as *const u8 as *const T) }
      },
      0x500_0000..=0x5ff_ffff => unsafe { *(&self.gpu.palette_ram[(address & 0x3ff) as usize] as *const u8 as *const T) },
      0x600_0000..=0x6ff_ffff => {
        let mut offset = address % VRAM_SIZE as u32;

        if offset > 0x18000 {
          offset -= 0x8000
        }

        unsafe { *(&self.gpu.vram[offset as usize] as *const u8 as *const T) }
      }
      0x700_0000..=0x7ff_ffff => unsafe { *(&self.gpu.oam_ram[(address & 0x3ff) as usize] as *const u8 as *const T) },
      0x800_0000..=0xdff_ffff => {
        let offset = address & 0x01ff_ffff;
        if offset >= self.cartridge.rom.len() as u32 {
          let x = (address / 2) & 0xffff;
          if address & 1 != 0 {
            num::cast::<u32, T>(x >> 8).unwrap()
          } else {
            num::cast::<u32, T>(x).unwrap()
          }
        } else {
          unsafe { *(&self.cartridge.rom[offset as usize] as *const u8 as *const T) }
        }
      }
      0xe00_0000..=0xeff_ffff | 0xf00_0000..=0xfff_ffff => {
        if let BackupMedia::Sram(sram) = &mut self.cartridge.backup {
          sram.read((address & 0x7fff) as usize)
        } else if let BackupMedia::Flash(flash) = &mut self.cartridge.backup {
          flash.read(address)
        } else {
          num::zero()
        }
      }
      // 0x1000_0000..=0xffff_ffff => panic!("unused memory"),
      _ => {
        // println!("reading from unsupported address: {:X}", address);
        num::zero()
      }
    }
  }

  pub fn mem_read_8(&mut self, address: u32) -> u8 {
    match address {
      0x400_0000..=0x4ff_ffff => self.io_read_8(address),
      // 0x1000_0000..=0xffff_ffff => panic!("unused memory"),
      _ => self.mem_read::<u8>(address)
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
      0x400_0048 => self.gpu.winin.bits(),
      0x400_004a => self.gpu.winout.bits(),
      0x400_0050 => self.gpu.bldcnt.value,
      0x400_0088 => self.apu.sound_bias,
      0x400_00ba => self.dma.channels[0].dma_control.bits(),
      0x400_00c6 => self.dma.channels[1].dma_control.bits(),
      0x400_00d2 => self.dma.channels[2].dma_control.bits(),
      0x400_00de => self.dma.channels[3].dma_control.bits(),
      0x400_0100 => self.timers.t[0].value,
      0x400_0102 => self.timers.t[0].timer_ctl.bits(),
      0x400_0104 => self.timers.t[1].value,
      0x400_0106 => self.timers.t[1].timer_ctl.bits(),
      0x400_0108 => self.timers.t[2].value,
      0x400_010a => self.timers.t[2].timer_ctl.bits(),
      0x400_010c => self.timers.t[3].value,
      0x400_0080 => self.apu.soundcnt_l.value,
      0x400_0082 => self.apu.soundcnt_h.bits(),
      0x400_0084 => {
        let value = if self.apu.fifo_enable { 1 } else { 0 };

        value << 7
      }
      0x400_010e => self.timers.t[3].timer_ctl.bits(),
      0x400_0130 => self.key_input.bits(),
      0x400_0200 => self.interrupt_enable.bits(),
      0x400_0202 => self.interrupt_request.bits(),
      0x400_0204 => self.waitcnt.value,
      0x400_0208 => if self.interrupt_master_enable { 1 } else { 0 },
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

    match address {
      0x400_0000..=0x4ff_ffff => {
        self.io_write_16(address, lower);
        self.io_write_16(address + 2, upper);
      }
      0xe00_0000..=0xeff_ffff | 0xf00_0000..=0xfff_ffff => {
        self.mem_write_16(address, lower);
        self.mem_write_16(address + 2, upper);
      }
      _ => self.mem_write::<u32>(address, val)
    }
  }

  pub fn mem_write_16(&mut self, address: u32, val: u16) {
    let upper = (val >> 8) as u8;
    let lower = (val & 0xff) as u8;

    match address {
      0x400_0000..=0x4ff_ffff => self.io_write_16(address, val),
      0xd00_0000..=0xdff_ffff if self.cartridge.rom.len() <= (16 * 1024 * 1024) || address >= 0xdff_ff00 => {
        if let BackupMedia::Eeprom(eeprom_controller) = &mut self.cartridge.backup {
          eeprom_controller.write(address, val);
        }
      }
      0xe00_0000..=0xeff_ffff | 0xf00_0000..=0xfff_ffff => {
        self.mem_write_8(address, lower);
        self.mem_write_8(address + 1, upper);
      }
      _ => self.mem_write::<u16>(address, val)
    }
  }

  pub fn mem_write_8(&mut self, address: u32, val: u8) {
    match address {
      0x400_0000..=0x4ff_ffff => self.io_write_8(address, val),
      0xe00_0000..=0xeff_ffff | 0xf00_0000..=0xfff_ffff => {
        if let BackupMedia::Sram(sram) = &mut self.cartridge.backup {
          sram.write((address & 0x7fff) as usize, val);
        } else if let BackupMedia::Flash(flash) = &mut self.cartridge.backup {
          flash.write(address, val);
        }
      }
      _ => self.mem_write::<u8>(address, val)
    }
  }

  pub fn mem_write<T: Number>(&mut self, address: u32, val: T) {
    match address {
      0x200_0000..=0x2ff_ffff => {
        unsafe { *(&mut self.board_wram[(address & 0x3_ffff) as usize] as *mut u8 as *mut T) = val };
      }
      0x500_0000..=0x5ff_ffff => {
        let base_address = address & 0x3fe;
        // self.gpu.palette_ram[base_address as usize] = lower;
        // self.gpu.palette_ram[(base_address + 1) as usize] = upper;

        unsafe { *(&mut self.gpu.palette_ram[base_address as usize] as *mut u8 as *mut T) = val };
      }
      0x600_0000..=0x6ff_ffff => {
        let mut offset = address % VRAM_SIZE as u32;

        if offset > 0x18000 {
          offset -= 0x8000
        }

        unsafe { *(&mut self.gpu.vram[offset as usize] as *mut u8 as *mut T) = val };
      }
      0x700_0000..=0x7ff_ffff => {
        let base_address = address & 0x3fe;
        // self.gpu.oam_ram[base_address as usize] = lower;
        // self.gpu.oam_ram[(base_address+ 1) as usize] = upper;

        unsafe { *(&mut self.gpu.oam_ram[base_address as usize] as *mut u8 as *mut T) = val };
      }
      0x300_0000..=0x3ff_ffff => {
        unsafe { *(&mut self.chip_wram[(address & 0x7fff) as usize] as *mut u8 as *mut T) = val };
      },
      _ => {
        // println!("writing go unsupported address: {:X}", address);
      }
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

        let new_value = existing & 0xffff | (((value & 0xfff) as i32) << 20) >> 4;

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
      0x400_0010 => self.gpu.bgxofs[0] = value & 0b111111111,
      0x400_0012 => self.gpu.bgyofs[0] = value & 0b111111111,
      0x400_0014 => self.gpu.bgxofs[1] = value & 0b111111111,
      0x400_0016 => self.gpu.bgyofs[1] = value & 0b111111111,
      0x400_0018 => self.gpu.bgxofs[2] = value & 0b111111111,
      0x400_001a => self.gpu.bgyofs[2] = value & 0b111111111,
      0x400_001c => self.gpu.bgxofs[3] = value & 0b111111111,
      0x400_001e => self.gpu.bgyofs[3] = value & 0b111111111,
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
      0x400_0040 => self.gpu.winh[0].write(value),
      0x400_0042 => self.gpu.winh[1].write(value),
      0x400_0044 => self.gpu.winv[0].write(value),
      0x400_0046 => self.gpu.winv[1].write(value),
      0x400_0048 => self.gpu.winin = WindowInRegister::from_bits_retain(value),
      0x400_004a => self.gpu.winout = WindowOutRegister::from_bits_retain(value),
      0x400_0050 => self.gpu.bldcnt.write(value),
      0x400_0052 => self.gpu.bldalpha.write(value),
      0x400_0054 => self.gpu.bldy.write(value),
      0x400_0080 => self.apu.soundcnt_l.write(value),
      0x400_0082 => {
        self.apu.soundcnt_h = SoundControlDma::from_bits_retain(value);

        self.apu.on_soundcnt_h_write();
      }
      0x400_0084 => self.apu.fifo_enable = (value >> 7) & 0b1 == 1,
      0x400_0088 => self.apu.write_sound_bias(value),
      0x400_00a0 | 0x400_00a2 => {
        self.apu.fifo_a.write((value & 0xff) as i8);
        self.apu.fifo_a.write(((value >> 8) & 0xff) as i8);
      },
      0x400_00a4 | 0x400_00a6 => {
        self.apu.fifo_b.write((value & 0xff) as i8);
        self.apu.fifo_b.write(((value >> 8) & 0xff) as i8);
      },
      0x400_00b0 => self.dma.set_source_address(0, value, AddressType::Low),
      0x400_00b2 => self.dma.set_source_address(0, value, AddressType::High),
      0x400_00b4 => self.dma.set_destination_address(0, value, AddressType::Low),
      0x400_00b6 => self.dma.set_destination_address(0, value, AddressType::High),
      0x400_00b8 => self.dma.channels[0].word_count = value,
      0x400_00ba => self.dma.channels[0].write_control(value),
      0x400_00bc => self.dma.set_source_address(1, value, AddressType::Low),
      0x400_00be => self.dma.set_source_address(1, value, AddressType::High),
      0x400_00c0 => self.dma.set_destination_address(1, value, AddressType::Low),
      0x400_00c2 => self.dma.set_destination_address(1, value, AddressType::High),
      0x400_00c4 => self.dma.channels[1].word_count = value,
      0x400_00c6 => self.dma.channels[1].write_control(value),
      0x400_00c8 => self.dma.set_source_address(2, value, AddressType::Low),
      0x400_00ca => self.dma.set_source_address(2, value, AddressType::High),
      0x400_00cc => self.dma.set_destination_address(2, value, AddressType::Low),
      0x400_00ce => self.dma.set_destination_address(2, value, AddressType::High),
      0x400_00d0 => self.dma.channels[2].word_count = value,
      0x400_00d2 => self.dma.channels[2].write_control(value),
      0x400_00d4 => self.dma.set_source_address(3, value, AddressType::Low),
      0x400_00d6 => self.dma.set_source_address(3, value, AddressType::High),
      0x400_00d8 => self.dma.set_destination_address(3, value, AddressType::Low),
      0x400_00da => self.dma.set_destination_address(3, value, AddressType::High),
      0x400_00dc => self.dma.channels[3].word_count = value,
      0x400_00de => self.dma.channels[3].write_control(value),
      0x400_0100 => self.timers.t[0].reload_timer_value(value),
      0x400_0102 => self.timers.t[0].write_timer_control(value, &mut self.scheduler),
      0x400_0104 => self.timers.t[1].reload_timer_value(value),
      0x400_0106 => self.timers.t[1].write_timer_control(value, &mut self.scheduler),
      0x400_0108 => self.timers.t[2].reload_timer_value(value),
      0x400_010a => self.timers.t[2].write_timer_control(value, &mut self.scheduler),
      0x400_010c => self.timers.t[3].reload_timer_value(value),
      0x400_010e => self.timers.t[3].write_timer_control(value, &mut self.scheduler),
      0x400_0200 => self.interrupt_enable = InterruptEnableRegister::from_bits_retain(value),
      0x400_0202 => self.clear_interrupts(value),
      0x400_0204 => {
        self.waitcnt.value = value;
        self.cycle_luts.update_tables(&self.waitcnt);
      }
      0x400_0208 => self.interrupt_master_enable = value != 0,
      0x400_0300 => self.post_flag = if value > 0 { 1 } else { 0 },
      0x400_0301 => {
        if value >> 7 & 0b1 == 0 {
          self.is_halted = true;
        } else {
          panic!("STOP not implemented");
        }
      }
      _ => {
        // println!("io register not implemented: {:X}", address)
      }
    }
  }

  pub fn io_write_8(&mut self, address: u32, value: u8) {
    let address = if address & 0xffff == 0x8000 {
      0x400_0800
    } else {
      address
    };

    // println!("im being called with address {:X}", address);

    match address {
      0x400_00a0..=0x400_00a3 => {
        self.apu.fifo_a.write(value as i8);
      },
      0x400_00a4..=0x400_00a7 => {
        self.apu.fifo_b.write(value as i8);
      }
      _ => {
        let mut temp = self.mem_read_16(address & !(0b1));

        temp = if address & 0b1 == 1 {
          (temp & 0xff) | (value as u16) << 8
        } else {
          (temp & 0xff00) | value as u16
        };

        self.mem_write_16(address & !(0b1), temp);
      }
    }

    // todo: implement sound
  }
}