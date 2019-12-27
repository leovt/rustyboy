extern crate image as im;
use im::{ImageBuffer, Rgba};

use crate::cpu::{Mmu};

pub const LCD_WIDTH:usize = 160;
pub const LCD_HEIGHT:usize = 144;

const LCD_PALETTE:[im::Rgba<u8>;4] = [
    im::Rgba([198,227,195,255]),
    im::Rgba([157,181,154,255]),
    im::Rgba([110,128,8,255]),
    im::Rgba([53,61,52,255]),
];

#[allow(unused)]
mod oam_flags {
    pub const PRIORITY:u8 = 0x80;
    pub const FLIP_Y:u8 = 0x40;
    pub const FLIP_X:u8 = 0x20;
    pub const PALETTE1:u8 = 0x10;
}

#[allow(unused)]
mod ctrl_flags {
    pub const DISPLAY_ENABLE:u8 = 0x80;
    pub const WINDOW_TMA:u8 = 0x40;
    pub const WINDOW_ENABLE:u8 = 0x20;
    pub const BGW_TILE_DATA:u8 = 0x10;
    pub const BG_TMA:u8 = 0x08;
    pub const OBJ_SIZE:u8 = 0x04;
    pub const OBJ_ENABLE:u8 = 0x02;
    pub const BG_ENABLE:u8 = 0x01;
}

#[allow(unused)]
mod stat_flags {
    pub const LY_INTERRUPT:u8 = 0x40;
    pub const M2_INTERRUPT:u8 = 0x20;
    pub const M1_INTERRUPT:u8 = 0x10;
    pub const M0_INTERRUPT:u8 = 0x08;
    pub const LY_FLAG:u8 = 0x04;
    pub const MODE_MASK:u8 = 0x03;
}

pub struct Ppu {
    pub cycles_left: isize,
    pub x: u8,
    pub mode: u8,
    pub cycles_left_current_line: isize,
}

impl Ppu {
    pub fn new() -> Ppu {
        Ppu {
            cycles_left: 0,
            x: 0,
            mode: 0,
            cycles_left_current_line: 0,
        }
    }

    pub fn run_for(&mut self, mmu: &mut Mmu, lcd: &mut ImageBuffer<Rgba<u8>, Vec<u8>>, cycles:isize) {
        self.cycles_left += cycles;

        let scroll_y = mmu.read(0xff42);
        let scroll_x = mmu.read(0xff43);
        let mut ly = mmu.read(0xff44);

        let control = mmu.read(0xff40);
        let bgw_tiles = if control & ctrl_flags::BGW_TILE_DATA != 0 {0x8000} else {0x8800};
        let bg_map = if control & ctrl_flags::BG_TMA == 0 {0x9800} else {0x9c00};

        while self.cycles_left > 0 {
            match self.mode {
                // vblank: 10 lines
                0 => {
                    if self.cycles_left >= 456 {
                        self.cycles_left -= 456;
                        if ly >= 153 {
                            ly = 0;
                            self.mode = 2;
                        } else {
                            ly += 1;
                        }
                    } else {
                        break;
                    }
                },
                // hblank:
                1 => {
                    if self.cycles_left >= self.cycles_left_current_line {
                        self.cycles_left -= self.cycles_left_current_line;
                        self.cycles_left_current_line = 0;
                        ly += 1;
                        if ly < 144 {
                            self.mode = 2;
                        } else {
                            self.mode = 0;
                            mmu.flag_interrupt(0x01);
                        }
                    } else {
                        break;
                    }
                },
                // oam_search
                2 => {
                    if self.cycles_left >= 80 {
                        self.cycles_left -= 80;
                        self.cycles_left_current_line = 456-80;
                        self.x = 0;
                        // todo: actually do the oam search
                        self.mode = 3;
                    } else {
                        break;
                    }
                },
                // drawing
                3 => {
                    let y_virt = (ly as usize + scroll_y as usize) % 256;
                    let y_map = (y_virt / 8) as u16;
                    let y_tile = (y_virt % 8 * 2) as u16;

                    let x_virt = ((self.x as usize + scroll_x as usize) % 256) as u8;
                    let x_map = (x_virt / 8) as u16;
                    let x_tile = 7 - x_virt % 8;

                    let tile_no = mmu.read(bg_map + 32*y_map + x_map) as u16;

                    let upper = mmu.read(bgw_tiles + tile_no * 16 + y_tile);
                    let lower = mmu.read(bgw_tiles + tile_no * 16 + y_tile + 1);
                    let upper_bit = (upper & (1 << x_tile)) >> x_tile;
                    let lower_bit = (lower & (1 << x_tile)) >> x_tile;

                    lcd.put_pixel(self.x as u32, ly as u32, LCD_PALETTE[(2*upper_bit + lower_bit) as usize]);

                    self.x += 1;
                    if self.x >= 160 {
                        self.mode = 1;
                    }
                    self.cycles_left -= 1;
                    self.cycles_left_current_line -= 1;

                }
                _ => panic!("mode illegal")
            }
        } // wend

        mmu.write(0xff41, self.mode);
        mmu.write(0xff44, ly);
    }


//            0xff40 => lcd_control_flags
//            0xff41 => lcd_status_flags
//            0xff42 => scroll_y
//            0xff43 => scroll_x
//            0xff44 => ly
//            0xff45 => ly_compare
//            0xff46 => dma
//            0xff47 => bg_palette
//            0xff48 => ob_palette_0
//            0xff49 => ob_palette_1
//            0xff4a => window_x
//            0xff4b => window_y
}
