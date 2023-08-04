use super::{CPU, PSRRegister, PC_REGISTER, SP_REGISTER, LR_REGISTER, MemoryAccess};

impl CPU {
  fn decode_thumb(&mut self, format: u16) -> fn(&mut CPU, instruction: u16) -> Option<MemoryAccess> {
    if format & 0b11111000 == 0b00011000 {
      CPU::add_subtract
    } else if format & 0b11100000 == 0 {
      CPU::move_shifted_register
    } else if format & 0b11100000 == 0b00100000 {
      CPU::move_compare_add_sub_imm
    } else if format & 0b11111100 == 0b01000000 {
      CPU::alu_operations
    } else if format & 0b11111100 == 0b01000100 {
      CPU::hi_register_ops
    } else if format & 0b11111000 == 0b01001000 {
      CPU::pc_relative_load
    } else if format & 0b11110010 == 0b01010000 {
      CPU::load_store_reg_offset
    } else if format & 0b11110010 == 0b01010010 {
      CPU::load_store_signed_byte_halfword
    } else if format & 0b11100000 == 0b01100000 {
      CPU::load_store_immediate_offset
    } else if format & 0b11110000 == 0b10000000 {
      CPU::load_store_halfword
    } else if format & 0b11110000 == 0b10010000 {
      CPU::sp_relative_load_store
    } else if format & 0b11110000 == 0b10100000 {
      CPU::load_address
    } else if format == 0b10110000 {
      CPU::add_offset_to_sp
    } else if format & 0b11110110 == 0b10110100 {
      CPU::push_pop_registers
    } else if format & 0b11110000 == 0b11000000 {
      CPU::multiple_load_store
    } else if format == 0b11011111 {
      CPU::thumb_software_interrupt
    } else if format & 0b11110000 == 0b11010000 {
      CPU::conditional_branch
    } else if format & 0b11111000 == 0b11100000 {
      CPU::unconditional_branch
    } else if format & 0b11110000 == 0b11110000 {
      CPU::long_branch_link
    } else {
      CPU::panic
    }
  }

  pub fn populate_thumb_lut(&mut self) {
    for i in 0..256 {
      let instr_fn = self.decode_thumb(i);
      self.thumb_lut.push(instr_fn);
    }
  }

  pub fn panic(&mut self, instr: u16) -> Option<MemoryAccess> {
    panic!("unsupported instruction: {:b}", instr);
  }

  fn move_shifted_register(&mut self, instr: u16) -> Option<MemoryAccess> {
    println!("inside move shifted register");
    let op_code = ((instr >> 11) & 0b11) as u8;
    let offset5 = ((instr >> 6) & 0b11111) as u8;
    let rs = ((instr >> 3) & 0b111) as u8;
    let rd = (instr & 0b111) as u8;

    println!("rs = {rs}, rd = {rd}, offset = {offset5}, op_code = {op_code}");
    println!("rs value = {:X}, rd value = {:X}", self.r[rs as usize], self.r[rd as usize]);

    match op_code {
      0 => self.lsl_offset(offset5, rs, rd),
      1 => self.lsr_offset(offset5, rs, rd),
      2 => self.asr_offset(offset5, rs, rd),
      _ => panic!("invalid op")
    }

    self.pc = self.pc.wrapping_add(2);

    Some(MemoryAccess::Sequential)
  }

  fn add_subtract(&mut self, instr: u16) -> Option<MemoryAccess> {
    println!("inside add subtract");
    let op_code = (instr >> 9) & 0b1;
    let rn_offset = (instr >> 6) & 0b111;
    let is_immediate = (instr >> 10) & 0b1 == 1;

    let rs = (instr >> 3) & 0b111;
    let rd = instr & 0b111;

    let operand1 = self.r[rs as usize];
    let operand2 = if is_immediate { rn_offset as u32 } else { self.r[rn_offset as usize] };

    self.r[rd as usize] = if op_code == 0 {
      println!("adding {operand1} and {operand2}");
      self.add(operand1, operand2)
    } else {
      println!("subtracting {operand1} and {operand2}");
      self.subtract(operand1, operand2)
    };

    self.pc = self.pc.wrapping_add(2);

    Some(MemoryAccess::Sequential)
  }

