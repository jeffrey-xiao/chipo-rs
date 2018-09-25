extern crate cfg_if;
extern crate js_sys;
extern crate wasm_bindgen;

mod utils;

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

const SCREEN_ROWS: usize = 32;
const SCREEN_COLS: usize = 64;
const FONTSET: [u8; 80] = [
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

#[wasm_bindgen]
pub struct Cpu {
    memory: [u8; 4096],
    registers: [u8; 16],
    index: u16,
    pc: u16,
    // TODO: optimize with bit manipulation
    screen: [u8; SCREEN_ROWS * SCREEN_COLS],
    delay_timer: u8,
    sound_timer: u8,
    stack: [u16; 16],
    sp: u16,
    keys: [bool; 16],
    should_draw: bool,
}

#[wasm_bindgen]
impl Cpu {
    fn clear_screen(&mut self) {
        for pixel in self.screen.iter_mut() {
            *pixel = 0;
        }
        self.should_draw = true;
    }

    pub fn new() -> Self {
        utils::set_panic_hook();
        return Cpu {
            memory: [0; 4096],
            registers: [0; 16],
            index: 0,
            pc: 0,
            screen: [0; SCREEN_ROWS * SCREEN_COLS],
            delay_timer: 0,
            sound_timer: 0,
            stack: [0; 16],
            sp: 0,
            keys: [false; 16],
            should_draw: false,
        }
    }

    pub fn initialize(&mut self) {
        for i in self.memory.iter_mut() {
            *i = 0;
        }

        for (d, s) in self.memory[0..80].iter_mut().zip(FONTSET.iter()) {
            *d = *s;
        }

        for i in self.registers.iter_mut() {
            *i = 0;
        }

        self.index = 0;
        self.pc = 0x200;
        self.clear_screen();
        self.delay_timer = 0;
        self.sound_timer = 0;

        for i in self.stack.iter_mut() {
            *i = 0;
        }

        self.sp = 0;

        for i in self.keys.iter_mut() {
            *i = false;
        }

        self.should_draw = false;
    }

    // TODO: Add file input using web-sys when crate gets published
    pub fn load_rom(&mut self, rom: &[u8]) {
        for (i, byte) in rom.iter().enumerate() {
            self.memory[i + 0x200] = *byte;
        }
    }

    pub fn fetch_opcode(&self) -> u16 {
        ((self.memory[self.pc as usize] as u16) << 8) | self.memory[(self.pc + 1) as usize] as u16
    }

    pub fn execute_cycle(&mut self) {
        let opcode = self.fetch_opcode();
        self.pc += 2;

        self.process_opcode(opcode);

        if self.delay_timer > 0 {
            self.delay_timer -= 1;
        }

        if self.sound_timer > 0 {
            self.sound_timer -= 1;
            log("BEEP!");
        }
    }

    pub fn process_opcode(&mut self, opcode: u16) {
        let tokens = (
            (opcode & 0xF000) >> 12,
            (opcode & 0x0F00) >> 8,
            (opcode & 0x00F0) >> 4,
            (opcode & 0x000F),
        );

        let x = tokens.1 as usize;
        let y = tokens.2 as usize;
        let nnn = opcode & 0x0FFF;
        let kk = (opcode & 0x00FF) as u8;
        let n = (opcode & 0x000F) as usize;

        match tokens {
            (0x0, 0x0, 0xE, 0x0) => self.clear_screen(),
            (0x0, 0x0, 0xE, 0xE) => {
                self.sp -= 1;
                self.pc = self.stack[self.sp as usize];
            },
            (0x1, _, _, _) => self.pc = nnn,
            (0x2, _, _, _) => {
                self.stack[self.sp as usize] = self.pc;
                self.sp += 1;
                self.pc = nnn;
            },
            (0x3, _, _, _) => {
                if self.registers[x] == kk {
                    self.pc += 2;
                }
            },
            (0x4, _, _, _) => {
                if self.registers[x] != kk {
                    self.pc += 2;
                }
            },
            (0x5, _, _, 0x0) => {
                if self.registers[x] == self.registers[y] {
                    self.pc += 2;
                }
            },
            (0x6, _, _, _) => {
                self.registers[x] = kk;
            },
            (0x7, _, _, _) => {
                self.registers[x] += kk;
            },
            (0x8, _, _, 0x0) => self.registers[x] = self.registers[y],
            (0x8, _, _, 0x1) => self.registers[x] |= self.registers[y],
            (0x8, _, _, 0x2) => self.registers[x] &= self.registers[y],
            (0x8, _, _, 0x3) => self.registers[x] ^= self.registers[y],
            (0x8, _, _, 0x4) => {
                let (res, overflow) = self.registers[x].overflowing_add(self.registers[y]);
                self.registers[x] = res;
                if overflow {
                    self.registers[15] = 1;
                } else {
                    self.registers[15] = 0;
                }
            },
            (0x8, _, _, 0x5) => {
                let (res, underflow) = self.registers[x].overflowing_sub(self.registers[y]);
                self.registers[x] = res;
                if underflow {
                    self.registers[15] = 0;
                } else {
                    self.registers[15] = 1;
                }
            },
            (0x8, _, _, 0x6) => {
                self.registers[15] = self.registers[x] & 1;
                self.registers[x] >>= 1;
            },
            (0x8, _, _, 0x7) => {
                let (res, underflow) = self.registers[y].overflowing_sub(self.registers[x]);
                self.registers[x] = res;
                if underflow {
                    self.registers[15] = 0;
                } else {
                    self.registers[15] = 1;
                }
            },
            (0x8, _, _, 0xE) => {
                self.registers[15] = self.registers[x] >> 7;
                self.registers[x] <<= 1;
            },
            (0x9, _, _, 0x0) => {
                if self.registers[x] != self.registers[y] {
                    self.pc += 2;
                }
            },
            (0xA, _, _, _) => self.index = nnn,
            (0xB, _, _, _) => self.pc = self.registers[0] as u16 + nnn,
            (0xC, _, _, _) => {
                let rand = (js_sys::Math::random() * 256.0).floor() as u8;
                self.registers[x] = rand & kk;
            },
            (0xD, _, _, _) => {
                self.registers[15] = 0;

                for row in 0..n {
                    let bitcode = self.memory[self.index as usize + row];
                    for col in 0..8 {
                        if bitcode & (0x80 >> col) != 0 {
                            let screen_index = (self.registers[y] as usize + row) * SCREEN_COLS + self.registers[x] as usize + col;
                            if self.screen[screen_index] != 0 {
                                self.registers[15] = 1;
                            }
                            self.screen[screen_index] ^= 1;
                        }
                    }
                }

                self.should_draw = true;
            }
            (0xE, _, 0x9, 0xE) => {
                if self.keys[self.registers[x] as usize] {
                    self.pc += 2;
                }
            },
            (0xE, _, 0xA, 0x1) => {
                if !self.keys[self.registers[x] as usize] {
                    self.pc += 2;
                }
            },
            (0xF, _, 0x0, 0x7) => self.registers[x] = self.delay_timer,
            (0xF, _, 0x0, 0xA) => {
                self.pc -= 2;
                for (i, key) in self.keys.iter().enumerate() {
                    if *key {
                        self.registers[x] = i as u8;
                        self.pc += 2;
                    }
                }
            },
            (0xF, _, 0x1, 0x5) => self.delay_timer = self.registers[x],
            (0xF, _, 0x1, 0x8) => self.sound_timer = self.registers[x],
            (0xF, _, 0x1, 0xE) => self.index += self.registers[x] as u16,
            (0xF, _, 0x2, 0x9) => self.index = self.registers[x] as u16 * 5,
            (0xF, _, 0x3, 0x3) => {
                self.memory[self.index as usize] = self.registers[x] / 100;
                self.memory[self.index as usize + 1] = ((self.registers[x]) / 10) % 10;
                self.memory[self.index as usize + 2] = self.registers[x] % 10;
            },
            (0xF, _, 0x5, 0x5) => {
                for i in 0..=x {
                    self.memory[self.index as usize + i] = self.registers[i];
                }
            },
            (0xF, _, 0x6, 0x5) => {
                for i in 0..=x {
                    self.registers[i] = self.memory[self.index as usize + i];
                }
            },
            _ => panic!("Unrecognized opcode: {}", opcode),
        }
    }

    pub fn screen(&self) -> *const u8 {
        self.screen.as_ptr()
    }
}

mod tests {
    use std::fs::File;
    use std::io::Read;
    use super::{SCREEN_ROWS, SCREEN_COLS, Cpu};
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test() {
        let mut reader = File::open("../chip-8-web/roms/TEST").unwrap();
        let mut cpu = Cpu::new();
        cpu.initialize();
        let mut buffer = vec![0; 3000];
        reader.read(&mut buffer).unwrap();
        cpu.load_rom(&buffer);

        for i in 0..100000 {
            cpu.execute_cycle();

            print!("{}[2J", 27 as char);
            println!("FRAME {}", i);
            for row in 0..SCREEN_ROWS {
                for col in 0..SCREEN_COLS {
                    if cpu.screen[row * SCREEN_COLS + col] == 0 {
                        print!(" ");
                    } else {
                        print!("█");
                    }
                }
                println!("");
            }
            println!("PC: {} | I: {}", cpu.pc, cpu.index);
            for (index, reg) in cpu.registers.iter().enumerate() {
                println!("V{}: {}", index, reg);
            }
            println!("");
            if i > 200 {
                use std::io::{stdin,stdout,Write};
                let mut s=String::new();
                stdin().read_line(&mut s).expect("Did not enter a correct string");
            }
            cpu.should_draw = false;
        }
    }
}
