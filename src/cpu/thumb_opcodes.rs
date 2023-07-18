use super::CPU;

impl CPU {
  pub fn decode_instruction(&mut self, instruction: u16) {
    let format = instruction >> 8;

    if format & 0b111 == 0 {

    } else if format & 0b11111 == 0b00011 {

    } else if
  }

  fn move_shifted_register(&mut self, instr: u16) {

  }
}