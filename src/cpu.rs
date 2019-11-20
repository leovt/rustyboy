use std::fs::File;
use std::io::prelude::*;


use crate::instructions::*;
use Operation::*;

struct Mmu {
    memory:[u8;0x10000],
}

impl Mmu {
    fn write(&mut self, address:u16, value:u8){
        self.memory[address as usize] = value;
    }

    fn read(&self, address:u16) -> u8{
        self.memory[address as usize]
    }

    fn new() -> Mmu {
        Mmu { memory:[0;0x10000] }
    }

    fn load(&mut self, filename: &str, base:u16) {
        let mut f = File::open(filename).expect("file not found");
        let mut data = Vec::new();
        f.read_to_end(&mut data).expect("error reading file");
        for (index, value) in data.iter().enumerate() {
            self.memory[index + base as usize] = *value;
        }
    }
}

const FLAG_Z:u8 = 1<<7;
const FLAG_N:u8 = 1<<6;
const FLAG_H:u8 = 1<<5;
const FLAG_C:u8 = 1<<4;

struct Cpu {
    mmu: Mmu,
    a:u8, f:u8,
    b:u8, c:u8,
    d:u8, e:u8,
    h:u8, l:u8,
    sp: u16,
    pc: u16,
    ie: bool,
    hlt: bool,
}

fn word(h:u8, l:u8) -> u16 {
    (h as u16) << 8 | (l as u16)
}

enum Immediate {
    None,
    Imm8(u8),
    Imm16(u16),
}

impl Cpu {
    fn fetch(&mut self) -> u8 {
        let val = self.mmu.read(self.pc);
        self.pc += 1;
        val
    }

    fn fetch_and_decode(&mut self) -> (Instruction, Immediate) {
        let mut instr = instructions[self.fetch() as usize];
        if instr.operation == PREFIX {
            instr = instructions[self.fetch() as usize + 0x100];
            (instr, Immediate::None)
        }
        else {
            let imm = match instr.length {
                1 => Immediate::None,
                2 => Immediate::Imm8(self.fetch()),
                3 => Immediate::Imm16(word(self.fetch(), self.fetch())),
                _ => panic!("Unecpected instruction length")
            };
            (instr, imm)
        }
    }

    fn condition_satisfied(&self, cond:JumpCondition) -> bool {
        match cond {
            JumpCondition::ALWAYS => true,
            JumpCondition::C  => self.f & FLAG_C != 0,
            JumpCondition::NC => self.f & FLAG_C == 0,
            JumpCondition::Z  => self.f & FLAG_Z != 0,
            JumpCondition::NZ => self.f & FLAG_Z == 0,
        }
    }

    fn jump(&mut self, op: OpJump, rst_target:u8, imm:Immediate){
        use OpJump::*;

        let addr = match imm {
            Immediate::Imm16(addr) => addr,
            Immediate::Imm8(offset) => if offset < 128 {self.pc + offset as u16} else {self.pc + offset as u16 - 0x100},
            Immediate::None => rst_target as u16,
        };

        match op {
            JP | JR => {
                self.pc = addr;
            },
            CALL | RST => {
                let [pch, pcl] = self.pc.to_be_bytes();
                self.sp -= 1;
                self.mmu.write(self.sp, pch);
                self.sp -= 1;
                self.mmu.write(self.sp, pcl);
                self.pc = addr;
            },
            RET | RETI => {
                let pcl = self.mmu.read(self.sp);
                self.sp += 1;
                let pch = self.mmu.read(self.sp);
                self.sp += 1;
                self.pc = word(pch, pcl);
                // TODO handle signalling of completion of interrupt handler for RETI
            },
        }
    }

    fn step(&mut self) {
        let oldpc = self.pc;
        let (instr, imm) = self.fetch_and_decode();
        let instr=instr;
        println!("0x{:04x}:  {:10}  A:{:02x} B:{:02x} C:{:02x} D:{:02x} E:{:02x} H:{:02x} L:{:02x} {}{}{}{}",
                  oldpc, instr.mnemo, self.a, self.b, self.c, self.d, self.e, self.h, self.l,
                  if FLAG_Z & self.f != 0 {"Z"} else {"-"},
                  if FLAG_N & self.f != 0 {"N"} else {"-"},
                  if FLAG_H & self.f != 0 {"H"} else {"-"},
                  if FLAG_C & self.f != 0 {"C"} else {"-"},
              );
        match instr.operation {
            DATA16 {..} => (),
            DATA8 {..} => (),
            JUMP  {op:op, cond:cond, rst_target:tgt} => if self.condition_satisfied(cond) {self.jump(op, tgt, imm)},
            PREFIX => panic!("PREFIX must not occur after decoding."),
            SCF => {self.f = (self.f & !FLAG_H & !FLAG_N) | FLAG_C;},
            CCF => {self.f = (self.f & !FLAG_H & !FLAG_N) ^ FLAG_C;},
            DI => {self.ie = false;},
            EI => {self.ie = true;},
            HALT => {self.hlt = true;},
            NOP => (),
            STOP => {self.hlt = true;}, //treat as HALT for now
            UNDEF => panic!("UNDEF instruction occured."),
        }
    }
}

pub fn main() {
    let mut mmu = Mmu::new();
    mmu.load("DMG_ROM.bin", 0);
    let mut cpu = Cpu{mmu:mmu,
        a:0, f:0,
        b:0, c:0,
        d:0, e:0,
        h:0, l:0,
        sp:0,
        pc:0,
        ie:false,
        hlt:false,
    };
    for i in 0..30 {
        cpu.step();
    }
}