  fn move_compare_add_sub_imm(&mut self, instr: u16) -> Option<MemoryAccess> {
    println!("inside move compare add sub imm");
    let op_code = (instr >> 11) & 0b11;
    let rd = (instr >> 8) & 0b111;
    let offset = instr & 0b11111111;

    println!("op code = {op_code}, rd = {rd}, offset = {offset}");

    match op_code {
      0 => self.mov(rd, offset as u32, true),
      1 => self.cmp(self.r[rd as usize], offset as u32),
      2 => self.r[rd as usize] = self.add(self.r[rd as usize], offset as u32),
      3 => self.r[rd as usize] = self.subtract(self.r[rd as usize], offset as u32),
      _ => unreachable!("impossible")
    }

    self.pc = self.pc.wrapping_add(2);

    Some(MemoryAccess::Sequential)
  }

  fn alu_operations(&mut self, instr: u16) -> Option<MemoryAccess> {
    println!("inside alu ops");
    let op_code = (instr >> 6) & 0b1111;
    let rs = (instr >> 3) & 0b111;
    let rd = instr & 0b111;

    match op_code {
      0 => self.r[rd as usize] = self.and(self.r[rs as usize], self.r[rd as usize]),
      1 => self.r[rd as usize] = self.xor(self.r[rs as usize], self.r[rd as usize]),
      2 => self.r[rd as usize] = self.lsl(self.r[rd as usize], self.r[rs as usize]),
      3 => self.r[rd as usize] = self.lsr(self.r[rd as usize], self.r[rs as usize]),
      4 => self.r[rd as usize] = self.asr(self.r[rd as usize], self.r[rs as usize]),
      5 => self.r[rd as usize] = self.adc(self.r[rd as usize], self.r[rs as usize]),
      6 => self.r[rd as usize] = self.sbc(self.r[rd as usize], self.r[rs as usize]),
      7 => self.r[rd as usize] = self.ror_thumb(self.r[rd as usize], self.r[rs as usize]),
      8 => { self.and(self.r[rs as usize], self.r[rd as usize]); },
      9 => self.r[rd as usize] = self.subtract(0, self.r[rs as usize]),
      10 => { self.subtract(self.r[rd as usize], self.r[rs as usize]); },
      11 => { self.add(self.r[rd as usize], self.r[rs as usize]); },
      12 => self.r[rd as usize] = self.or(self.r[rd as usize], self.r[rs as usize]),
      13 => self.r[rd as usize] = self.mul(self.r[rd as usize], self.r[rs as usize]),
      14 => self.r[rd as usize] = self.bic(self.r[rd as usize] ,self.r[rs as usize]),
      15 => self.r[rd as usize] = self.mvn(self.r[rs as usize]),
      _ => unreachable!("impossible")
    }

    self.pc = self.pc.wrapping_add(2);

    Some(MemoryAccess::Sequential)
  }

  fn hi_register_ops(&mut self, instr: u16) -> Option<MemoryAccess> {
    println!("inside hi register ops");
    let op_code = (instr >> 8) & 0b11;
    let h1 = (instr >> 7) & 0b1;
    let h2 = (instr >> 6) & 0b1;

    let mut source = (instr >> 3) & 0b111;
    let mut destination = instr & 0b111;

    if h1 == 1 {
      destination += 8;
    }
    if h2 == 1 {
      source += 8;
    }

    println!("reading from register {source} and destination {destination}, op code is {op_code}");

    let operand1 = if destination == PC_REGISTER as u16 {
      self.pc
    } else {
      self.r[destination as usize]
    };

    let operand2 = if source == PC_REGISTER as u16 {
      self.pc
    } else {
      self.r[source as usize]
    };

    println!("operand1 = {operand1}, operand2 = {operand2}");

    let mut should_update_pc = true;

    let mut return_result = Some(MemoryAccess::Sequential);

    match op_code {
      0 => {
        let result = operand1.wrapping_add(operand2);
        if destination == PC_REGISTER as u16 {
          self.pc = result & !(0b1);

          should_update_pc = false;
          return_result = None;
        } else {
          self.r[destination as usize] = result;
        }
      }
      1 => { self.subtract(operand1, operand2); }
      2 => {
        self.mov(destination, operand2, false);
        if destination == PC_REGISTER as u16 {
          return_result = None;
          should_update_pc = false;
        }
      }
      3 => {
        self.bx(operand2);
        should_update_pc = false;
        return_result = None;
      },
      _ => unreachable!("can't be")
    }

    if should_update_pc {
      self.pc = self.pc.wrapping_add(2);
    }

    return_result
  }

