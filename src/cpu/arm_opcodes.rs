use super::CPU;

impl CPU {
  pub fn populate_arm_lut(&mut self) {
    for i in 0..4096 {
      let instr_fn = self.decode_arm((i & 0xff0) >> 4, i & 0xf);
      self.arm_lut.push(instr_fn);
    }
  }

  fn decode_arm(&mut self, upper: u16, lower: u16) -> fn(&mut CPU, instr: u32) {
    if upper & 0b11000000 == 0 {
      CPU::data_processing
    } else if upper & 0b11111100 == 0 && lower == 0b1001 {
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

  fn arm_panic(&mut self, instr: u32) {
    panic!("unsupported instr: {:b}", instr)
  }

  fn data_processing(&mut self, instr: u32) {
    println!("inside data processing");
  }

  fn multiply(&mut self, instr: u32) {
    println!("inside multiply");
  }

  fn multiply_long(&mut self, instr: u32) {
    println!("inside multiply long");
  }

  fn single_data_swap(&mut self, instr: u32) {
    println!("inside single data swap");
  }

  fn branch_and_exchange(&mut self, instr: u32) {
    println!("inside branch and exchange");
  }

  fn halfword_data_transfer_register(&mut self, instr: u32) {
    println!("inside halfword data transfer register");
  }

  fn halfword_data_transfer_immediate(&mut self, instr: u32) {
    println!("inside halfword data transfer immediate");
  }

  fn single_data_transfer(&mut self, instr: u32) {
    println!("inside single data transfer");
  }

  fn block_data_transfer(&mut self, instr: u32) {
    println!("inside block data transfer");
  }

  fn branch(&mut self, instr: u32) {
    println!("inside branch");
  }

  fn arm_software_interrupt(&mut self, instr: u32) {
    println!("inside arm software interrupt");
  }
}