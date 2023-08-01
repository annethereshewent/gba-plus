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
      CPU::data_processing
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
    panic!("unsupported instr: {:b}", instr)
  }

  fn data_processing(&mut self, instr: u32) -> Option<MemoryAccess> {
    println!("inside data processing");

    let mut return_val = Some(MemoryAccess::Sequential);

    let i = (instr >> 25) & 0b1;
    let op_code = (instr >> 21) & 0b1111;
    let mut s = (instr >> 20) & 0b1;
    let rn = (instr >> 16) & 0b1111;
    let rd = (instr >> 12) & 0b1111;

    println!("operand1 coming from register {rn}");

    let mut operand1 = if rn == PC_REGISTER as u32 {
      self.pc
    } else {
      self.r[rn as usize]
    };

    let mut carry = self.cpsr.contains(PSRRegister::CARRY);
    let mut overflow = self.cpsr.contains(PSRRegister::OVERFLOW);

    let operand2 = if i == 1 {
      let immediate = instr & 0xff;
      let rotate = (2 * ((instr >> 8) & 0b1111)) as u8;

      self.ror_arm(immediate, rotate, &mut carry)
    } else {
      let shift_by_register = (instr >> 4) & 0b1 == 1;

      let shift = if shift_by_register {
        if rn == PC_REGISTER as u32 {
          operand1 += 4;
        }
        let rs = (instr >> 8) & 0b1111;

        self.r[rs as usize]
      } else {
        (instr >> 7) & 0b11111
      };

      let shift_type = (instr >> 5) & 0b11;

      let rm = instr & 0b1111;

      let shifted_operand = if rm == PC_REGISTER as u32 {
        self.pc
      } else {
        self.r[rm as usize]
      };

      match shift_type {
        0 => shifted_operand << shift,
        1 => shifted_operand >> shift,
        2 => ((shifted_operand as i32) >> shift) as u32,
        3 => shifted_operand.rotate_right(shift),
        _ => unreachable!("can't happen")
      }
    };

    if rd == PC_REGISTER as u32 && s == 1 {
      self.transfer_spsr_mode();
      s = 0;
    }

    println!("rd is {rd}, operand 1 is {operand1}, operand2 is {operand2} and op code is {op_code}");

    // finally do the operation on the two operands and store in rd
    let (result, should_update) = match op_code {
      0 => (operand1 & operand2, true),
      1 => (operand1 ^ operand2, true),
      2 => (self.subtract_arm(operand1, operand2, &mut carry, &mut overflow), true),
      3 => (self.subtract_arm(operand2,operand1, &mut carry, &mut overflow), true),
      4 => (self.add_arm(operand1, operand2, &mut carry, &mut overflow), true),
      5 => (self.add_carry_arm(operand1, operand2, &mut carry, &mut overflow), true),
      6 => (self.subtract_carry_arm(operand1, operand2, &mut carry, &mut overflow), true),
      7 => (self.subtract_carry_arm(operand2, operand1, &mut carry, &mut overflow), true),
      8 => (operand1 & operand2, false),
      9 => (operand1 ^ operand2, false),
      10 => (self.subtract_arm(operand1, operand2, &mut carry, &mut overflow), false),
      11 => (self.add_arm(operand1, operand2, &mut carry, &mut overflow), false),
      12 => (operand1 | operand2, true),
      13 => (operand2, true),
      14 => (operand1 & !operand2, true),
      15 => (!operand2, true),
      _ => unreachable!("not possible")
    };

    if s == 1 {
      println!("compared {operand1} with {operand2}, op code is {op_code}");
      println!("updating flags for result {result}");
      self.update_flags(result, overflow, carry);
    }

    if should_update {
      if rd == PC_REGISTER as u32 {
        self.pc = result & !(0b11);

        if self.cpsr.contains(PSRRegister::STATE_BIT) {
          println!("switched to arm");
          self.reload_pipeline16();
        } else {
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
    println!("inside multiply");
    self.pc = self.pc.wrapping_add(4);
    Some(MemoryAccess::Sequential)
  }

  fn multiply_long(&mut self, instr: u32) -> Option<MemoryAccess> {
    println!("inside multiply long");

    self.pc = self.pc.wrapping_add(4);
    Some(MemoryAccess::Sequential)
  }

  fn single_data_swap(&mut self, instr: u32) -> Option<MemoryAccess> {
    println!("inside single data swap");

    self.pc = self.pc.wrapping_add(4);
    Some(MemoryAccess::Sequential)
  }

  fn branch_and_exchange(&mut self, instr: u32) -> Option<MemoryAccess> {
    println!("inside branch and exchange");

    let rn = instr & 0b1111;

    println!("reading register {rn}");

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
    println!("inside halfword data transfer register");

    self.pc = self.pc.wrapping_add(4);
    Some(MemoryAccess::Sequential)
  }

  fn halfword_data_transfer_immediate(&mut self, instr: u32) -> Option<MemoryAccess>  {
    println!("inside halfword data transfer immediate");

    self.pc = self.pc.wrapping_add(4);
    Some(MemoryAccess::Sequential)
  }

  fn single_data_transfer(&mut self, instr: u32) -> Option<MemoryAccess>  {
    println!("inside single data transfer");

    let mut result = Some(MemoryAccess::NonSequential);

    let i = (instr >> 25) & 0b1;
    let p = (instr >> 24) & 0b1;
    let u = (instr >> 23) & 0b1;
    let b = (instr >> 22) & 0b1;
    let w = (instr >> 21) & 0b1;
    let l = (instr >> 20) & 0b1;

    let rn = (instr >> 16) & 0b1111;
    let rd = (instr >> 12) & 0b1111;
    let mut offset: u32 = instr & 0xfff;

    let mut should_update_pc = true;

    let mut address = if rn == PC_REGISTER as u32 {
      self.pc
    } else {
      self.r[rn as usize]
    };

    if i == 1 {
      // offset is a register shifted in some way
      let shift_type = (instr >> 5) & 0b11;

      let rm = offset & 0xf;

      let shifted_operand = if rm == PC_REGISTER as u32 {
        self.pc + 4
      } else {
        self.r[rm as usize]
      };

      let shift_by_register = (instr >> 4) & 0b1;

      let shift = if shift_by_register == 1 {
        let rs = offset >> 8;

        if rs == PC_REGISTER as u32 {
          self.pc & 0xff
        } else {
          self.r[rs as usize] & 0xff
        }
      } else {
        offset >> 7
      };

      offset = match shift_type {
        0 => shifted_operand << shift,
        1 => shifted_operand >> shift,
        2 => ((shifted_operand as i32) >> shift) as u32,
        3 => shifted_operand.rotate_right(shift),
        _ => unreachable!("can't happen")
      };
    }

    if u == 0 {
      offset = -(offset as i32) as u32;
    }

    let effective_address = (address as i32).wrapping_add(offset as i32) as u32;

    let old_mode = self.cpsr.mode();

    if p == 0 && w == 1 {
      self.set_mode(OperatingMode::User);
    }

    if p == 1 {
      address = effective_address;
    }

    if l == 1 {
      // load
      let data = if b == 1 {
        self.mem_read_8(address) as u32
      } else {
        self.mem_read_32(address)
      };

      println!("setting register {rd} to {data} from address {:X}", address);

      if rd == PC_REGISTER as u32 {
        self.pc = data & !(0b11);

        result = None;

        should_update_pc = false;

        self.reload_pipeline32();
      } else {
        self.r[rd as usize] = data;
      }
    } else {
      // store
      let value = if rd == PC_REGISTER as u32 {
        self.pc + 4
      } else {
        self.r[rd as usize]
      };

      println!("storing {value} at {:X}", address);

      if b == 1 {
        self.mem_write_8(address, value as u8);
      } else {
        self.mem_write_32(address & !(0b11), value);
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
    println!("inside block data transfer");

    let mut result = Some(MemoryAccess::Sequential);

    let mut p = (instr >> 24) & 0b1;
    let u = (instr >> 23) & 0b1;
    let s = (instr >> 22) & 0b1;
    let mut w = (instr >> 21) & 0b1;
    let l = (instr >> 20) & 0b1;

    let rn = (instr >> 16) & 0b1111;

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
              self.r[i as usize]
            }
          } else if is_first_register {
            old_base
          } else {
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

          println!("(p = {}), writing {:X} to {:X}", p, value, address & !(0b11));

          self.mem_write_32(address & !(0b11), value);


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

          let value = self.mem_read_32(address & !(0b11));

          if i == 14 {
            println!("reading {:X} from {:X}", value, address & !(0b11));
          }

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
    println!("inside branch");
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

  fn arm_software_interrupt(&mut self, instr: u32) -> Option<MemoryAccess>  {
    println!("inside arm software interrupt");

    self.interrupt();

    None
  }

  fn ror_arm(&mut self, immediate: u32, amount: u8, carry: &mut bool) -> u32 {
    let amount = amount % 32;

    let result = immediate.rotate_right(amount as u32);

    *carry = (result >> 31) & 0b1 == 1;

    result
  }

  fn update_flags(&mut self, result: u32, overflow: bool, carry: bool) {
    self.cpsr.set(PSRRegister::CARRY, carry);
    self.cpsr.set(PSRRegister::OVERFLOW, overflow);
    self.cpsr.set(PSRRegister::ZERO, result == 0);
    self.cpsr.set(PSRRegister::NEGATIVE, (result as i32) < 0);

    println!("updating carry to {}, overflow to {}, zero to {}, negative to {}", self.cpsr.contains(PSRRegister::CARRY), self.cpsr.contains(PSRRegister::OVERFLOW), self.cpsr.contains(PSRRegister::ZERO), self.cpsr.contains(PSRRegister::NEGATIVE));
  }

  fn subtract_arm(&mut self, operand1: u32, operand2: u32, carry: &mut bool, overflow: &mut bool) -> u32 {
    let (result, carry_result) = operand1.overflowing_sub(operand2);

    *carry = carry_result;


    let (_, overflow_result) = (operand1 as i32).overflowing_sub(operand2 as i32);

    *overflow = overflow_result;

    result
  }

  fn subtract_carry_arm(&mut self, operand1: u32, operand2: u32, carry: &mut bool, overflow: &mut bool) -> u32 {
    let (result1, carry_result1) = operand1.overflowing_sub(operand2);
    let (result2, carry_result2) = result1.overflowing_sub(if *carry { 0 } else { 1 });

    *carry = carry_result1 || carry_result2;

    let (overflow_add1, overflow_result1) = (operand1 as i32).overflowing_sub(operand2 as i32);
    let (_, overflow_result2) = (overflow_add1 as i32).overflowing_sub(if *carry { 0 } else { 1 });

    *overflow = overflow_result1 || overflow_result2;

    result2
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
    let (result2, carry_result2) = result1.overflowing_add(if *carry { 1 } else { 0 });

    *carry = carry_result1 || carry_result2;

    let (overflow_add1, overflow_result1) = (operand1 as i32).overflowing_add(operand2 as i32);
    let (_, overflow_result2) = (overflow_add1 as i32).overflowing_add(if *carry { 1 } else { 0 });

    *overflow = overflow_result1 || overflow_result2;

    result2
  }

  fn transfer_spsr_mode(&mut self) {

    if self.spsr.mode() as u8 != self.cpsr.mode() as u8 {
      self.set_mode(self.spsr.mode());
    }

    self.cpsr = self.spsr;
  }
}