  fn pc_relative_load(&mut self, instr: u16) -> Option<MemoryAccess> {
    println!("inside pc relative load");
    let rd = (instr >> 8) & 0b111;

    let immediate = (instr & 0xff) << 2;

    let address = (self.pc & !(0b11)) + immediate as u32;

    self.r[rd as usize] = self.mem_read_32(address);

    println!("loaded {:X} into register {rd}", self.r[rd as usize]);

    self.pc = self.pc.wrapping_add(2);

    Some(MemoryAccess::Sequential)
  }

  fn load_store_reg_offset(&mut self, instr: u16) -> Option<MemoryAccess> {
    println!("inside load store reg offset");
    let b = (instr >> 10) & 0b1;
    let l = (instr >> 11) & 0b1;

    let ro = (instr >> 6) & 0b111;
    let rb = (instr >> 3) & 0b111;
    let rd = instr & 0b111;

    let address = self.r[rb as usize].wrapping_add(self.r[ro as usize]);

    if l == 0 {
      println!("writing to address {:X}", address);
    } else {
      println!("reading from address {:X}", address);
    }

    match (l, b) {
      (0, 0) => {
        self.mem_write_32(address, self.r[rd as usize]);
      }
      (0, 1) => {
        self.mem_write_8(address, self.r[rd as usize] as u8);
      }
      (1, 0) => {
        let value = self.mem_read_32(address);

        self.r[rd as usize] = value;
      }
      (1, 1) => {
        let value = self.mem_read_8(address) as u32;

        self.r[rd as usize] = value;
      }
      _ => unreachable!("can't be")
    }

    self.pc = self.pc.wrapping_add(2);

    Some(MemoryAccess::Sequential)
  }

  fn load_store_signed_byte_halfword(&mut self, instr: u16) -> Option<MemoryAccess> {
    println!("inside load store signed byte halfword");
    let h = (instr >> 11) & 0b1;
    let s = (instr >> 10) & 0b1;

    let ro = (instr >> 6) & 0b111;
    let rb = (instr >> 3) & 0b111;
    let rd = instr & 0b111;

    println!("r{ro} = {:X}, r{rb} = {:X}", self.r[ro as usize], self.r[rb as usize]);

    let address = self.r[ro as usize].wrapping_add(self.r[rb as usize]);

    match (s, h) {
      (0, 0) => {
        let value = (self.r[rd as usize] & 0xffff) as u16;
        self.mem_write_16(address & !(0b1), value);
      }
      (0, 1) => {
        let value = self.ldr_halfword(address);

        self.r[rd as usize] = value as u32;
      }
      (1, 0) => {
        let value = self.mem_read_8(address) as i8 as i32;

        self.r[rd as usize] = value as u32;
      }
      (1,1) => {
        // let value = self.mem_read_16(address) as i16 as i32;

        // self.r[rd as usize] = value as u32;
        self.r[rd as usize] = if address & 0b1 != 0 {
          self.mem_read_8(address) as i8 as i32 as u32
        } else {
          self.mem_read_16(address) as i16 as i32 as u32
        };
      }
      _ => unreachable!("can't be")
    }

    self.pc = self.pc.wrapping_add(2);

    Some(MemoryAccess::Sequential)
  }

  fn load_store_immediate_offset(&mut self, instr: u16) -> Option<MemoryAccess> {
    println!("inside load store immediate offset");
    let b = (instr >> 12) & 0b1;
    let l = (instr >> 11) & 0b1;
    let offset = (instr >> 6) & 0b11111;
    let rb = (instr >> 3) & 0b111;
    let rd = instr & 0b111;

    let address = self.r[rb as usize].wrapping_add(offset as u32);

    match (l, b) {
      (0, 0) => {
        self.mem_write_32(address, self.r[rd as usize]);
      }
      (1, 0) => {
        let value = self.mem_read_32(address);

        self.r[rd as usize] = value;
      }
      (0, 1) => {
        let byte  = (self.r[rd as usize] & 0xff) as u8;
        self.mem_write_8(address, byte);
      }
      (1, 1) => {
        let val = self.mem_read_8(address) as u32;

        self.r[rd as usize] = val;
      }
      _ => unreachable!("cannot be")
    }

    self.pc = self.pc.wrapping_add(2);

    Some(MemoryAccess::Sequential)
  }

