use crate::cpu::{PC_REGISTER, PSRRegister, LR_REGISTER, OperatingMode};

use super::{CPU, MemoryAccess};

impl CPU {
  pub fn populate_arm_lut(&mut self) {
    for i in 0..4096 {
      let instr_fn = self.decode_arm((i & 0xff0) >> 4, i & 0xf);
      self.arm_lut.push(instr_fn);
    }
  }

  fn decode_arm(&mut self, upper: u16, lower: u16) -> fn(&mut CPU, instr: u32) -> Option<MemoryAccess> {
    if upper & 0b11111100 == 0 && lower == 0b1001 {
      CPU::multiply
    } else if upper & 0b11111000 == 0b00001000 && lower == 0b1001 {
      CPU::multiply_long
    } else if upper & 0b11110011 == 0b00010000 && lower == 0b1001 {
      CPU::single_data_swap
    } else if upper == 0b00010010 && lower == 1 {
      CPU::branch_and_exchange
    } else if upper & 0b11100100 == 0 && lower & 0b1001 == 0b1001 {
      CPU::halfword_data_transfer_register
    } else if upper & 0b11100100 == 0b00000100 && lower & 0b1001 == 0b1001 {
      CPU::halfword_data_transfer_immediate
    } else if upper & 0b11000000 == 0 {
      // check for psr transfer instructions as they are a subset of data processing
      let s = upper & 0b1;
      let op_code = (upper >> 1) & 0xf;

      let is_updating_flags_only = (op_code & 0b1100) == 0b1000;

      if s == 0 && is_updating_flags_only {
        if op_code & 0b1 == 0 {
          CPU::transfer_status_to_register
        } else {
          CPU::transfer_register_to_status
        }
      } else {
        CPU::data_processing
      }

    } else if upper & 0b11100000 == 0b01100000 && lower & 0b1 == 1 {
      // undefined instruction
      CPU::arm_panic
    } else if upper & 0b11000000 == 0b01000000 {
      CPU::single_data_transfer
    } else if upper & 0b11100000 == 0b10000000 {
      CPU::block_data_transfer
    } else if upper & 0b11100000 == 0b10100000 {
      CPU::branch
    } else if upper & 0b11110000 == 0b11110000 {
      CPU::arm_software_interrupt
    }  else {
      CPU::arm_panic
    }
  }

  fn arm_panic(&mut self, instr: u32) -> Option<MemoryAccess> {
    panic!("unsupported instr: {:032b}", instr)
  }

  fn data_processing(&mut self, instr: u32) -> Option<MemoryAccess> {
    // println!("inside data processing");

    let mut return_val = Some(MemoryAccess::Sequential);

    let i = (instr >> 25) & 0b1;
    let op_code = (instr >> 21) & 0xf;
    let mut s = (instr >> 20) & 0b1;
    let rn = (instr >> 16) & 0xf;
    let rd = (instr >> 12) & 0xf;

    let mut operand1 = self.get_register(rn as usize);

    let mut carry = self.cpsr.contains(PSRRegister::CARRY);
    let mut overflow = self.cpsr.contains(PSRRegister::OVERFLOW);

    let operand2 = if i == 1 {
      let immediate = instr & 0xff;
      let rotate = (2 * ((instr >> 8) & 0xf)) as u8;

      self.ror(immediate, rotate, false, true, &mut carry)
    } else {
      // println!("using register for 2nd operand");
      self.get_data_processing_register_operand(instr, rn, &mut operand1, &mut carry)
    };

    if rd == PC_REGISTER as u32 && s == 1 {
      self.transfer_spsr_mode();
      s = 0;
    }

    // println!("operand1 = {operand1} operand2 = {operand2}");
    // println!("rn = {rn} rd = {rd}");
    // println!("{} r{rd}, {operand2}", self.get_op_name(op_code as u8));

    // finally do the operation on the two operands and store in rd
    let (result, should_update) = self.execute_alu_op(op_code, operand1, operand2, &mut carry, &mut overflow);

    if s == 1 {
      self.update_flags(result, overflow, carry);
    }

    if should_update {
      if rd == PC_REGISTER as u32 {
        if self.cpsr.contains(PSRRegister::STATE_BIT) {
          self.pc = result & !(0b1);
          // println!("switched to thumb");
          self.reload_pipeline16();
        } else {
          self.pc = result & !(0b11);
          // println!("switched to arm");
          self.reload_pipeline32();
        }

        return_val = None;
      } else {
        self.r[rd as usize] = result;
      }
    }

    if !should_update || rd != PC_REGISTER as u32 {
      self.pc = self.pc.wrapping_add(4);
    }

    return_val
  }

