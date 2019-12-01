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

mod oam_flags {
    pub const PRIORITY:u8 = 0x80;
    pub const FLIP_Y:u8 = 0x40;
    pub const FLIP_X:u8 = 0x20;
    pub const PALETTE1:u8 = 0x10;
}

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

mod stat_flags {
    pub const LY_INTERRUPT:u8 = 0x40;
    pub const M2_INTERRUPT:u8 = 0x20;
    pub const M1_INTERRUPT:u8 = 0x10;
    pub const M0_INTERRUPT:u8 = 0x08;
    pub const LY_FLAG:u8 = 0x04;
    pub const MODE_MASK:u8 = 0x03;
}

pub struct PPURegister {
    // at adress FF40
    lcd_control_flags: u8,
    lcd_status_flags: u8,
    pub scroll_x: u8,
    pub scroll_y: u8,
    ly: u8,
    ly_compare: u8,
    dma: u8,
    bg_palette: u8,
    ob_palette_0: u8,
    ob_palette_1: u8,
    window_y: u8,
    window_x: u8,
}

#[derive(Copy, Clone)]
pub struct OAMEntry {
    pos_x: u8,
    pos_y: u8,
    tile_no: u8,
    flags: u8,
}

#[derive(Copy, Clone)]
pub struct Tile {
    data: [u8; 16],
}

fn obj_visible(oam: &OAMEntry, ppu: &PPURegister) -> bool {
    let h = if ppu.lcd_control_flags & ctrl_flags::OBJ_SIZE == 0 {8} else {16};

    oam.pos_x > 0 &&
    ppu.ly + 16 >= oam.pos_y &&
    ppu.ly + 16 < oam.pos_y + h
}