  fn load_store_halfword(&mut self, instr: u16) -> Option<MemoryAccess> {
    println!("inside load store halfword");
    let l = (instr >> 11) & 0b1;
    let offset = (instr >> 6) & 0b11111;
    let rb = (instr >> 3) & 0b111;
    let rd = instr & 0b111;

    let address = self.r[rb as usize].wrapping_add(offset as u32);

    println!("rd = {rd} and rb = {rb} and offset = {offset}");

    if l == 0 {
      let value = (self.r[rd as usize] & 0xffff) as u16;
      self.mem_write_16(address & !(0b1), value);
    } else {
      let value = self.ldr_halfword(address) as u32;

      self.r[rd as usize] = value;
    }

    self.pc = self.pc.wrapping_add(2);

    Some(MemoryAccess::Sequential)
  }

  fn sp_relative_load_store(&mut self, instr: u16) -> Option<MemoryAccess> {
    println!("inside sp relative load store");
    let l = (instr >> 11) & 0b1;
    let rd = (instr >> 8) & 0b111;
    let word8 = (instr & 0xff) << 2;

    let address = self.r[SP_REGISTER].wrapping_add(word8 as u32);

    if l == 0 {
      self.mem_write_32(address, self.r[rd as usize]);
    } else {
      let value = self.mem_read_32(address);

      self.r[rd as usize] = value;
    }

    self.pc = self.pc.wrapping_add(2);

    Some(MemoryAccess::Sequential)
  }

  fn load_address(&mut self, instr: u16) -> Option<MemoryAccess> {
    println!("inside load address");
    let sp = (instr >> 11) & 0b1;
    let rd = (instr >> 8) & 0b111;
    let word8 = (instr & 0xff) << 2;

    self.r[rd as usize] = if sp == 1 {
      self.r[SP_REGISTER].wrapping_add(word8 as u32)
    } else {
      let pc_value = (self.pc.wrapping_sub(4) & !(0b1 << 1)) + 4;
      pc_value.wrapping_add(word8 as u32)
    };

    self.pc = self.pc.wrapping_add(2);

    Some(MemoryAccess::Sequential)
  }

  fn add_offset_to_sp(&mut self, instr: u16) -> Option<MemoryAccess> {
    println!("inside add offset to sp");
    let s = (instr >> 7) & 0b1;
    let sword7 = ((instr & 0b1111111) << 2) as i32;

    self.r[SP_REGISTER] = if s == 0 {
      // add immediate to sp
      (self.r[SP_REGISTER] as i32).wrapping_add(sword7) as u32
    } else {
      // subtract immediate from sp
      (self.r[SP_REGISTER] as i32).wrapping_sub(sword7) as u32
    };

    self.pc = self.pc.wrapping_add(2);

    Some(MemoryAccess::Sequential)
  }

  fn push_pop_registers(&mut self, instr: u16) -> Option<MemoryAccess> {
    println!("inside push pop registers");
    let l = (instr >> 11) & 0b1;
    let r = (instr >> 8) & 0b1;
    let register_list = instr & 0xff;

    let mut should_update_pc = true;
    let mut result = Some(MemoryAccess::Sequential);

    // push
    if l == 0 {
      if r == 1 {
        // push LR to the stack
        println!("pushing register 14 to the stack");
        self.push(self.r[LR_REGISTER]);
      }
      for i in (0..8).rev() {
        if (register_list >> i) & 0b1 == 1 {
          println!("pushing register {i} to the stack");
          self.push(self.r[i]);
        }
      }
    } else {
      // pop
      for i in 0..8 {
        if (register_list >> i) & 0b1 == 1 {
          println!("popping register {i} from the stack");
          self.r[i] = self.pop();
        }
      }
      if r == 1 {
        // pop PC off the stack
        println!("popping the pc off da stack");
        self.pc = self.pop();
        self.pc &= !(1);

        // reload the pipeline
        self.reload_pipeline16();

        should_update_pc = false;
        result = None;
      }
    }

    if should_update_pc {
      self.pc = self.pc.wrapping_add(2);
    }

    result
  }

