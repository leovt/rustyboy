use std::fs::File;
use std::io::prelude::*;


use crate::instructions::*;
use Operation::*;

pub struct Mmu {
    memory:[u8;0x10000],
    boot_rom:[u8;0x100],
    boot_rom_enable:bool,
}

impl Mmu {
    pub fn write(&mut self, address:u16, value:u8){
        match address {
            0xff50 => {self.boot_rom_enable = false;},
            0x0000..=0x7fff => (), //panic!("write to rom"),
            _ => {self.memory[address as usize] = value;}
        }
    }

    pub fn read(&self, address:u16) -> u8{
        if address < 0x100 && self.boot_rom_enable {
            self.boot_rom[address as usize]
        } else {
            self.memory[address as usize]
        }
    }

    pub fn new() -> Mmu {
        Mmu {
            memory:[0xff;0x10000],
            boot_rom:[0xff;0x100],
            boot_rom_enable:true,
         }
    }

    pub fn load(&mut self, filename: &str, base:u16) {
        let mut f = File::open(filename).expect("file not found");
        let mut data = Vec::new();
        f.read_to_end(&mut data).expect("error reading file");
        for (index, value) in data.iter().enumerate() {
            self.memory[index + base as usize] = *value;
        }
    }

    pub fn load_boot_rom(&mut self, filename: &str) {
        let mut f = File::open(filename).expect("file not found");
        let mut data = Vec::new();
        f.read_to_end(&mut data).expect("error reading file");
        for (index, value) in data.iter().enumerate() {
            self.boot_rom[index] = *value;
        }
    }

    pub fn flag_interrupt(&mut self, irq:u8){
        self.write(0xff0f, irq | self.read(0xff0f));
    }
}

pub const FLAG_Z:u8 = 1<<7;
pub const FLAG_N:u8 = 1<<6;
pub const FLAG_H:u8 = 1<<5;
pub const FLAG_C:u8 = 1<<4;

pub struct Cpu {
    pub mmu: Mmu,
    pub a:u8, pub f:u8,
    pub b:u8, pub c:u8,
    pub d:u8, pub e:u8,
    pub h:u8, pub l:u8,
    pub sp: u16,
    pub pc: u16,
    pub ie: bool,
    pub hlt: bool,
}

pub fn word(h:u8, l:u8) -> u16 {
    (h as u16) << 8 | (l as u16)
}

#[derive(Copy, Clone, PartialEq)]
enum Immediate {
    None,
    Imm8(u8),
    Imm16(u16),
}

fn add(a:u8, b:u8, c:u8, cf_out:&mut bool, hf_out:&mut bool) -> u8 {
    *hf_out = ((a & 0x0F) + (b & 0x0F) + c) & 0xF0 != 0;
    let [h,l] = ((a as u16) + (b as u16) + (c as u16)).to_be_bytes();
    *cf_out = h != 0;
    l
}

fn add16(a:u16, b:u16, c:u8, cf_out:&mut bool, hf_out:&mut bool) -> u16 {
    let [ah, al] = a.to_be_bytes();
    let [bh, bl] = b.to_be_bytes();
    let rl = add(al, bl, c, cf_out, hf_out);
    let rh = add(ah, bh, if *cf_out {1} else {0}, cf_out, hf_out);
    word(rh, rl)
}

#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;

    #[test]
    fn test_add() {
        let mut c: bool = false;
        let mut h: bool = true;
        assert_eq!(add(0xa3, 0x12, 0, &mut c, &mut h), 0xb5);
        assert_eq!(c, false);
        assert_eq!(h, false);
        assert_eq!(add(0xa3, 0x12, 1, &mut c, &mut h), 0xb6);
        assert_eq!(c, false);
        assert_eq!(h, false);
        assert_eq!(add(0xa3, 0x1c, 0, &mut c, &mut h), 0xbf);
        assert_eq!(c, false);
        assert_eq!(h, false);
        assert_eq!(add(0xa3, 0x1c, 1, &mut c, &mut h), 0xc0);
        assert_eq!(c, false);
        assert_eq!(h, true);
        assert_eq!(add(0xa3, 0x1d, 0, &mut c, &mut h), 0xc0);
        assert_eq!(c, false);
        assert_eq!(h, true);
        assert_eq!(add(0xe3, 0x1c, 0, &mut c, &mut h), 0xff);
        assert_eq!(c, false);
        assert_eq!(h, false);
        assert_eq!(add(0xe3, 0x1c, 1, &mut c, &mut h), 0x00);
        assert_eq!(c, true);
        assert_eq!(h, true);
        assert_eq!(add(0x64, !1, 1, &mut c, &mut h), 0x63);
        assert_eq!(c, true);
        assert_eq!(h, true);
    }

    #[test]
    fn test_add16() {
        let mut c: bool = false;
        let mut h: bool = true;
        assert_eq!(add16(0xa300, 0x1200, 0, &mut c, &mut h), 0xb500);
        assert_eq!(c, false);
        assert_eq!(h, false);
        assert_eq!(add16(0xa300, 0x1200, 1, &mut c, &mut h), 0xb501);
        assert_eq!(c, false);
        assert_eq!(h, false);
        assert_eq!(add16(0xa300, 0x1c00, 0, &mut c, &mut h), 0xbf00);
        assert_eq!(c, false);
        assert_eq!(h, false);
        assert_eq!(add16(0xa300, 0x1cff, 1, &mut c, &mut h), 0xc000);
        assert_eq!(c, false);
        assert_eq!(h, true);
        assert_eq!(add16(0xa300, 0x1d00, 0, &mut c, &mut h), 0xc000);
        assert_eq!(c, false);
        assert_eq!(h, true);
        assert_eq!(add16(0xe300, 0x1c00, 0, &mut c, &mut h), 0xff00);
        assert_eq!(c, false);
        assert_eq!(h, false);
        assert_eq!(add16(0xe300, 0x1cff, 1, &mut c, &mut h), 0x0000);
        assert_eq!(c, true);
        assert_eq!(h, true);
    }


}

