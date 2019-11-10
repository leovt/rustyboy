//mod ppu {
    pub const LCD_WIDTH:u32 = 160;
    pub const LCD_HEIGHT:u32 = 144;

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
        scroll_x: u8,
        scroll_y: u8,
        ly: u8,
        ly_compare: u8,
        dma: u8,
        bg_palette: u8,
        ob_palette_0: u8,
        ob_palette_1: u8,
        window_y: u8,
        window_x: u8,
    }

    pub struct OAMEntry {
        pos_x: u8,
        pos_y: u8,
        tile_no: u8,
        flags: u8,
    }

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
                 ob_tiles: &[Tile; 256]) -> ([u32;160], u32)
    {
        let mut pixels:[u32;160] = [0;160];
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


        (pixels, cycles)
    }
//}