  fn multiply(&mut self, instr: u32) -> Option<MemoryAccess> {
    // println!("inside multiply");

    let a = (instr >> 21) & 0b1;
    let s = (instr >> 20) & 0b1;
    let rd = (instr >> 16) & 0xf;
    let rn = (instr >> 12) & 0xf;
    let rs = (instr >> 8) & 0xf;
    let rm = instr & 0xf;

    let operand1 = self.get_register(rm as usize);
    let operand2 = self.get_register(rs as usize);
    let operand3 = self.get_register(rn as usize);

    let result = if a == 0 {
      operand1.wrapping_mul(operand2)
    } else {
      self.add_cycles(1);
      operand1.wrapping_mul(operand2).wrapping_add(operand3)
    };

    let cycles = self.get_multiplier_cycles(operand2);

    self.add_cycles(cycles);

    if s == 1 {
      // update flags
      self.update_flags(result, false, false);
    }

    self.r[rd as usize] = result;

    self.pc = self.pc.wrapping_add(4);

    Some(MemoryAccess::Sequential)
  }

  fn multiply_long(&mut self, instr: u32) -> Option<MemoryAccess> {
    // println!("inside multiply long");

    let u = (instr >> 22) & 0b1;
    let a = (instr >> 21) & 0b1;
    let s = (instr >> 20) & 0b1;

    let rd_hi = (instr >> 16) & 0xf;
    let rd_low = (instr >> 12) & 0xf;
    let rs = (instr >> 8) & 0xf;
    let rm = instr & 0xf;

    let operand1 = self.get_register(rm as usize);
    let operand2 = self.get_register(rs as usize);

    let mut result = if u == 0 {
      // unsigned
      (operand1 as u64).wrapping_mul(operand2 as u64)
    } else {
      // signed
      (operand1 as i32 as i64).wrapping_mul(operand2 as i32 as i64) as u64
    };

    if a == 1 {
      let accumulate = (self.r[rd_hi as usize] as u64) << 32 | (self.r[rd_low as usize] as u64);

      result = result.wrapping_add(accumulate);

      self.add_cycles(1);
    }

    // store the results in rd high and rd low
    self.r[rd_low as usize] = (result & 0xffffffff) as i32 as u32;
    self.r[rd_hi as usize] = (result >> 32) as i32 as u32;

    let cycles = self.get_multiplier_cycles(operand2);

    self.add_cycles(cycles);

    if s == 1 {
      self.cpsr.set(PSRRegister::NEGATIVE, result >> 63 & 0b1 == 1);
      self.cpsr.set(PSRRegister::ZERO, result == 0);
      self.cpsr.set(PSRRegister::CARRY, false);
      self.cpsr.set(PSRRegister::OVERFLOW, false);
    }

    self.pc = self.pc.wrapping_add(4);
    Some(MemoryAccess::Sequential)
  }

  fn single_data_swap(&mut self, instr: u32) -> Option<MemoryAccess> {
    // println!("inside single data swap");

    let b = (instr >> 22) & 0b1;
    let rn = (instr >> 16) & 0xf;
    let rd = (instr >> 12) & 0xf;
    let rm = instr & 0xf;

    let base_address = self.get_register(rn as usize);

    if b == 1 {
      let temp = self.load_8(base_address, MemoryAccess::NonSequential);
      self.store_8(base_address, self.get_register(rm as usize) as u8, MemoryAccess::Sequential);
      self.r[rd as usize] = temp as u32;
    } else {
      let temp = self.ldr_word(base_address);
      self.store_32(base_address & !(0b11), self.get_register(rm as usize), MemoryAccess::Sequential);
      self.r[rd as usize] = temp;
    }

    self.pc = self.pc.wrapping_add(4);
    Some(MemoryAccess::NonSequential)
  }

