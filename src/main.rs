extern crate sdl2;

use rand::prelude::*;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::Point;
use std::env;
use std::fs;
use std::time::Duration;

const MEMORY_SIZE: usize = 4096;
const PROGRAM_START: usize = 512;

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
    0xF0, 0x80, 0xF0, 0x80, 0x80, // F
];

struct CPU {
    memory: [u8; MEMORY_SIZE],
    v: [u8; 16],
    i: u16,
    pc: usize,
    gfx: [u8; 64 * 32],
    delay_timer: u8,
    sound_timer: u8,
    stack: Vec<usize>,
    key: [u8; 16],
    draw_flag: bool,
    key_pressed: Option<u8>,
}

#[derive(Debug)]
struct Instruction {
    op: u8,
    x: u8,
    y: u8,
    n: u8,
    nn: u8,
    nnn: u16,
}

impl CPU {
    fn initialize() -> Self {
        let mut memory: [u8; MEMORY_SIZE] = [0; MEMORY_SIZE];
        memory[0..FONT_SET.len()].copy_from_slice(&FONT_SET);
        CPU {
            pc: 0x200,
            i: 0,
            gfx: [0; 64 * 32],
            stack: vec![],
            v: [0; 16],
            memory,
            delay_timer: 0,
            sound_timer: 0,
            key: [0; 16],
            draw_flag: false,
            key_pressed: None,
        }
    }

    fn load_program(&mut self, program: &Vec<u8>) {
        self.memory[PROGRAM_START..(PROGRAM_START + program.len())].copy_from_slice(program);
    }

    fn fetch(&mut self) -> u16 {
        let instruction = (self.memory[self.pc] as u16) << 8 | (self.memory[self.pc + 1] as u16);
        self.pc += 2;
        instruction
    }

    fn decode(&self, bytes: u16) -> Instruction {
        let op: u8 = (bytes >> 12) as u8;
        let x: u8 = ((bytes & 0x0F00) >> 8) as u8;
        let y: u8 = ((bytes & 0x00F0) >> 4) as u8;
        let n: u8 = (bytes & 0x000F) as u8;
        let nn: u8 = (bytes & 0x00FF) as u8;
        let nnn: u16 = bytes & 0x0FFF;
        Instruction {
            op,
            x,
            y,
            n,
            nn,
            nnn,
        }
    }

    /// CLS
    fn e_00e0(&mut self) {
        self.gfx = [0u8; 64 * 32];
        self.draw_flag = true;
    }

    /// JP addr
    fn e_1nnn(&mut self, nnn: u16) {
        self.pc = nnn as usize;
    }

    /// RET
    fn e_00ee(&mut self) {
        self.pc = self.stack.pop().expect("Value on stack");
    }

    /// CALL addr
    fn e_2nnn(&mut self, nnn: u16) {
        self.stack.push(self.pc);
        self.pc = nnn as usize;
    }

    /// SE Vx, byte
    fn e_3xnn(&mut self, x: u8, nn: u8) {
        if self.v[x as usize] == nn {
            self.pc += 2;
        }
    }

    /// SNE Vx, byte
    fn e_4xnn(&mut self, x: u8, nn: u8) {
        if self.v[x as usize] != nn as u8 {
            self.pc += 2;
        }
    }

    /// SE Vx, Vy
    fn e_5xy0(&mut self, x: u8, y: u8) {
        if self.v[x as usize] == self.v[y as usize] {
            self.pc += 2;
        }
    }

    /// SNE Vx, Vy
    fn e_9xy0(&mut self, x: u8, y: u8) {
        if self.v[x as usize] != self.v[y as usize] {
            self.pc += 2;
        }
    }

    /// LD Vx, byte
    fn e_6xnn(&mut self, x: u8, nn: u8) {
        self.v[x as usize] = nn as u8;
    }

    /// ADD Vx, byte
    fn e_7xnn(&mut self, x: u8, nn: u8) {
        self.v[x as usize] = ((self.v[x as usize] as u16 + nn as u16) & 0xFF) as u8;
    }

    /// LD Vx, Vy
    fn e_8xy0(&mut self, x: u8, y: u8) {
        self.v[x as usize] = self.v[y as usize];
    }

    /// OR Vx, Vy
    fn e_8xy1(&mut self, x: u8, y: u8) {
        self.v[x as usize] = self.v[x as usize] | self.v[y as usize];
    }

    /// AND Vx, Vy
    fn e_8xy2(&mut self, x: u8, y: u8) {
        self.v[x as usize] = self.v[x as usize] & self.v[y as usize];
    }