  fn multiple_load_store(&mut self, instr: u16) -> Option<MemoryAccess> {
    println!("inside multiple load store");
    let l = (instr >> 11) & 0b1;
    let rb = (instr >> 8) & 0b111;
    let rlist = instr & 0xff;

    let mut address = self.r[rb as usize] & !(0b11);
    let align_preserve = self.r[rb as usize] & (0b11);

    if rlist != 0 {
      if l == 0 {
        // store
        let mut first = false;

        for r in 0..8 {
          let val = if r != rb {
            self.r[r as usize]
          } else if first {
            first = false;
            address
          } else {
            address + (rlist.count_ones() - 1) * 4
          };

          self.mem_write_32(address, val);

          address += 4;
        }
        self.r[rb as usize] = address + align_preserve;
      } else {
        // load
        for r in 0..8 {
          if (rlist >> r) & 0b1 == 1 {
            let val = self.mem_read_32(address);

            self.r[r] = val;
            address += 4;
          }
        }

        if (rlist >> rb) & 0b1 == 0 {
          self.r[rb as usize] = address + align_preserve;
        }
      }
    } else {
      // from gbatek: Empty Rlist: R15 loaded/stored (ARMv4 only), and Rb=Rb+40h (ARMv4-v5).
      if l == 0 {
        // store PC
        self.mem_write_32(address, self.pc + 2);
      } else {
        // load PC
        let val = self.mem_read_32(address);

        self.pc = val & !(0b1);

        // reload the pipeline
        self.reload_pipeline16();
      }

      address += 0x40;

      self.r[rb as usize] = address + align_preserve;
    }

    self.pc = self.pc.wrapping_add(2);

    Some(MemoryAccess::Sequential)
  }

  fn conditional_branch(&mut self, instr: u16) -> Option<MemoryAccess> {
    println!("inside conditional branch");
    let cond = (instr >> 8) & 0b1111;

    let signed_offset = ((((instr & 0xff) as u32) << 24) as i32) >> 23;

    println!("condition = {cond}");

    match cond {
      0 => self.branch_if(self.cpsr.contains(PSRRegister::ZERO), signed_offset),
      1 => self.branch_if(!self.cpsr.contains(PSRRegister::ZERO), signed_offset),
      2 => self.branch_if(self.cpsr.contains(PSRRegister::CARRY), signed_offset),
      3 => self.branch_if(!self.cpsr.contains(PSRRegister::CARRY), signed_offset),
      4 => self.branch_if(self.cpsr.contains(PSRRegister::NEGATIVE), signed_offset),
      5 => self.branch_if(!self.cpsr.contains(PSRRegister::NEGATIVE), signed_offset),
      6 => self.branch_if(self.cpsr.contains(PSRRegister::OVERFLOW), signed_offset),
      7 => self.branch_if(!self.cpsr.contains(PSRRegister::OVERFLOW), signed_offset),
      8 => self.branch_if(self.cpsr.contains(PSRRegister::CARRY) && !self.cpsr.contains(PSRRegister::ZERO), signed_offset),
      9 => self.branch_if(!self.cpsr.contains(PSRRegister::CARRY) || self.cpsr.contains(PSRRegister::ZERO), signed_offset),
      10 => self.branch_if(self.bge(), signed_offset),
      11 => self.branch_if(self.blt(), signed_offset),
      12 => self.branch_if(self.bgt(), signed_offset),
      13 => self.branch_if(self.ble(), signed_offset),
      _ => panic!("shouldn't happen")
    }
  }

  fn thumb_software_interrupt(&mut self, _instr: u16) -> Option<MemoryAccess> {
    println!("inside software interrupt");

    self.interrupt();

    None
  }

  fn unconditional_branch(&mut self, instr: u16) -> Option<MemoryAccess> {
    println!("inside unconditional branch");
    let address = ((((instr & 0b11111111111) as i32) << 21)) >> 20;

    self.pc = (self.pc as i32).wrapping_add((address) as i32) as u32;

    self.reload_pipeline16();

    None
  }

  fn long_branch_link(&mut self, instr: u16) -> Option<MemoryAccess> {
    println!("inside long branch link");
    let h = (instr >> 11) & 0b1;
    let offset = (instr & 0b11111111111) as i32;

    if h == 0 {
      let address = (offset << 21) >> 9;

      self.r[LR_REGISTER] = (self.pc as i32).wrapping_add(address) as u32;

      self.pc = self.pc.wrapping_add(2);

      Some(MemoryAccess::Sequential)
    } else {
      let address = (offset << 1) as i32;
      let lr_result = (self.pc - 2) | 0b1;

      self.pc = ((self.r[LR_REGISTER] & !1) as i32).wrapping_add(address) as u32;

      self.r[LR_REGISTER] = lr_result;

      self.reload_pipeline16();

      None
    }
  }

