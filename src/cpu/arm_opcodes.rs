use crate::cpu::{PC_REGISTER, PSRRegister, LR_REGISTER};

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

    self.pc = self.pc.wrapping_add(4);
    Some(MemoryAccess::Sequential)
  }

  fn multiply(&mut self, instr: u32) -> Option<MemoryAccess> {
    println!("inside multiply");
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

    // pipeline is now flushed
    None
  }

  fn halfword_data_transfer_register(&mut self, instr: u32) -> Option<MemoryAccess>  {
    println!("inside halfword data transfer register");

    self.pc.wrapping_add(4);
    Some(MemoryAccess::Sequential)
  }

  fn halfword_data_transfer_immediate(&mut self, instr: u32) -> Option<MemoryAccess>  {
    println!("inside halfword data transfer immediate");

    self.pc.wrapping_add(4);
    Some(MemoryAccess::Sequential)
  }

  fn single_data_transfer(&mut self, instr: u32) -> Option<MemoryAccess>  {
    println!("inside single data transfer");

    self.pc.wrapping_add(4);
    Some(MemoryAccess::Sequential)
  }

  fn block_data_transfer(&mut self, instr: u32) -> Option<MemoryAccess>  {
    println!("inside block data transfer");

    self.pc.wrapping_add(4);
    Some(MemoryAccess::Sequential)
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

    // flush
    None
  }

  fn arm_software_interrupt(&mut self, instr: u32) -> Option<MemoryAccess>  {
    println!("inside arm software interrupt");

    self.pc = self.pc.wrapping_add(4);
    Some(MemoryAccess::Sequential)
  }
}