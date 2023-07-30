extern crate gba_emulator;

use std::{fs, env};

use gba_emulator::cpu::CPU;


fn main() {
  let mut cpu = CPU::new();
  cpu.skip_bios();

  let args: Vec<String> = env::args().collect();

  if args.len() != 2 {
    panic!("please specify a file");
  }

  let filepath = &args[1];

  let bytes: Vec<u8> = fs::read(filepath).unwrap();

  cpu.load_game(bytes);
  cpu.load_bios(fs::read("../gba_bios.bin").unwrap());

  for _ in 0..1024 {
    cpu.step();
  }
}