  fn bge(&self) -> bool {
    self.cpsr.contains(PSRRegister::NEGATIVE) == self.cpsr.contains(PSRRegister::OVERFLOW)
  }

  fn blt(&self) -> bool {
    self.cpsr.contains(PSRRegister::NEGATIVE) != self.cpsr.contains(PSRRegister::OVERFLOW)
  }

  fn bgt(&self) -> bool {
    self.cpsr.contains(PSRRegister::ZERO) && self.cpsr.contains(PSRRegister::NEGATIVE) == self.cpsr.contains(PSRRegister::OVERFLOW)
  }

  fn ble(&self) -> bool {
    self.cpsr.contains(PSRRegister::ZERO) && self.cpsr.contains(PSRRegister::NEGATIVE) != self.cpsr.contains(PSRRegister::OVERFLOW)
  }

  fn mov(&mut self, rd: u16, val: u32, set_flags: bool) {
    if rd == 15 {
      self.pc = val & !(0b1);

      self.reload_pipeline16();
    } else {
      self.r[rd as usize] = val;
    }

    if set_flags {
      self.set_carry_zero_and_negative_flags(val, self.cpsr.contains(PSRRegister::CARRY));
    }
  }

  fn cmp(&mut self, operand1: u32, operand2: u32) {
    self.subtract(operand1, operand2);
  }

  fn add(&mut self, operand1: u32, operand2: u32) -> u32 {
    let (result, carry) = operand1.overflowing_add(operand2);

    let (_, overflow) = (operand1 as i32).overflowing_add(operand2 as i32);

    self.set_carry_zero_and_negative_flags(result, carry);
    self.cpsr.set(PSRRegister::OVERFLOW, overflow);

    result
  }

  fn adc(&mut self, operand1: u32, operand2: u32) -> u32 {
    let carry_to_add = if self.cpsr.contains(PSRRegister::CARRY) { 1 } else { 0 };

    let (result1, carry1) = operand1.overflowing_add(operand2);
    let (result2, carry2) = result1.overflowing_add(carry_to_add);

    let (temp, overflow1) = (operand1 as i32).overflowing_add(operand2 as i32);
    let (_, overflow2) = temp.overflowing_add(carry_to_add as i32);

    self.cpsr.set(PSRRegister::OVERFLOW, overflow1 || overflow2);
    self.set_carry_zero_and_negative_flags(result2, carry1 || carry2);

    result2
  }

  fn and(&mut self, operand1: u32, operand2: u32) -> u32 {
    let result = operand1 & operand2;

    self.set_carry_zero_and_negative_flags(result, self.cpsr.contains(PSRRegister::CARRY));

    result
  }

  fn xor(&mut self, operand1: u32, operand2: u32) -> u32 {
    let result = operand1 ^ operand2;

    self.set_carry_zero_and_negative_flags(result, self.cpsr.contains(PSRRegister::CARRY));

    result
  }

  fn subtract(&mut self, operand1: u32, operand2: u32) -> u32 {
    let carry = operand2 > operand1;
    let result = operand1.wrapping_sub(operand2);

    let (_, overflow) = (operand1 as i32).overflowing_sub(operand2 as i32);

    self.set_carry_zero_and_negative_flags(result, carry);
    self.cpsr.set(PSRRegister::OVERFLOW, overflow);

    result
  }

  fn sbc(&mut self, operand1: u32, operand2: u32) -> u32 {
    let carry_to_subtract = if self.cpsr.contains(PSRRegister::CARRY) { 0 } else { 1 };

    let (result1, carry1) = operand1.overflowing_sub(operand2);
    let (result2, carry2) = result1.overflowing_sub(carry_to_subtract);

    let (temp, overflow1) = (operand1 as i32).overflowing_sub(operand2 as i32);
    let (_, overflow2) = temp.overflowing_sub(carry_to_subtract as i32);

    self.set_carry_zero_and_negative_flags(result2, carry1 || carry2);
    self.cpsr.set(PSRRegister::OVERFLOW, overflow1 || overflow2);

    result2
  }

  fn lsl_offset(&mut self, offset: u8, rs: u8, rd: u8) {
    self.r[rd as usize] = self.lsl(self.r[rs as usize], offset as u32);
  }

