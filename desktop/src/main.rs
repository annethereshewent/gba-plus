extern crate gba_emulator;

use std::{fs, env, collections::HashMap};

use gba_emulator::{cpu::{CPU, registers::key_input_register::KeyInputRegister}, gpu::{SCREEN_WIDTH, SCREEN_HEIGHT, CYCLES_PER_FRAME}};
use sdl2::{pixels::PixelFormatEnum, event::Event, keyboard::Keycode};


fn main() {
  let mut cpu = CPU::new();

  cpu.skip_bios();

  let args: Vec<String> = env::args().collect();

  if args.len() != 2 {
    panic!("please specify a file");
  }

  let filepath = &args[1];

  let bytes: Vec<u8> = fs::read(filepath).unwrap();

  cpu.load_game(bytes, filepath.to_string());
  cpu.load_bios(fs::read("../gba_bios.bin").unwrap());

  let sdl_context = sdl2::init().unwrap();
  let video_subsystem = sdl_context.video().unwrap();

  let mut key_map = HashMap::new();

  key_map.insert(Keycode::W, KeyInputRegister::Up);
  key_map.insert(Keycode::S, KeyInputRegister::Down);
  key_map.insert(Keycode::D, KeyInputRegister::Right);
  key_map.insert(Keycode::A, KeyInputRegister::Left);

  key_map.insert(Keycode::Space, KeyInputRegister::ButtonA);
  key_map.insert(Keycode::K, KeyInputRegister::ButtonA);

  key_map.insert(Keycode::LShift, KeyInputRegister::ButtonB);
  key_map.insert(Keycode::J, KeyInputRegister::ButtonB);

  key_map.insert(Keycode::C, KeyInputRegister::ButtonL);
  key_map.insert(Keycode::V, KeyInputRegister::ButtonR);

  key_map.insert(Keycode::Return, KeyInputRegister::Start);
  key_map.insert(Keycode::Tab, KeyInputRegister::Select);


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
        Event::KeyDown { keycode, .. } => {
          if let Some(button) = key_map.get(&keycode.unwrap_or(Keycode::Return)) {
            cpu.key_input.set(*button, false);
          }
        }
        Event::KeyUp { keycode, .. } => {
          if let Some(button) = key_map.get(&keycode.unwrap_or(Keycode::Return)) {
            cpu.key_input.set(*button, true);
          }
        }
        _ => { /* do nothing */ }
      }
    }
  }
}