  fn branch_and_exchange(&mut self, instr: u32) -> Option<MemoryAccess> {
    // println!("inside branch and exchange");

    let rn = instr & 0xf;

    // println!("reading register {rn} with address {:X}", self.r[rn as usize]);

    if rn == PC_REGISTER as u32 {
      panic!("using pc register for branch and exchange");
    }

    let address = self.r[rn as usize];

    if address & 0b1 == 0 {
      // stay in arm mode
      self.pc = address & !(0b11);

      self.cpsr.remove(PSRRegister::STATE_BIT);

      // reload the pipeline
      self.reload_pipeline32();
    } else {
      // enter thumb state
      self.pc = address & !(0b1);
      self.cpsr.insert(PSRRegister::STATE_BIT);

      // reload the pipeline
      self.reload_pipeline16();
    }
    None
  }

  fn halfword_data_transfer_register(&mut self, instr: u32) -> Option<MemoryAccess>  {
    // println!("inside halfword data transfer register");
    let rm = instr & 0xf;

    let offset = self.get_register(rm as usize);

    self.halfword_transfer(offset, instr)
  }

  fn halfword_data_transfer_immediate(&mut self, instr: u32) -> Option<MemoryAccess>  {
    // println!("inside halfword data transfer immediate");

    let offset_high = (instr >> 8) & 0xf;
    let offset_low = instr & 0xf;

    let offset = offset_high << 4 | offset_low;

    // println!("offset = {offset}");

    self.halfword_transfer(offset, instr)
  }

  fn halfword_transfer(&mut self, offset: u32, instr: u32) -> Option<MemoryAccess> {
    let sh = (instr >> 5) & 0b11;
    let rd = (instr >> 12) & 0xf;
    let rn = (instr >> 16) & 0xf;

    let l = (instr >> 20) & 0b1;
    let w = (instr >> 21) & 0b1;
    let u = (instr >> 23) & 0b1;
    let p = (instr >> 24) & 0b1;

    // println!("using register r{rn} for the base address");

    let mut address = self.get_register(rn as usize);

    // println!("base = {:X}", address);

    let offset = if u == 0 {
      -(offset as i32) as u32
    } else {
      offset
    };

    let mut should_update_pc = true;

    let effective_address = (address as i32).wrapping_add(offset as i32) as u32;

    if p == 1 {
      address = effective_address;
    }

    let mut result = Some(MemoryAccess::NonSequential);

    if l == 0 {
      // store
      let value = if rd == PC_REGISTER as u32 {
        self.pc + 4
      } else {
        self.r[rd as usize]
      };

      if sh == 1 {
        self.store_16(address & !(0b1), value as u16, MemoryAccess::NonSequential);
      } else {
        panic!("invalid option for storing half words");
      }
    } else {
      // load
      let value = match sh {
        1 => self.ldr_halfword(address) as u32, // unsigned halfwords
        2 => self.load_8(address, MemoryAccess::NonSequential) as i8 as i32 as u32, // signed byte
        3 => self.ldr_signed_halfword(address) as i32 as u32, // signed halfwords,
        _ => panic!("shouldn't happen")
      };

      if rd == PC_REGISTER as u32 {
        self.pc = value & !(0b11);

        self.reload_pipeline32();

        should_update_pc = false;

        result = None;
      } else {
        self.r[rd as usize] = value;
      }

      // println!("loaded value {value} from address {:X}", address);

      self.add_cycles(1);
    }

    if (l == 0 || rd != rn) && (w == 1 || p == 0) {
      self.r[rn as usize] = effective_address;
    }

    if should_update_pc {
      self.pc = self.pc.wrapping_add(4);
    }

    result
  }

