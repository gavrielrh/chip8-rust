extern crate sdl2;

use std::fs;
use std::env;
use rand::prelude::*;
use sdl2::rect::Point;
use sdl2::pixels::Color;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use std::time::Duration;

struct CPU {
    memory: [u8; 4096],
    v: [u8; 16],
    i: usize,
    pc: usize,
    gfx: [u8; 64 * 32],
    delay_timer: u8,
    sound_timer: u8,
    stack: [u16; 16],
    sp: usize,
    key: [u8; 16],
    draw_flag: bool,
    key_pressed: bool
}

const FONT_SET: [u8; 80] = [
    0xF0, 0x90, 0x90, 0x90, 0xF0, // 0
    0x20, 0x60, 0x20, 0x20, 0x70, // 1
    0xF0, 0x10, 0xF0, 0x80, 0xF0, // 2
    0xF0, 0x10, 0xF0, 0x10, 0xF0, // 3
    0x90, 0x90, 0xF0, 0x10, 0x10, // 4
    0xF0, 0x80, 0xF0, 0x10, 0xF0, // 5
    0xF0, 0x80, 0xF0, 0x90, 0xF0, // 6
    0xF0, 0x10, 0x20, 0x40, 0x40, // 7
    0xF0, 0x90, 0xF0, 0x90, 0xF0, // 8
    0xF0, 0x90, 0xF0, 0x10, 0xF0, // 9
    0xF0, 0x90, 0xF0, 0x90, 0x90, // A
    0xE0, 0x90, 0xE0, 0x90, 0xE0, // B
    0xF0, 0x80, 0x80, 0x80, 0xF0, // C
    0xE0, 0x90, 0x90, 0x90, 0xE0, // D
    0xF0, 0x80, 0xF0, 0x80, 0xF0, // E
    0xF0, 0x80, 0xF0, 0x80, 0x80  // F
];

impl CPU {
    fn initialize() -> Self {
        let mut memory: [u8; 4096] = [0; 4096];
        memory[0..80].copy_from_slice(&FONT_SET);
        CPU {
            pc: 0x200,
            i: 0,
            sp: 0,
            gfx: [0; 64 * 32],
            stack: [0; 16],
            v: [0; 16],
            memory,
            delay_timer: 0,
            sound_timer: 0,
            key: [0; 16],
            draw_flag: false,
            key_pressed: false
        }
    }

    fn load_program(&mut self, program: &Vec<u8>) {
        self.memory[512..(512 + program.len())].copy_from_slice(program);
    }

    fn fetch(&self) -> u16 {
        (self.memory[self.pc] as u16) << 8 | (self.memory[self.pc + 1] as u16)
    }