// will need to change for emulating horizontal timing
pub fn draw_line(ppu: &PPURegister,
             oam: &[OAMEntry; 40], // starting at adress FE00
             window_map: &[[u8; 32];32],
             bg_map: &[[u8; 32];32],
             bgw_tiles: &[Tile; 256],
             ob_tiles: &[Tile; 256]) -> ([u8;LCD_WIDTH], u32)
{
    let mut pixels:[u8;LCD_WIDTH] = [0;LCD_WIDTH];
    let mut cycles:u32 = 0;

    // oam search: 80 cycles
    let mut obj_no:usize = 0;
    let mut active_obj:[u8;10] = [0xff;10];
    for i in 0..39 {
        if obj_visible(&oam[i], ppu) {
            active_obj[obj_no] = i as u8;
            obj_no += 1;
            if obj_no >= 10 {
                break;
            }
        }
    }
    let obj_no = obj_no;
    let active_obj = active_obj;

    // pixel transfer
    let y_virt = (ppu.ly as usize + ppu.scroll_y as usize) % 256;
    let y_map = (y_virt / 8) as usize;
    let y_tile = (y_virt % 8 * 2) as usize;

    for x in 0..LCD_WIDTH {
        let x_virt = ((x + ppu.scroll_x as usize) % 256) as u8;
        let x_map = (x_virt / 8) as usize;
        let x_tile = 7 - x_virt % 8;
        let upper = bgw_tiles[bg_map[y_map][x_map] as usize].data[y_tile];
        let lower = bgw_tiles[bg_map[y_map][x_map] as usize].data[y_tile+1];

        let upper_bit = (upper & (1 << x_tile)) >> x_tile;
        let lower_bit = (lower & (1 << x_tile)) >> x_tile;

        pixels[x as usize] = 2*upper_bit + lower_bit;
    }

    (pixels, cycles)
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
                        ly += 1;
                        if ly >= 154 {
                            ly = 0;
                            self.mode = 2;
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
                        self.mode = if ly < 144 {2} else {0};
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

pub struct PpuStandalone {
    pub ppu: PPURegister,
    oam: [OAMEntry; 40],
    window_map: [[u8; 32];32],
    bg_map: [[u8; 32];32],
    bgw_tiles: [Tile; 256],
    ob_tiles: [Tile; 256]
}

const SAMPLE_MAP:[[u8;32];32] = [
[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,  0,0,0,0,0,0,0,0,0,0,0,0],
[0,1,0,1,0,1,0,1,0,1,0,1,0,1,0,1,0,1,0,0,  0,0,0,0,0,0,0,0,0,0,0,0],
[0,0,1,0,1,0,1,0,1,0,1,0,1,0,1,0,1,0,1,0,  0,0,0,0,0,0,0,0,0,0,0,0],
[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,  0,0,0,0,0,0,0,0,0,0,0,0],
[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,  0,0,0,0,0,0,0,0,0,0,0,0],
[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,  0,0,0,0,0,0,0,0,0,0,0,0],
[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,  0,0,0,0,0,0,0,0,0,0,0,0],
[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,  0,0,0,0,0,0,0,0,0,0,0,0],
[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,  0,0,0,0,0,0,0,0,0,0,0,0],
[0,0,1,1,1,1,1,1,0,0,1,0,0,0,0,0,1,1,0,0,  0,1,0,0,0,0,0,0,0,0,0,0],
[0,0,1,0,0,0,0,1,0,0,1,0,0,0,0,1,0,0,1,0,  1,1,0,0,0,0,0,0,0,0,0,0],
[0,0,1,0,0,0,0,1,0,0,1,0,0,0,0,1,0,0,1,0,  0,1,0,0,0,0,0,0,0,0,0,0],
[0,0,1,0,0,0,0,1,0,0,1,0,0,0,0,1,0,0,1,0,  1,1,0,0,0,0,0,0,0,0,0,0],
[0,0,1,0,0,0,0,1,0,0,1,0,0,0,0,0,1,1,0,0,  0,1,0,0,0,0,0,0,0,0,0,0],
[0,0,1,0,0,0,0,1,0,0,1,0,0,0,0,0,0,0,0,0,  0,0,0,0,0,0,0,0,0,0,0,0],
[0,0,1,0,0,0,0,1,0,0,1,0,0,0,0,0,1,1,1,1,  1,0,0,0,0,0,0,0,0,0,0,0],
[0,0,1,0,0,0,0,1,0,0,1,0,0,0,0,1,0,0,0,0,  0,1,0,0,0,0,0,0,0,0,0,0],
[0,0,1,1,1,1,1,1,0,0,1,0,0,0,0,0,1,1,1,1,  1,0,0,0,0,0,0,0,0,0,0,0],

[0,0,0,0,0,0,0,0,  0,0,0,0,0,0,0,0,  0,0,0,0,0,0,0,0,  0,0,0,0,0,0,0,0],
[0,0,0,0,0,0,0,0,  0,0,0,0,0,0,0,0,  0,0,0,0,0,0,0,0,  0,0,0,0,0,0,0,0],
[0,0,0,0,0,0,0,0,  0,0,0,0,0,0,0,0,  0,0,0,0,0,0,0,0,  0,0,0,0,0,0,0,0],
[0,0,0,0,0,0,0,0,  0,0,0,0,0,0,0,0,  0,0,0,0,0,0,0,0,  0,0,0,0,0,0,0,0],
[0,0,0,0,0,0,0,0,  0,0,0,0,0,0,0,0,  0,0,0,1,0,0,0,0,  0,0,0,0,0,0,0,0],
[0,0,0,0,0,0,0,0,  0,0,0,0,0,0,0,0,  1,0,0,1,0,0,1,0,  0,0,0,0,0,0,0,0],
[0,0,0,0,0,0,0,0,  0,0,0,0,0,0,0,0,  0,1,0,1,0,1,0,0,  0,0,0,0,0,0,0,0],
[0,0,0,0,0,0,0,0,  0,0,0,0,0,0,0,0,  0,0,0,1,0,0,0,0,  0,0,0,0,0,0,0,0],
[0,0,0,0,0,0,0,0,  0,0,0,0,0,0,0,1,  1,1,1,1,1,1,1,1,  0,0,0,0,0,0,0,0],
[0,0,0,0,0,0,0,0,  0,0,0,0,0,0,0,0,  0,0,0,1,0,0,0,0,  0,0,0,0,0,0,0,0],
[0,0,0,0,0,0,0,0,  0,0,0,0,0,0,0,0,  0,1,0,1,0,1,0,0,  0,0,0,0,0,0,0,0],
[0,0,0,0,0,0,0,0,  0,0,0,0,0,0,0,0,  1,0,0,1,0,0,1,0,  0,0,0,0,0,0,0,0],
[0,0,0,0,0,0,0,0,  0,0,0,0,0,0,0,0,  0,0,0,1,0,0,0,0,  0,0,0,0,0,0,0,0],
[0,0,0,0,0,0,0,0,  0,0,0,0,0,0,0,0,  0,0,0,0,0,0,0,0,  0,0,0,0,0,0,0,0]];

const TILE_0:Tile = Tile {
    data: [0u8;16]
};

const TILE_1:Tile = Tile {
    data: [0xff, 0xff, 0x81, 0xff, 0x91, 0xef, 0x91, 0xef, 0x91, 0xef, 0x9d, 0xe3, 0x81, 0xff, 0xff, 0xff]
};

impl PpuStandalone {
    pub fn new() -> PpuStandalone {
        let mut p = PpuStandalone {
            ppu: PPURegister {
                lcd_control_flags: ctrl_flags::DISPLAY_ENABLE | ctrl_flags::BG_ENABLE,
                lcd_status_flags: 0,
                scroll_x: 0, scroll_y: 0,
                ly: 0, ly_compare: 255,
                dma: 0,
                bg_palette: 0b11100100,
                ob_palette_0: 0b11100100,
                ob_palette_1: 0b11100100,
                window_y: 0,
                window_x: 0,
            },
            oam: [OAMEntry{pos_x:0, pos_y:0, tile_no:0, flags:0};40],
            window_map: [[0;32];32],
            bg_map: [[0;32];32],
            bgw_tiles: [Tile{data:[0;16]};256],
            ob_tiles: [Tile{data:[0;16]};256],
        };
        p.bg_map = SAMPLE_MAP;
        p.bgw_tiles[0] = TILE_0;
        p.bgw_tiles[1] = TILE_1;
        p
    }

    pub fn draw_frame(&mut self, lcd: &mut ImageBuffer<Rgba<u8>, Vec<u8>>, palette: &[Rgba<u8>;4]) {
        for y in 0..LCD_HEIGHT {
            self.ppu.ly = y as u8;
            let pixels = draw_line(&self.ppu, &self.oam, &self.window_map, &self.bg_map, &self.bgw_tiles, &self.ob_tiles).0;
            for (x, p) in pixels.iter().enumerate() {
                lcd.put_pixel(x as u32, y as u32, palette[*p as usize]);
            }
        }
    }
}