    /// XOR Vx, Vy
    fn e_8xy3(&mut self, x: u8, y: u8) {
        self.v[x as usize] = self.v[x as usize] ^ self.v[y as usize];
    }

    /// ADD Vx, Vy
    fn e_8xy4(&mut self, x: u8, y: u8) {
        let (sum, overflowed) = self.v[x as usize].overflowing_add(self.v[y as usize]);
        self.v[x as usize] = sum;
        if overflowed {
            self.v[0xF] = 1;
        } else {
            self.v[0xF] = 0;
        }
    }

    /// SUB Vx, Vy
    fn e_8xy5(&mut self, x: u8, y: u8) {
        let (diff, overflowed) = self.v[x as usize].overflowing_sub(self.v[y as usize]);
        self.v[x as usize] = diff;
        if overflowed {
            self.v[0xF] = 1;
        } else {
            self.v[0xF] = 0;
        }
    }

    /// SUBN Vx, Vy
    fn e_8xy7(&mut self, x: u8, y: u8) {
        let (diff, overflowed) = self.v[y as usize].overflowing_sub(self.v[x as usize]);
        self.v[x as usize] = diff;
        if overflowed {
            self.v[0xF] = 1;
        } else {
            self.v[0xF] = 0;
        }
    }

    /// SHR Vx {, Vy}
    fn e_8xy6(&mut self, x: u8, y: u8) {
        if MODERN {
            self.v[x as usize] = self.v[y as usize];
        }
        if self.v[x as usize] & 1 == 1 {
            self.v[0xF] = 1;
        } else {
            self.v[0xF] = 0;
        }
        self.v[x as usize] >>= 1;
    }

    /// SHL Vx {, Vy}
    fn e_8xye(&mut self, x: u8, y: u8) {
        if MODERN {
            self.v[x as usize] = self.v[y as usize];
        }
        if self.v[x as usize] & 0x80 == 0x80 {
            self.v[0xF] = 1;
        } else {
            self.v[0xF] = 0;
        }
        self.v[x as usize] <<= 1;
    }

    /// LD I, addr
    fn e_annn(&mut self, nnn: u16) {
        self.i = nnn;
    }

    /// JP V0, addr
    fn e_bnnn(&mut self, x: u8, nnn: u16) {
        if MODERN {
            self.pc = (nnn + self.v[x as usize] as u16) as usize;
        } else {
            self.pc = (nnn + self.v[0] as u16) as usize;
        }
    }

    /// RND Vx, byte
    fn e_cxnn(&mut self, x: u8, nn: u8) {
        let num: u8 = random();
        self.v[x as usize] = num & nn;
    }

