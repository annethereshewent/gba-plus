extern crate gba_emulator;

use std::{fs, env};

use gba_emulator::{cpu::CPU, gpu::{SCREEN_WIDTH, SCREEN_HEIGHT, CYCLES_PER_FRAME}};
use sdl2::{pixels::PixelFormatEnum, event::Event, keyboard::Keycode, video};


fn main() {
  let mut cpu = CPU::new();

  // cpu.skip_bios();

  let args: Vec<String> = env::args().collect();

  if args.len() != 2 {
    panic!("please specify a file");
  }

  let filepath = &args[1];

  let bytes: Vec<u8> = fs::read(filepath).unwrap();

  cpu.load_game(bytes);
  cpu.load_bios(fs::read("../gba_bios.bin").unwrap());

  let sdl_context = sdl2::init().unwrap();
  let video_subsystem = sdl_context.video().unwrap();

  let window = video_subsystem
    .window("GBA Emulator", (SCREEN_WIDTH * 3) as u32, (SCREEN_HEIGHT * 3) as u32)
    .position_centered()
    .build()
    .unwrap();

  let mut canvas = window.into_canvas().present_vsync().build().unwrap();
  canvas.set_scale(3.0, 3.0).unwrap();

  let mut event_pump = sdl_context.event_pump().unwrap();

  let creator = canvas.texture_creator();
  let mut texture = creator
    .create_texture_target(PixelFormatEnum::RGB24, SCREEN_WIDTH as u32, SCREEN_HEIGHT as u32)
    .unwrap();

  let mut cycles = 0;
  loop {
    while cycles < CYCLES_PER_FRAME {
      cycles += cpu.step();
    }

    cycles = 0;

    texture.update(None, &cpu.gpu.picture.data, SCREEN_WIDTH as usize * 3).unwrap();

    canvas.copy(&texture, None, None).unwrap();

    canvas.present();

    for event in event_pump.poll_iter() {
      match event {
        Event::Quit { .. }
        | Event::KeyDown {
            keycode: Some(Keycode::Escape),
            ..
        } => std::process::exit(0),
        _ => { /* do nothing */ }
      }
    }
  }
}