  fn single_data_transfer(&mut self, instr: u32) -> Option<MemoryAccess>  {
    // println!("inside single data transfer");

    let mut result = Some(MemoryAccess::NonSequential);

    let i = (instr >> 25) & 0b1;
    let p = (instr >> 24) & 0b1;
    let u = (instr >> 23) & 0b1;
    let b = (instr >> 22) & 0b1;
    let w = (instr >> 21) & 0b1;
    let l = (instr >> 20) & 0b1;

    let rn = (instr >> 16) & 0xf;
    let rd = (instr >> 12) & 0xf;
    let mut offset: u32 = instr & 0xfff;

    let mut should_update_pc = true;

    // println!("getting address from register {rn}");

    let mut address = self.get_register(rn as usize);

    if i == 1 {
      // println!("offset is a register shifted in some way");
      // offset is a register shifted in some way
      self.update_single_data_transfer_offset(instr, &mut offset);
    }

    if u == 0 {
      offset = -(offset as i32) as u32;
    }

    let effective_address = (address as i32).wrapping_add(offset as i32) as u32;

    // println!("offset = {:X} address = {:X} effective = {:X}", offset, address, effective_address);

    let old_mode = self.cpsr.mode();

    if p == 0 && w == 1 {
      // println!("changing mode to user mode in single data transfer");
      self.set_mode(OperatingMode::User);
    }

    if p == 1 {
      address = effective_address;
    }

    if l == 1 {
      // load
      let data = if b == 1 {
        self.load_8(address, MemoryAccess::NonSequential) as u32
      } else {
        self.ldr_word(address)
      };

      // println!("setting register {rd} to {data} from address {:X}", address);

      if rd == PC_REGISTER as u32 {
        self.pc = data & !(0b11);

        result = None;

        should_update_pc = false;

        self.reload_pipeline32();
      } else {
        self.r[rd as usize] = data;
      }

      self.add_cycles(1);
    } else {
      // store
      let value = if rd == PC_REGISTER as u32 {
        self.pc + 4
      } else {
        self.r[rd as usize]
      };

      // println!("(rd = {rd}) storing {value} at {:X}", address);

      if b == 1 {
        self.store_8(address, value as u8, MemoryAccess::NonSequential);
      } else {
        self.store_32(address & !(0b11), value, MemoryAccess::NonSequential);
      }
    }

    if (l == 0 || rn != rd) && (p == 0 || w == 1) {
      if rn == PC_REGISTER as u32 {
        panic!("shouldn't happen");
      } else {
        self.r[rn as usize] = effective_address;
      }
    }

    if p == 0 && w == 1 {
      self.set_mode(old_mode);
    }

    if should_update_pc {
      self.pc = self.pc.wrapping_add(4);
    }

    result
  }