  fn lsl(&mut self, operand: u32, shift: u32) -> u32 {
    let carry_shift = 32 - shift;
    let carry = shift != 0 && (operand >> carry_shift) & 0b1 == 1;

    let result = if shift < 32 { operand << shift } else { 0 };

    self.set_carry_zero_and_negative_flags(result, carry);

    result
  }

  fn ror_thumb(&mut self, operand: u32, shift: u32) -> u32 {
    let result = operand.rotate_right(shift);
    let carry = (result >> 31) & 0b1 == 1;

    self.set_carry_zero_and_negative_flags(result, carry);

    result
  }

  fn or(&mut self, operand1: u32, operand2: u32) -> u32 {
    let result = operand1 | operand2;

    self.set_carry_zero_and_negative_flags(result, self.cpsr.contains(PSRRegister::CARRY));

    result
  }

  fn lsr_offset(&mut self, offset: u8, rs: u8, rd: u8) {
    self.r[rd as usize] = self.lsr(self.r[rs as usize], offset as u32);
  }

  fn lsr(&mut self, operand: u32, shift: u32) -> u32 {
    let carry = if shift != 0 { ((operand >> (shift - 1)) & 0b1) == 1 } else { false };
    let result = operand >> shift;

    self.set_carry_zero_and_negative_flags(result, carry);

    result
  }

  fn set_carry_zero_and_negative_flags(&mut self, result: u32, carry: bool) {
    self.cpsr.set(PSRRegister::CARRY, carry);
    self.cpsr.set(PSRRegister::ZERO, result == 0);
    self.cpsr.set(PSRRegister::NEGATIVE, (result >> 31 & 0b1) == 1);
  }

  fn asr(&mut self, operand: u32, shift: u32) -> u32 {
    let carry = ((operand) >> (shift - 1)) & 0b1 == 1;
    let result = (operand as i32 >> shift) as u32;

    self.set_carry_zero_and_negative_flags(result, carry);

    result
  }

  fn mul(&mut self, operand1: u32, operand2: u32) -> u32 {
    let (result, _) = operand1.overflowing_mul(operand2);

    self.cpsr.set(PSRRegister::CARRY, false);
    self.cpsr.set(PSRRegister::OVERFLOW, false);

    result
  }

  fn bic(&mut self, operand1: u32, operand2: u32) -> u32 {
    let result = operand1 & !operand2;

    self.set_carry_zero_and_negative_flags(result, self.cpsr.contains(PSRRegister::CARRY));

    result
  }

  fn mvn(&mut self, operand2: u32) -> u32 {
    let result = !operand2;

    self.set_carry_zero_and_negative_flags(result, self.cpsr.contains(PSRRegister::CARRY));

    result
  }

  fn asr_offset(&mut self, offset: u8, rs: u8, rd: u8) {
    let carry = ((self.r[rs as usize]) >> (offset - 1)) & 0b1 == 1;
    let val = (self.r[rs as usize] as i32 >> offset) as u32;

    self.r[rd as usize] = val;

    self.set_carry_zero_and_negative_flags(val, carry)
  }

  fn bx(&mut self, source: u32) {
    // if thumb mode
    if source & 0b1 == 1 {
      let address = source & !(0b1);

      self.cpsr.set(PSRRegister::STATE_BIT, true);

      self.pc = address;

      // reload the pipeline here
      self.reload_pipeline16();
    } else {
      println!("switching to ARM");
      // if ARM mode
      let address = source & !(0b11);

      self.cpsr.set(PSRRegister::STATE_BIT, false);

      self.pc = address;

      // reload pipeline
      self.reload_pipeline32();
    }
  }

  fn ldr_halfword(&mut self, address: u32) -> u16 {
    if address & 0b1 != 0 {
      let rotation = (address & 0b1) << 3;

      let value = self.mem_read_16(address & !(0b1));

      let mut carry = self.cpsr.contains(PSRRegister::CARRY);
      let return_val = self.ror(value as u32, rotation as u8, &mut carry) as u16;

      self.cpsr.set(PSRRegister::CARRY, carry);

      return_val
    } else {
      self.mem_read_16(address)
    }
  }

  fn branch_if(&mut self, cond: bool, offset: i32) -> Option<MemoryAccess> {
    if cond {
      println!("branching");
      self.pc = (self.pc as i32).wrapping_add(offset) as u32;

      // reload pipeline
      self.reload_pipeline16();

      return None
    }

    println!("not branching");

    self.pc = self.pc.wrapping_add(2);

    Some(MemoryAccess::Sequential)
  }
}