    /// DRW Vx, Vy, nibble
    fn e_dxyn(&mut self, x: u8, y: u8, n: u8) {
        let x_coord = self.v[x as usize];
        let y_coord = self.v[y as usize];
        self.v[0xF] = 0;
        for row in 0..n {
            let row_pixels = self.memory[self.i as usize + row as usize];
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
    }

    /// SKP Vx
    fn e_ex9e(&mut self, x: u8) {
        if self.key[self.v[x as usize] as usize] != 0 {
            self.pc += 2;
        }
    }

    /// SKNP Vx
    fn e_exa1(&mut self, x: u8) {
        if self.key[self.v[x as usize] as usize] == 0 {
            self.pc += 2;
        }
    }

    /// LD Vx, DT
    fn e_fx07(&mut self, x: u8) {
        self.v[x as usize] = self.delay_timer;
    }

    /// LD DT, Vx
    fn e_fx15(&mut self, x: u8) {
        self.delay_timer = self.v[x as usize];
    }

    /// LD ST, Vx
    fn e_fx18(&mut self, x: u8) {
        self.sound_timer = self.v[x as usize];
    }

    /// ADD I, Vx
    fn e_fx1e(&mut self, x: u8) {
        self.i = self.i + self.v[x as usize] as u16;
    }

    /// LD Vx, K
    fn e_fx0a(&mut self, x: u8) {
        if let Some(key) = self.key_pressed {
            self.v[x as usize] = key;
        } else {
            self.pc -= 2;
        }
    }

    /// LD F, Vx
    fn e_fx29(&mut self, x: u8) {
        self.i = self.memory[self.v[x as usize] as usize] as u16;
    }

    /// LD B, Vx
    fn e_fx33(&mut self, x: u8) {
        self.memory[self.i as usize] = self.v[x as usize] / 100;
        self.memory[self.i as usize + 1] = (self.v[x as usize] / 10) % 10;
        self.memory[self.i as usize + 2] = (self.v[x as usize] % 100) % 10;
    }

    /// LD [I], Vx
    fn e_fx55(&mut self, x: u8) {
        self.memory[(self.i as usize)..=(self.i as usize + x as usize)]
            .copy_from_slice(&self.v[0..=x as usize]);
    }

    /// LD Vx, [I]
    fn e_fx65(&mut self, x: u8) {
        self.v[0..=x as usize]
            .copy_from_slice(&self.memory[(self.i as usize)..=(self.i as usize + x as usize)]);
    }

    fn e_unknown(&mut self, instruction: Instruction) {
        println!("Unknown Instruction: {:?}", instruction);
    }

    fn execute(&mut self, instruction: Instruction) {
        let op = instruction.op;
        let x = instruction.x;
        let y = instruction.y;
        let n = instruction.n;
        let nn = instruction.nn;
        let nnn = instruction.nnn;
        match op {
            0x0 => match nnn {
                0x0E0 => self.e_00e0(),
                0x0EE => self.e_00ee(),
                _ => self.e_unknown(instruction),
            },
            0x1 => self.e_1nnn(nnn),
            0x2 => self.e_2nnn(nnn),
            0x3 => self.e_3xnn(x, nn),
            0x4 => self.e_4xnn(x, nn),
            0x5 => match n {
                0x0 => self.e_5xy0(x, y),
                _ => self.e_unknown(instruction),
            },
            0x6 => self.e_6xnn(x, nn),
            0x7 => self.e_7xnn(x, nn),
            0x8 => match n {
                0x0 => self.e_8xy0(x, y),
                0x1 => self.e_8xy1(x, y),
                0x2 => self.e_8xy2(x, y),
                0x3 => self.e_8xy3(x, y),
                0x4 => self.e_8xy4(x, y),
                0x5 => self.e_8xy5(x, y),
                0x6 => self.e_8xy6(x, y),
                0x7 => self.e_8xy7(x, y),
                0xE => self.e_8xye(x, y),
                _ => self.e_unknown(instruction),
            },
            0x9 => match n {
                0x0 => self.e_9xy0(x, y),
                _ => self.e_unknown(instruction),
            },
            0xA => self.e_annn(nnn),
            0xB => self.e_bnnn(x, nnn),
            0xC => self.e_cxnn(x, nn),
            0xD => self.e_dxyn(x, y, n),
            0xE => match nn {
                0x9E => self.e_ex9e(x),
                0xA1 => self.e_exa1(x),
                _ => self.e_unknown(instruction),
            },
            0xF => match nn {
                0x07 => self.e_fx07(x),
                0x0A => self.e_fx0a(x),
                0x15 => self.e_fx15(x),
                0x18 => self.e_fx18(x),
                0x1E => self.e_fx1e(x),
                0x29 => self.e_fx29(x),
                0x33 => self.e_fx33(x),
                0x55 => self.e_fx55(x),
                0x65 => self.e_fx65(x),
                _ => self.e_unknown(instruction),
            },
            _ => self.e_unknown(instruction),
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

fn keycode_to_hex(keycode: Keycode) -> Option<u8> {
    match keycode {
        Keycode::Num1 => Some(0x1),
        Keycode::Num2 => Some(0x2),
        Keycode::Num3 => Some(0x3),
        Keycode::Num4 => Some(0xC),
        Keycode::Q => Some(0x4),
        Keycode::W => Some(0x5),
        Keycode::E => Some(0x6),
        Keycode::R => Some(0xD),
        Keycode::A => Some(0x7),
        Keycode::S => Some(0x8),
        Keycode::D => Some(0x9),
        Keycode::F => Some(0xE),
        Keycode::Z => Some(0xA),
        Keycode::X => Some(0x0),
        Keycode::C => Some(0xB),
        Keycode::V => Some(0xF),
        _ => None,
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

    let window = video_subsystem
        .window("CHIP-8", 640, 320)
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
                Event::Quit { .. } => break 'running,
                Event::KeyDown { keycode, .. } => {
                    let keycode = keycode.expect("Some key pressed");
                    if let Some(key_hex) = keycode_to_hex(keycode) {
                        if Some(key_hex) != chip8.key_pressed {
                            println!("Pressed {:?}", keycode);
                        }
                        chip8.key_pressed = Some(key_hex);
                    } else {
                        println!("Unknown key");
                    }
                }
                Event::KeyUp { .. } => {
                    chip8.key_pressed = None;
                }
                _ => {}
            }
        }
        let bytes = chip8.fetch();
        let instruction = chip8.decode(bytes);
        chip8.execute(instruction);

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

        ::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / FPS));
    }
    Ok(())
}

const FPS: u32 = 60;
const MODERN: bool = true;