  fn block_data_transfer(&mut self, instr: u32) -> Option<MemoryAccess>  {
    // println!("inside block data transfer");

    let mut result = Some(MemoryAccess::NonSequential);

    let mut p = (instr >> 24) & 0b1;
    let u = (instr >> 23) & 0b1;
    let s = (instr >> 22) & 0b1;
    let mut w = (instr >> 21) & 0b1;
    let l = (instr >> 20) & 0b1;

    let rn = (instr >> 16) & 0xf;

    // println!("rn = r{rn} = {:X}", self.r[rn as usize]);

    let register_list = instr & 0xffff;

    let mut should_increment_pc = true;

    if s == 1 && (matches!(self.cpsr.mode(), OperatingMode::User) || matches!(self.cpsr.mode(), OperatingMode::System)) {
      panic!("s bit set in unprivileged mode");
    }

    let user_banks_transferred = if s == 1 {
      if l == 1 {
        (register_list << 15) & 0b1 == 0
      } else {
        true
      }
    } else {
      false
    };

    let old_mode = self.cpsr.mode();

    if user_banks_transferred {
      self.set_mode(OperatingMode::User);
    }

    let psr_transfer = s == 1 && l == 1 && (register_list << 15) & 0b1 == 1;

    let num_registers = register_list.count_ones();

    let mut address = self.r[rn as usize];

    let old_base = address;

    if register_list != 0 && u == 0 {
      address = address.wrapping_sub(num_registers * 4);

      if w == 1 {
        self.r[rn as usize] = address;
        w = 0;
      }
      if p == 0 {
        p = 1;
      } else {
        p = 0;
      }
    }

    let mut access = MemoryAccess::Sequential;

    if register_list != 0 {
      if l == 0 {
        // store
        let mut is_first_register = true;
        for i in 0..16 {
          if (register_list >> i) & 0b1 == 1 {
            let value = if i != rn {
              if i == PC_REGISTER as u32 {
                // pc - 8 + 12 = + 4
                self.pc + 4
              } else {
                // println!("pushing from register {i}");
                self.r[i as usize]
              }
            } else if is_first_register {
              // println!("using old base");
              old_base
            } else {
              // println!("using old base +- offset");
              let offset = num_registers * 4;

              if u == 1 {
                old_base + offset
              } else {
                old_base - offset
              }
            };

            is_first_register = false;

            if p == 1 {
              address += 4;
            }

            self.store_32(address & !(0b11), value, access);

            access = MemoryAccess::Sequential;


            if p == 0 {
              address += 4;
            }
          }
        }
      } else {
        // load
        for i in 0..16 {
          if (register_list >> i) & 0b1 == 1 {
            if i == rn {
              w = 0;
            }

            if p == 1 {
              address += 4;
            }

            let value = self.load_32(address & !(0b11), access);

            access = MemoryAccess::Sequential;

            // println!("popping {:X} from {:X} to register {i}", value, address);

            if i == PC_REGISTER as u32 {
              self.pc = value & !(0b11);

              if psr_transfer {
                self.transfer_spsr_mode();
              }

              should_increment_pc = false;
              self.reload_pipeline32();

              result = None;

            } else {
              self.r[i as usize] = value;
            }

            if p == 0 {
              address += 4;
            }
          }
        }

        self.add_cycles(1);
      }
    } else {
      // empty rlist edge case
      if l == 0 {
        let address = match (u, p) {
          (0, 0) => address.wrapping_sub(0x3c),
          (0, 1) => address.wrapping_sub(0x40),
          (1, 0) => address,
          (1, 1) => address.wrapping_add(4),
          _ => unreachable!("shouldn't happen")
        };
        self.store_32(address & !(0b11), self.pc + 4, MemoryAccess::NonSequential);

        // println!("stored pc value {:X} at address {:X}", self.pc + 4, address);
      } else {
        let val = self.ldr_word(address);
        self.pc = val & !(0b11);
        self.reload_pipeline32();

        result = None;
      }

      address = if u == 1 {
        address.wrapping_add(0x40)
      } else {
        address.wrapping_sub(0x40)
      };
    }

    if user_banks_transferred {
      self.set_mode(old_mode);
    }

    if w == 1 {
      self.r[rn as usize] = address;
    }

    if should_increment_pc {
      self.pc = self.pc.wrapping_add(4);
    }

    result
  }

  fn branch(&mut self, instr: u32) -> Option<MemoryAccess> {
    // println!("inside branch");
    let l = (instr >> 24) & 0b1;
    let offset = (((instr & 0xFFFFFF) << 8) as i32) >> 6;

    if l == 1 {
      // pc current instruction address is self.pc - 8, plus the word size of 4 bytes = self.pc - 4
      self.r[LR_REGISTER] = (self.pc - 4) & !(0b1);
    }

    self.pc = ((self.pc as i32).wrapping_add(offset) as u32) & !(0b1);

    self.reload_pipeline32();

    None
  }

  fn arm_software_interrupt(&mut self, _instr: u32) -> Option<MemoryAccess>  {
    // println!("inside arm software interrupt");

    self.software_interrupt();

    None
  }


  fn transfer_status_to_register(&mut self, instr: u32) -> Option<MemoryAccess> {
    // println!("inside psr transfer to register (mrs)");
    let p = (instr >> 22) & 0b1;

    let value = if p == 0 {
      self.cpsr.bits()
    } else {
      self.spsr.bits()
    };

    let rd = (instr >> 12) & 0xf;

    if rd == PC_REGISTER as u32 {
      self.pc = value & !(0b11);
    } else {
      self.r[rd as usize] = value;
    }

    self.pc = self.pc.wrapping_add(4);

    Some(MemoryAccess::Sequential)
  }

