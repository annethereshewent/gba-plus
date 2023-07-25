use super::{CPU, PSRRegister};

impl CPU {
  pub fn decode_instruction(&mut self, format: u16) -> fn(&mut CPU, instruction: u16) {
    if format & 0b11100000 == 0 {
      CPU::move_shifted_register
    } else if format & 0b11111000 == 0b00011000 {
      CPU::add_subtract
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
    } else if format & 0b11110000 == 0b11010000 {
      CPU::conditional_branch
    } else if format == 0b11011111 {
      CPU::software_interrupt
    } else if format & 0b11111000 == 0b11100000 {
      CPU::unconditional_branch
    } else if format & 0b11110000 == 0b11110000 {
      CPU::long_branch_link
    } else {
      CPU::panic
    }
  }

  fn move_shifted_register(&mut self, instr: u16) {
    let op_code = ((instr >> 11) & 0b11) as u8;
    let offset5 = ((instr >> 6) & 0b11111) as u8;
    let rs = ((instr >> 3) & 0b111) as u8;
    let rd = (instr & 0b111) as u8;

    match op_code {
      0 => self.lsl_offset(offset5, rs, rd),
      1 => self.lsr_offset(offset5, rs, rd),
      2 => self.asr_offset(offset5, rs, rd),
      _ => panic!("invalid op")
    }
  }

  pub fn populate_thumb_lut(&mut self) {
    for i in 0..256 {
      let instr_fn = self.decode_instruction(i);
      self.thumb_lut.push(instr_fn);
    }
  }

  fn add_subtract(&mut self, instr: u16) {
    let op_code = (instr >> 9) & 0b1;
    let rn_offset = (instr >> 6) & 0b111;
    let is_immediate = (instr >> 10) & 0b1 == 1;

    let rs = (instr >> 3) & 0b111;
    let rd = instr & 0b111;

    let operand1 = self.r[rs as usize];
    let operand2 = if is_immediate { rn_offset as u32 } else { self.r[rn_offset as usize] };

    self.r[rd as usize] = if op_code == 0 {
      self.add(operand1, operand2)
    } else {
      self.subtract(operand1, operand2)
    };
  }

  fn panic(&mut self, instr: u16) {
    panic!("unsupported instruction: {:X}", instr);
  }

  fn move_compare_add_sub_imm(&mut self, instr: u16) {
    let op_code = (instr >> 11) & 0b11;
    let rd = (instr >> 8) & 0b111;
    let offset = instr & 0b11111111;

    match op_code {
      0 => self.mov(rd, offset),
      1 => self.cmp(self.r[rd as usize], offset as u32),
      2 => self.r[rd as usize] = self.add(self.r[rd as usize], offset as u32),
      3 => self.r[rd as usize] = self.subtract(self.r[rd as usize], offset as u32),
      _ => unreachable!("impossible")
    }
  }

  fn alu_operations(&mut self, instr: u16) {
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
      7 => self.r[rd as usize] = self.ror(self.r[rd as usize], self.r[rs as usize]),
      8 => { self.and(self.r[rs as usize], self.r[rd as usize]); },
      9 => self.r[rd as usize] = self.subtract(0, self.r[rs as usize]),
      10 => { self.subtract(self.r[rd as usize], self.r[rs as usize]); },
      11 => { self.add(self.r[rd as usize], self.r[rs as usize]); },
      12 => self.r[rd as usize] = self.or(self.r[rd as usize], self.r[rs as usize]),
      13 => self.r[rd as usize] = self.mul(self.r[rd as usize], self.r[rs as usize]),
      14 => self.r[rd as usize] = self.bic(self.r[rd as usize] ,self.r[rs as usize]),
      15 => self.r[rd as usize] = self.mvn(self.r[rd as usize], self.r[rs as usize]),
      _ => unreachable!("impossible")
    }
  }

  fn hi_register_ops(&mut self, instr: u16) {

  }

  fn pc_relative_load(&mut self, instr: u16) {

  }

  fn load_store_reg_offset(&mut self, instr: u16) {

  }

  fn load_store_signed_byte_halfword(&mut self, instr: u16) {

  }

  fn load_store_immediate_offset(&mut self, instr: u16) {

  }

  fn load_store_halfword(&mut self, instr: u16) {

  }

  fn sp_relative_load_store(&mut self, instr: u16) {

  }

  fn load_address(&mut self, instr: u16) {

  }

  fn add_offset_to_sp(&mut self, instr: u16) {

  }

  fn push_pop_registers(&mut self, instr: u16) {

  }

  fn multiple_load_store(&mut self, instr: u16) {

  }

  fn conditional_branch(&mut self, instr: u16) {

  }

  fn software_interrupt(&mut self, instr: u16) {

  }

  fn unconditional_branch(&mut self, instr: u16) {

  }

  fn long_branch_link(&mut self, instr: u16) {

  }

  fn mov(&mut self, rd: u16, val: u16) {

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

  fn ror(&mut self, operand: u32, shift: u32) -> u32 {
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
    let carry = ((operand >> (shift - 1)) & 0b1) == 1;
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

  fn mvn(&mut self, operand1: u32, operand2: u32) -> u32 {
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
}