    fn emulate_cycle(&mut self) {
        let instruction = self.fetch();
        self.pc += 2;
        let op: u8 = (instruction >> 12) as u8;
        let x: usize = ((instruction & 0x0F00) >> 8) as usize;
        let y: usize = ((instruction & 0x00F0) >> 4) as usize;
        let n: u8 = (instruction & 0x000F) as u8;
        let nn: u8 = (instruction & 0x00FF) as u8;
        let nnn: usize = (instruction & 0x0FFF) as usize;

        match op {
            0x0 => {
                match nnn {
                    0x0000 => {
                        // Ignore
                    },
                    0x00E0 => {
                        self.gfx = [0u8; 64 * 32];
                        self.draw_flag = true;
                    },
                    0x00EE => {
                        self.pc = self.stack[self.sp] as usize;
                        self.sp -= 1;
                    },
                    _ => println!("Unknown opcode: {:?}", instruction)
                }
            },
            0x1 => {
                self.pc = nnn as usize;
            },
            0x2 => {
                self.sp += 1;
                self.stack[self.sp] = self.pc as u16;
                self.pc = nnn as usize;
            },
            0x3 => {
                if self.v[x as usize] == nn as u8 {
                    self.pc += 2;
                }
            },
            0x4 => {
                if self.v[x as usize] != nn as u8 {
                    self.pc += 2;
                }
            },
            0x5 => {
                match n {
                    0 => {
                        if self.v[x] == self.v[y] {
                            self.pc += 2;
                        }
                    },
                    _ => println!("Unknown opcode: {:?}", instruction)
                }
            },
            0x6 => {
                self.v[x] = nn as u8;
            },
            0x7 => {
                self.v[x] = ((self.v[x] as usize + nn as usize) & 0xFF) as u8;
            },
            0x8 => {
                match n {
                    0x0 => {
                        self.v[x] = self.v[y];
                    },
                    0x1 => {
                        self.v[x] = self.v[x] | self.v[y];
                    },
                    0x2 => {
                        self.v[x] = self.v[x] & self.v[y];
                    },
                    0x3 => {
                        self.v[x] = self.v[x] ^ self.v[y];
                    },
                    0x4 => {
                        // there is probably a way cleaner way of doing this
                        let sum: u16 = self.v[x] as u16 + self.v[y] as u16;
                        self.v[x] = (sum & 0xFF) as u8;
                        if sum > 255 {
                            self.v[0xF] = 1;
                        } else {
                            self.v[0xF] = 0;
                        }
                    },
                    0x5 => {
                        if self.v[x] > self.v[y] {
                            self.v[0xF] = 1;
                        } else {
                            self.v[0xF] = 0;
                        }
                        self.v[x] = self.v[x] - (self.v[y] & self.v[x]);
                    },
                    0x6 => {
                        if self.v[x] & 1 == 1 {
                            self.v[0xF] = 1;
                        } else {
                            self.v[0xF] = 0;
                        }
                        self.v[x] >>= 2;
                    },
                    0x7 => {
                        if self.v[y] > self.v[x] {
                            self.v[0xF] = 1;
                        } else {
                            self.v[0xF] = 0;
                        }
                        self.v[x] = self.v[y] - (self.v[x] & self.v[y]);
                    },
                    0xE => {
                        if self.v[x] & 0x80 == 0x80 {
                            self.v[0xF] = 1;
                        } else {
                            self.v[0xF] = 0;
                        }
                        self.v[x] <<= 2;
                    },
                    _ => println!("Unknown opcode: {:?}", instruction)
                }
            },
            0x9 => {
                match n {
                    0x0 => {
                        if self.v[x] != self.v[y] {
                            self.pc += 2;
                        }
                    },
                    _ => println!("Unknown opcode: {:?}", instruction)
                }
            },
            0xA => {
                self.i = nnn;
            },
            0xB => {
                self.pc = nnn + self.v[0] as usize;
            },
            0xC => {
                let num: u8 = random();
                self.v[x] = num & nn as u8;
            },
            0xD => {
                let x_coord = self.v[x];
                let y_coord = self.v[y];
                self.v[0xF] = 0;
                for row in 0..n {
                    let row_pixels = self.memory[self.i + row as usize];
                    for col in 0..8 {
                        // Current pixel is on
                        if row_pixels & (0x80 >> col) != 0 {
                            let curr_x: usize = (x_coord as usize + col as usize) % 64;
                            let curr_y: usize = (y_coord as usize + row as usize) % 32;
                            let i = curr_x + (64 * curr_y);
                            // Pixel at X,Y on screen is on
                            if self.gfx[i] == 1 {
                                // Collision (trying to draw on top of drawn pixel)
                                self.v[0xF] = 1;
                            }
                            // Toggle the pixel on the screen
                            self.gfx[i] ^= 1;
                        }
                    }
                }
                self.draw_flag = true;
            },
            0xE => {
                match nn {
                    0x9E => {
                        if self.key[self.v[x] as usize] != 0 {
                            self.pc += 2;
                        }
                    },
                    0xA1 => {
                        if self.key[self.v[x] as usize] == 0 {
                            self.pc += 2;
                        }
                    },
                    _ => println!("Unknown opcode: {:?}", instruction)
                }
            },
            0xF => {
                match nn {
                    0x07 => {
                        self.v[x] = self.delay_timer;
                    },
                    0x0A => {
                        if !self.key_pressed {
                            self.pc -= 2;
                        }
                    },
                    0x15 => {
                        self.delay_timer = self.v[x];
                    },
                    0x18 => {
                        self.sound_timer = self.v[x];
                    },
                    0x1E => {
                        self.i = self.i + self.v[x] as usize;
                    },
                    0x29 => {
                        self.i = self.memory[self.v[x] as usize] as usize;
                    },
                    0x33 => {
                        self.memory[self.i] = self.v[x] / 100;
                        self.memory[self.i + 1] = (self.v[x] / 10) % 10;
                        self.memory[self.i + 1] = (self.v[x] % 100) % 10;
                    },
                    0x55 => {
                        self.memory[(self.i)..(self.i + 16)].copy_from_slice(&self.v);
                    },
                    0x65 => {
                        self.v.copy_from_slice(&self.memory[(self.i)..(self.i + 16)]);
                    },
                    _ => println!("Unknown opcode: {:?}", instruction)
                }
            },
            _ => println!("Unknown opcode: {:?}", instruction)
        }

        if self.delay_timer > 0 {
            self.delay_timer -= 1;
        }
        if self.sound_timer > 0 {
            println!("BEEP!");
            self.sound_timer -= 1;
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error + 'static>> {
    let mut chip8 = CPU::initialize();
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        panic!("Must include ROM filepath as argument")
    }
    let program_filepath = &args[1];
    let program = fs::read(program_filepath)?;
    chip8.load_program(&program);

    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    let window = video_subsystem.window("CHIP-8", 640, 320)
        .position_centered()
        .build()
        .unwrap();
    
    let mut canvas = window.into_canvas().build().unwrap();
    canvas.set_scale(10.0, 10.0)?;
    canvas.set_draw_color(Color::RGB(0, 0, 0));
    canvas.clear();
    canvas.present();
    let mut event_pump = sdl_context.event_pump().unwrap();
    'running: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit {..} => {
                    break 'running
                },
                Event::KeyDown { keycode, .. } => {
                    chip8.key_pressed = true;
                    println!("Pressed {:?}", keycode);
                    let chip8_key = match keycode {
                        Some(Keycode::Num1) => Some(0x1),
                        Some(Keycode::Num2) => Some(0x2),
                        Some(Keycode::Num3) => Some(0x3),
                        Some(Keycode::Num4) => Some(0xC),
                        Some(Keycode::Q) => Some(0x4),
                        Some(Keycode::W) => Some(0x5),
                        Some(Keycode::E) => Some(0x6),
                        Some(Keycode::R) => Some(0xD),
                        Some(Keycode::A) => Some(0x7),
                        Some(Keycode::S) => Some(0x8),
                        Some(Keycode::D) => Some(0x9),
                        Some(Keycode::F) => Some(0xE),
                        Some(Keycode::Z) => Some(0xA),
                        Some(Keycode::X) => Some(0x0),
                        Some(Keycode::C) => Some(0xB),
                        Some(Keycode::V) => Some(0xF),
                        _ => None
                    };
                    if let Some(ckey) = chip8_key {
                        chip8.key[ckey] = 1;
                    } else {
                        println!("Unknown Keycode: {:?}", keycode);
                    }
                },
                _ => {}
            }
        }
        chip8.emulate_cycle();

        if chip8.draw_flag {
            canvas.set_draw_color(Color::RGB(0, 0, 0));
            canvas.clear();
            canvas.set_draw_color(Color::RGB(255, 255, 255));
            for i in 0..chip8.gfx.len() {
                if chip8.gfx[i] != 0 {
                    let x = (i % 64) as i32;
                    let y = (i / 64) as i32;
                    canvas.draw_point(Point::new(x, y))?;
                }
            }

            chip8.draw_flag = false;
            canvas.present();
        }

        chip8.key = [0u8; 16];
        chip8.key_pressed = false;
        ::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 60));
    }
    Ok(())
}