  fn transfer_register_to_status(&mut self, instr: u32) -> Option<MemoryAccess> {
    // println!("inside PSR transfer from register (msr)");
    let i = (instr >> 25) & 0b1;
    let p = (instr >> 22) & 0b1;

    let f = (instr >> 19) & 0b1;
    let s = (instr >> 18) & 0b1;
    let x = (instr >> 17) & 0b1;
    let c = (instr >> 16) & 0b1;

    let mut mask = 0;

    let value = if i == 1 {
      let immediate = instr & 0xff;
      let rotate = ((instr >> 8) & 0xf) * 2;

      let mut carry = self.cpsr.contains(PSRRegister::CARRY);

      let value = self.ror(immediate, rotate as u8, false, true, &mut carry);

      self.cpsr.set(PSRRegister::CARRY, carry);

      value
    } else {
      let rm = instr & 0xf;

      // println!("using register r{rm}");

      self.r[rm as usize]
    };

    if f == 1 {
      mask |= 0xff << 24;
    }
    if s == 1 {
      mask |= 0xff << 16;
    }
    if x == 1 {
      mask |= 0xff << 8;
    }
    if c == 1 {
      mask |= 0xff;
    }

    if matches!(self.cpsr.mode(), OperatingMode::User) {
      if p == 1 {
        panic!("SPSR not accessible in user mode");
      }
      let new_cpsr = self.cpsr.bits() & !(0xf000_0000) | (value & 0xf000_0000);

      self.cpsr = PSRRegister::from_bits_retain(new_cpsr);
    } else {
      if p == 1 {
        self.spsr = PSRRegister::from_bits_retain(value);
      } else {
        let new_psr = PSRRegister::from_bits_retain((self.cpsr.bits() & !mask) | (value & mask));

        if self.cpsr.mode() as u8 != new_psr.mode() as u8 {
          self.set_mode(new_psr.mode());
        }

        self.cpsr = new_psr;
      }
    }

    self.pc = self.pc.wrapping_add(4);

    Some(MemoryAccess::Sequential)
  }

  fn update_flags(&mut self, result: u32, overflow: bool, carry: bool) {
    self.cpsr.set(PSRRegister::CARRY, carry);
    self.cpsr.set(PSRRegister::OVERFLOW, overflow);
    self.cpsr.set(PSRRegister::ZERO, result == 0);
    self.cpsr.set(PSRRegister::NEGATIVE, (result as i32) < 0);

    // println!("updating carry to {}, overflow to {}, zero to {}, negative to {}", self.cpsr.contains(PSRRegister::CARRY), self.cpsr.contains(PSRRegister::OVERFLOW), self.cpsr.contains(PSRRegister::ZERO), self.cpsr.contains(PSRRegister::NEGATIVE));
  }

  fn subtract_arm(&mut self, operand1: u32, operand2: u32, carry: &mut bool, overflow: &mut bool) -> u32 {
    let result = operand1.wrapping_sub(operand2);

    *carry = operand1 >= operand2;

    let (_, overflow_result) = (operand1 as i32).overflowing_sub(operand2 as i32);

    *overflow = overflow_result;

    result
  }

  pub fn subtract_carry_arm(&mut self, operand1: u32, operand2: u32, carry: &mut bool, overflow: &mut bool) -> u32 {
    self.add_carry_arm(operand1, !operand2, carry, overflow)
  }

  fn add_arm(&mut self, operand1: u32, operand2: u32, carry: &mut bool, overflow: &mut bool) -> u32 {
    let (result, carry_result) = operand1.overflowing_add(operand2);

    *carry = carry_result;

    let (_, overflow_result) = (operand1 as i32).overflowing_add(operand2 as i32);

    *overflow = overflow_result;

    result
  }

  fn add_carry_arm(&mut self, operand1: u32, operand2: u32, carry: &mut bool, overflow: &mut bool) -> u32 {
    let (result1, carry_result1) = operand1.overflowing_add(operand2);
    let (result2, carry_result2) = result1.overflowing_add(if self.cpsr.contains(PSRRegister::CARRY) { 1 } else { 0 });

    *carry = carry_result1 || carry_result2;

    *overflow = (!(operand1 ^ operand2) & (operand2 ^ (result2))) >> 31 == 1;

    result2
  }

  fn transfer_spsr_mode(&mut self) {
    let spsr = self.spsr;

    if spsr.mode() as u8 != self.cpsr.mode() as u8 {
      self.set_mode(spsr.mode());
    }

    self.cpsr = spsr;
  }