impl Cpu {
    fn fetch(&mut self) -> u8 {
        let val = self.mmu.read(self.pc);
        self.pc += 1;
        val
    }

    fn fetch_and_decode(&mut self) -> (Instruction, Immediate) {
        let mut instr = INSTRUCTIONS[self.fetch() as usize];
        if instr.operation == PREFIX {
            instr = INSTRUCTIONS[self.fetch() as usize + 0x100];
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
            DEC => add(d, !1, 1, &mut cf_out, &mut hf_out),
            INC => add(d, 1, 0, &mut cf_out, &mut hf_out),
            LD => s,
            OR => d | s,
            RES => d & !bit,
            RL => {cf_out = d & 0x80 != 0; (d << 1) | c_in},
            RLC => {cf_out = d & 0x80 != 0; (d << 1) | (if cf_out {1} else {0})},
            RR => {cf_out = d & 1 != 0; (d >> 1) | (c_in << 7)},
            RRC => {cf_out = d & 1 != 0; (d >> 1) | (if cf_out {0x80} else {0})},
            SBC => add(d, !s, c_in, &mut cf_out, &mut hf_out),
            SET => d | bit,
            SLA => {cf_out = d & 0x80 != 0; (d << 1)},
            SRA => {cf_out = d & 1 != 0; (d >> 1) | (d & 0x80)},
            SRL => {cf_out = d & 1 != 0; (d >> 1)},
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

        let mut hf_out = false;
        let mut cf_out = false;

        use OpData::*;
        let r = match op {
            ADD => add16(d, s, 0, &mut cf_out, &mut hf_out),
            DEC => add16(d, !s, 1, &mut cf_out, &mut hf_out),
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

    pub fn new(mmu:Mmu) -> Cpu {
        Cpu{mmu:mmu,
            a:0, f:0,
            b:0, c:0,
            d:0, e:0,
            h:0, l:0,
            sp:0,
            pc:0,
            ie:false,
            hlt:false,
        }
    }

    pub fn step(&mut self) -> isize {
        let (instr, imm) = self.fetch_and_decode();

        match instr.operation {
            DATA16 {op, dst, src, z, n, h, c, } => self.data16(op, dst, src, z, n, h, c, imm),
            DATA8 {op, dst, src, z, n, h, c, bit} => self.data8(op, dst, src, z, n, h, c, bit, imm),
            JUMP  {op, cond, rst_target} => if self.condition_satisfied(cond) {self.jump(op, rst_target, imm)},
            SPIMM8 {dst} => panic!("{} not implemented. {:?}", instr.mnemo, dst),
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

        let irq = self.mmu.read(0xffff) & self.mmu.read(0xff0f);
        if self.ie && (irq != 0) {
            self.ie = false;
            self.mmu.write(0xff0f, 0);
            let mut rst_target:u8 = 0;
            if irq & 0x01 != 0 {rst_target = 0x40;}
            else if irq & 0x02 != 0 {rst_target = 0x48;}
            else if irq & 0x04 != 0 {rst_target = 0x50;}
            else if irq & 0x08 != 0 {rst_target = 0x58;}
            else if irq & 0x10 != 0 {rst_target = 0x60;}
            self.jump(OpJump::RST, rst_target, Immediate::None);
            16 + instr.cycles as isize
        }
        else {
            instr.cycles as isize
        }
    }
}
