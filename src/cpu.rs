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

#[derive(Copy, Clone, PartialEq)]
enum Immediate {
    None,
    Imm8(u8),
    Imm16(u16),
}

fn add(a:u8, b:u8, c:u8, cf_out:&mut bool, hf_out:&mut bool) -> u8 {
    *hf_out = (((a & 0x0F) + (b & 0x0F) + c) & 0xF0 != 0);
    let [h,l] = ((a as u16) + (b as u16) + (c as u16)).to_be_bytes();
    *cf_out = (h != 0);
    l
}

fn add16(a:u16, b:u16, c:u8, cf_out:&mut bool, hf_out:&mut bool) -> u16 {
    let [ah, al] = a.to_be_bytes();
    let [bh, bl] = a.to_be_bytes();
    let rl = add(al, bl, c, cf_out, hf_out);
    let rh = add(ah, bh, (if *cf_out {1} else {0}), cf_out, hf_out);
    word(rh, rl)
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
                3 => {let l = self.fetch(); Immediate::Imm16(word(self.fetch(), l))},
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

    fn readloc8(&self, loc:Location8, imm:Immediate) -> u8 {
        use Location8::*;
        match loc {
            Empty => 0,
            A | A_RO => self.a,
            B => self.b,
            C => self.c,
            D => self.d,
            E => self.e,
            H => self.h,
            L => self.l,
            IMM8 => match imm {Immediate::Imm8(i) => i,
                        _ => panic!("Expect IMM8!")},
            ADDR_BC => self.mmu.read(word(self.b, self.c)),
            ADDR_DE => self.mmu.read(word(self.d, self.e)),
            ADDR_HL | ADDR_HL_DEC | ADDR_HL_INC => self.mmu.read(word(self.h, self.l)),
            ADDR_IMM16 => match imm {Immediate::Imm16(a) => self.mmu.read(a),
                        _ => panic!("Expect IMM16!")},
            ADDR_C => self.mmu.read(word(0xFF, self.c)),
            ADDR_IMM8 => match imm {Immediate::Imm8(i) => self.mmu.read(word(0xFF, i)),
                        _ => panic!("Expect IMM8!")},
        }
    }

    fn writeloc8(&mut self, loc:Location8, imm:Immediate, value:u8) {
        use Location8::*;
        match loc {
            Empty | A_RO => (),
            A => {self.a = value;},
            B => {self.b = value;},
            C => {self.c = value;},
            D => {self.d = value;},
            E => {self.e = value;},
            H => {self.h = value;},
            L => {self.l = value;},
            IMM8 => panic!("Illegal destination IMM8!"),
            ADDR_BC => self.mmu.write(word(self.b, self.c), value),
            ADDR_DE => self.mmu.write(word(self.d, self.e), value),
            Location8::ADDR_HL | Location8::ADDR_HL_DEC | Location8::ADDR_HL_INC => self.mmu.write(word(self.h, self.l), value),
            ADDR_IMM16 => match imm {Immediate::Imm16(a) => self.mmu.write(a,value),
                        _ => panic!("Expect IMM16!")},
            ADDR_C => self.mmu.write(word(0xFF, self.c), value),
            ADDR_IMM8 => match imm {Immediate::Imm8(i) => self.mmu.write(word(0xFF, i), value),
                        _ => panic!("Expect IMM8!")},
        }
    }

    fn data8(&mut self, op: OpData, dst:Location8, src:Location8, z:FlagOp, n:FlagOp, h:FlagOp, c:FlagOp, bit:u8, imm:Immediate) {
        let s = self.readloc8(src, imm);
        let d = self.readloc8(dst, imm);
        let c_in:u8 = if self.f & FLAG_C == 0 {0} else {1};

        let mut hf_out = false;
        let mut cf_out = false;

        use OpData::*;
        let r = match op {
            ADC => add(d, s, c_in, &mut cf_out, &mut hf_out),
            ADD => add(d, s, 0, &mut cf_out, &mut hf_out),
            AND => s & d,
            BIT => s & bit,
            CP => add(d, !s, 1, &mut cf_out, &mut hf_out),
            CPL => !d,
            DAA => d,
            DEC => d-1,
            INC => add(d, 1, 0, &mut cf_out, &mut hf_out),
            LD | LDH | LDHL | POP | PUSH => s,
            OR => d | s,
            RES => d & !bit,
            RL => d,
            RLC => d,
            RR => d,
            RRC => d,
            SBC => add(d, !s, c_in, &mut cf_out, &mut hf_out),
            SET => d | bit,
            SLA => d,
            SRA => d,
            SRL => d,
            SUB => add(d, !s, 1, &mut cf_out, &mut hf_out),
            SWAP => (d >> 4) | (d << 4),
            XOR => d ^ s,
        };

        use FlagOp::*;
        match z {
            Unaffected => (),
            SetFlag => {self.f |= FLAG_Z;},
            ResetFlag => {self.f &= !FLAG_Z;},
            CalculateFlag => {if r == 0 {self.f |= FLAG_Z;} else {self.f &= !FLAG_Z};},
        };
        match n {
            Unaffected => (),
            SetFlag => {self.f |= FLAG_N;},
            ResetFlag => {self.f &= !FLAG_N;},
            CalculateFlag => panic!("Flag N can not be calculated."),
        };
        match h {
            Unaffected => (),
            SetFlag => {self.f |= FLAG_H;},
            ResetFlag => {self.f &= !FLAG_H;},
            CalculateFlag => {if hf_out {self.f |= FLAG_H} else {self.f &= !FLAG_H};},
        };
        match c {
            Unaffected => (),
            SetFlag => {self.f |= FLAG_C;},
            ResetFlag => {self.f &= !FLAG_C;},
            CalculateFlag => {if cf_out {self.f |= FLAG_C} else {self.f &= !FLAG_C};},
        };

        self.writeloc8(dst, imm, r);

        use Location8::{ADDR_HL_INC, ADDR_HL_DEC};
        if src == ADDR_HL_INC || dst == ADDR_HL_INC {
            let [h, l] = (word(self.h, self.l) + 1).to_be_bytes();
            self.h = h;
            self.l = l;
        }
        if src == ADDR_HL_DEC || dst == ADDR_HL_DEC {
            let [h, l] = (word(self.h, self.l) - 1).to_be_bytes();
            self.h = h;
            self.l = l;
        }
    }

    fn readloc16(&self, loc:Location16, imm:Immediate) -> u16 {
        use Location16::*;
        match loc {
            Empty_W | ADDR_SP_DEC => 0,
            AF => word(self.a, self.f),
            BC => word(self.b, self.c),
            DE => word(self.d, self.e),
            HL => word(self.h, self.l),
            SP => self.sp,
            IMM16 => match imm {Immediate::Imm16(i) => i,
                        _ => panic!("Expect IMM16!")},
            ADDR_SP_INC => word(self.mmu.read(self.sp+1), self.mmu.read(self.sp)),
            ADDR_IMM16_W => match imm {Immediate::Imm16(a) => word(self.mmu.read(a+1), self.mmu.read(a)),
                        _ => panic!("Expect IMM16!")},
        }
    }

    fn writeloc16(&mut self, loc:Location16, imm:Immediate, value:u16) {
        use Location16::*;
        let [vh, vl] = value.to_be_bytes();
        match loc {
            AF => {self.a = vh; self.f = vl;},
            BC => {self.b = vh; self.c = vl;},
            DE => {self.d = vh; self.e = vl;},
            HL => {self.h = vh; self.l = vl;},
            SP => {self.sp = value;}
            Empty_W | IMM16 | ADDR_SP_INC => panic!("Illegal destination IMM!"),
            ADDR_SP_DEC => {self.mmu.write(self.sp-1, vh); self.mmu.write(self.sp-2, vl);},
            ADDR_IMM16_W => match imm {Immediate::Imm16(a) => {self.mmu.write(a+1, vh); self.mmu.write(a, vl);},
                        _ => panic!("Expect IMM16!")},
        }
    }

    fn data16(&mut self, op: OpData, dst:Location16, src:Location16, z:FlagOp, n:FlagOp, h:FlagOp, c:FlagOp, imm:Immediate) {

        let s = self.readloc16(src, imm);
        let d = self.readloc16(dst, imm);
        let c_in:u8 = if self.f & FLAG_C == 0 {0} else {1};

        let mut hf_out = false;
        let mut cf_out = false;

        use OpData::*;
        let r = match op {
            ADD => add16(d, s, 0, &mut cf_out, &mut hf_out),
            DEC => d-1,
            INC => add16(d, 1, 0, &mut cf_out, &mut hf_out),
            LD => s,
            _ => panic!("operation not available for 16bit"),
        };

        use FlagOp::*;
        match z {
            Unaffected => (),
            SetFlag => {self.f |= FLAG_Z;},
            ResetFlag => {self.f &= !FLAG_Z;},
            CalculateFlag => {if r == 0 {self.f |= FLAG_Z;} else {self.f &= !FLAG_Z};},
        };
        match n {
            Unaffected => (),
            SetFlag => {self.f |= FLAG_N;},
            ResetFlag => {self.f &= !FLAG_N;},
            CalculateFlag => panic!("Flag N can not be calculated."),
        };
        match h {
            Unaffected => (),
            SetFlag => {self.f |= FLAG_H;},
            ResetFlag => {self.f &= !FLAG_H;},
            CalculateFlag => {if hf_out {self.f |= FLAG_H} else {self.f &= !FLAG_H};},
        };
        match c {
            Unaffected => (),
            SetFlag => {self.f |= FLAG_C;},
            ResetFlag => {self.f &= !FLAG_C;},
            CalculateFlag => {if cf_out {self.f |= FLAG_C} else {self.f &= !FLAG_C};},
        };

        self.writeloc16(dst, imm, r);

        if src == Location16::ADDR_SP_INC {
            self.sp += 2;
        }
        if dst == Location16::ADDR_SP_DEC {
            self.sp -= 2;
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
            DATA16 {op, dst, src, z, n, h, c, } => self.data16(op, dst, src, z, n, h, c, imm),
            DATA8 {op, dst, src, z, n, h, c, bit} => self.data8(op, dst, src, z, n, h, c, bit, imm),
            JUMP  {op, cond, rst_target} => if self.condition_satisfied(cond) {self.jump(op, rst_target, imm)},
            SPIMM8 {dst} => (),
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
    for i in 0..100000 {
        cpu.step();
    }
}