  fn get_op_name(&self, op_code: u8) -> &'static str {
    match op_code {
      0 => "AND",
      1 => "EOR",
      2 => "SUB",
      3 => "RSB",
      4 => "ADD",
      5 => "ADC",
      6 => "SBC",
      7 => "RSC",
      8 => "TST",
      9 => "TEQ",
      10 => "CMP",
      11 => "CMN",
      12 => "ORR",
      13 => "MOV",
      14 => "BIC",
      15 => "MVN",
      _ => unreachable!("can't happen")
    }
  }

  fn execute_alu_op(&mut self, op_code: u32, operand1: u32, operand2: u32, carry: &mut bool, overflow: &mut bool) -> (u32, bool) {
    match op_code {
      0 => (operand1 & operand2, true),
      1 => (operand1 ^ operand2, true),
      2 => (self.subtract_arm(operand1, operand2, carry, overflow), true),
      3 => (self.subtract_arm(operand2,operand1, carry, overflow), true),
      4 => (self.add_arm(operand1, operand2, carry, overflow), true),
      5 => (self.add_carry_arm(operand1, operand2, carry, overflow), true),
      6 => (self.subtract_carry_arm(operand1, operand2, carry, overflow), true),
      7 => (self.subtract_carry_arm(operand2, operand1, carry, overflow), true),
      8 => (operand1 & operand2, false),
      9 => (operand1 ^ operand2, false),
      10 => (self.subtract_arm(operand1, operand2, carry, overflow), false),
      11 => (self.add_arm(operand1, operand2, carry, overflow), false),
      12 => (operand1 | operand2, true),
      13 => (operand2, true),
      14 => (operand1 & !operand2, true),
      15 => (!operand2, true),
      _ => unreachable!("not possible")
    }
  }

  fn get_data_processing_register_operand(&mut self, instr: u32, rn: u32, operand1: &mut u32, carry: &mut bool) -> u32 {
    let shift_by_register = (instr >> 4) & 0b1 == 1;

    let mut immediate = true;

    let shift = if shift_by_register {
      immediate = false;

      if rn == PC_REGISTER as u32 {
        *operand1 += 4;
      }
      self.add_cycles(1);

      let rs = (instr >> 8) & 0xf;

      // println!("rs = {rs}");

      self.r[rs as usize] & 0xff
    } else {
      (instr >> 7) & 0x1f
    };

    let shift_type = (instr >> 5) & 0b11;

    let rm = instr & 0xf;

    // println!("rm = {rm}");

    let mut shifted_operand = self.get_register(rm as usize);

    if shift_by_register && rm == PC_REGISTER as u32 {
      shifted_operand += 4;
    }

    // println!("shifted_operand = {shifted_operand} shift is {shift} shift type is {shift_type}");

    match shift_type {
      0 => self.lsl(shifted_operand, shift, carry),
      1 => self.lsr(shifted_operand, shift, immediate, carry),
      2 => self.asr(shifted_operand, shift, immediate, carry),
      3 => self.ror(shifted_operand, shift as u8, immediate, true, carry),
      _ => unreachable!("can't happen")
    }
  }

  fn update_single_data_transfer_offset(&mut self, instr: u32, offset: &mut u32) {
    // offset is a register shifted in some way
    let shift_type = (instr >> 5) & 0b11;

    let rm = *offset & 0xf;

    let shifted_operand = if rm == PC_REGISTER as u32 {
      self.pc + 4
    } else {
      // println!("using r{rm} = {:X}", self.r[rm as usize]);
      self.r[rm as usize]
    };

    let shift_by_register = (instr >> 4) & 0b1;

    let mut immediate = true;

    let shift = if shift_by_register == 1 {
      immediate = false;
      let rs = *offset >> 8;

      if rs == PC_REGISTER as u32 {
        self.pc & 0xff
      } else {
        self.r[rs as usize] & 0xff
      }
    } else {
      *offset >> 7
    };

    let mut carry = self.cpsr.contains(PSRRegister::CARRY);

    *offset = match shift_type {
      0 => self.lsl(shifted_operand, shift, &mut carry),
      1 => self.lsr(shifted_operand, shift, immediate, &mut carry),
      2 => self.asr(shifted_operand, shift, immediate, &mut carry),
      3 => self.ror(shifted_operand, shift as u8, immediate, true, &mut carry),
      _ => unreachable!("can't happen")
    };
  }
}