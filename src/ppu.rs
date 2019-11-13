extern crate image;
use image::{ImageBuffer, Rgba};

pub const LCD_WIDTH:usize = 160;
pub const LCD_HEIGHT:usize = 144;

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
