extern crate gba_emulator;

use std::{collections::HashMap, env, fs, sync::Arc};

use gba_emulator::{cpu::{CPU, registers::key_input_register::KeyInputRegister}, gpu::{SCREEN_WIDTH, SCREEN_HEIGHT, CYCLES_PER_FRAME}, apu::APU};
use ringbuf::{storage::Heap, traits::{Consumer, Split}, wrap::caching::Caching, HeapRb, SharedRb};
use sdl2::{pixels::PixelFormatEnum, event::Event, keyboard::Keycode, audio::{AudioSpecDesired, AudioCallback}};

const NUM_SAMPLES: usize = 8192 * 2;

struct GbaAudioCallback {
  consumer: Caching<Arc<SharedRb<Heap<f32>>>, false, true>
}

impl AudioCallback for GbaAudioCallback {
  type Channel = f32;

  fn callback(&mut self, buf: &mut [Self::Channel]) {
    for b in buf.iter_mut() {
      *b = if let Some(sample) = self.consumer.try_pop() {
        sample
      } else {
        0.0
      };
    }
  }
}


fn main() {

  let args: Vec<String> = env::args().collect();

  if args.len() != 2 {
    panic!("please specify a file");
  }

  let filepath = &args[1];

  let bytes: Vec<u8> = fs::read(filepath).unwrap();

  let ringbuffer = HeapRb::<f32>::new(NUM_SAMPLES);

  let (producer, consumer) = ringbuffer.split();

  let mut cpu = CPU::new(producer);

  cpu.load_game(bytes, Some(filepath.to_string()));
  cpu.load_bios(fs::read("../gba_bios.bin").unwrap());

  cpu.skip_bios();

  let sdl_context = sdl2::init().unwrap();
  let video_subsystem = sdl_context.video().unwrap();
  let audio_subsystem = sdl_context.audio().unwrap();

  let spec = AudioSpecDesired {
    freq: Some(44100),
    channels: Some(2),
    samples: Some(4096)
  };



  let device = audio_subsystem.open_playback(
    None,
    &spec,
    |_| GbaAudioCallback { consumer }
  ).unwrap();

  device.resume();

  let game_controller_subsystem = sdl_context.game_controller().unwrap();

  let available = game_controller_subsystem
      .num_joysticks()
      .map_err(|e| format!("can't enumerate joysticks: {}", e)).unwrap();

  let _controller = (0..available)
    .find_map(|id| {
      match game_controller_subsystem.open(id) {
        Ok(c) => {
          Some(c)
        }
        Err(_) => {
          None
        }
      }
    });

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

  let mut joypad_map = HashMap::new();

  joypad_map.insert(0, KeyInputRegister::ButtonA);
  joypad_map.insert(2, KeyInputRegister::ButtonB);

  joypad_map.insert(6, KeyInputRegister::Start);
  joypad_map.insert(4, KeyInputRegister::Select);

  joypad_map.insert(11, KeyInputRegister::Up);
  joypad_map.insert(12, KeyInputRegister::Down);
  joypad_map.insert(13, KeyInputRegister::Left);
  joypad_map.insert(14, KeyInputRegister::Right);

  joypad_map.insert(9, KeyInputRegister::ButtonL);
  joypad_map.insert(10, KeyInputRegister::ButtonR);


  let window = video_subsystem
    .window("GBA+", (SCREEN_WIDTH * 3) as u32, (SCREEN_HEIGHT * 3) as u32)
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


  loop {
    while !cpu.gpu.frame_finished {
      cpu.step();
    }

    cpu.gpu.cap_fps();

    // TODO: change this to use opengl.
    texture.update(None, &cpu.gpu.picture.data, SCREEN_WIDTH as usize * 3).unwrap();

    canvas.copy(&texture, None, None).unwrap();

    canvas.present();

    cpu.gpu.frame_finished = false;

    for event in event_pump.poll_iter() {
      match event {
        Event::Quit { .. } => std::process::exit(0),
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
        Event::JoyButtonDown { button_idx, .. } => {
          if let Some(button) = joypad_map.get(&button_idx){
            cpu.key_input.set(*button, false);
          }
        }
        Event::JoyButtonUp { button_idx, .. } => {
          if let Some(button) = joypad_map.get(&button_idx){
            cpu.key_input.set(*button, true);
          }
        }
        _ => { /* do nothing */ }
      }
    }
  }
}