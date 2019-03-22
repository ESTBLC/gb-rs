use crate::lcd::Lcd;

pub struct Ppu {
    lcd: Lcd,
    vram: Vec<u8>,
    frame: Vec<u32>,

    cycles: u8,

    pub lcdc_control: u8,
    lcdc_status: u8,
    pub scy: u8,
    // SCX: u8,
    pub ly: u8,
    // LYC: u8,
    // WY: u8,
    // WX: u8,
    pub bg_colorpalette: u8,
}

enum LCDModes {
    OAM = 2,
    TRANSFER = 3,
    HBLANK = 0,
    VBLANK = 1,
}

enum Colors {
    BLACK = 0x00000000,
    WHITE = 0xFFFFFFFF,
}

const VIEWPORT_SIZE_X: usize = 160;
const VIEWPORT_SIZE_Y: usize = 144;

const TILE_SIZE: u16 = 0x10;
const BG_LINE_SIZE: u16 = 0x20;

const VRAM_SIZE: usize = 0x9FFF - 0x8000 + 1;

//Memory management
impl Ppu {
    pub fn new() -> Self {
        Ppu {
            lcd: Lcd::new(),
            vram: vec![0; VRAM_SIZE],
            frame: vec![0],
            cycles: 0,

            lcdc_control: 0,
            lcdc_status: 0 + LCDModes::OAM as u8,
            scy: 0,
            ly: 0,

            bg_colorpalette: 0,
        }
    }

    pub fn do_cycle(&mut self) {
        if !self.lcd_ison() {
            return;
        }

        while self.cycles != 0 {
            self.cycles -= 1;
            return;
        }

        let mode = self.get_mode();
        self.cycles += match mode {
            LCDModes::HBLANK => self.do_hblank(),
            LCDModes::VBLANK => self.do_vblank(),
            LCDModes::OAM => self.do_oam(),
            LCDModes::TRANSFER => self.do_transfer(),
        }
    }

    fn do_hblank(&self) -> u8 {
        1
    }

    fn do_vblank(&mut self) -> u8 {
        self.lcd.update(&self.frame);
        self.set_mode(LCDModes::OAM);
        1
    }

    fn do_oam(&mut self) -> u8 {
        self.set_mode(LCDModes::TRANSFER);
        1
    }

    fn do_transfer(&mut self) -> u8 {
        self.frame_gen();
        self.set_mode(LCDModes::VBLANK); // Should be HBLANK but for now, no HBLANK
        1
    }

    // pub fn read(&self, addr: u16) -> u8 {
    //     self.vram[addr as usize]
    // }

    pub fn write(&mut self, addr: u16, data: u8) {
        self.vram[addr as usize] = data;
    }

    fn get_mode(&self) -> LCDModes {
        match self.lcdc_status & 0b00000011 {
            0 => LCDModes::HBLANK,
            1 => LCDModes::VBLANK,
            2 => LCDModes::OAM,
            3 => LCDModes::TRANSFER,
            _ => panic!("Impossible case"),
        }
    }

    fn set_mode(&mut self, mode: LCDModes) {
        self.lcdc_status &= 0b11111100;
        self.lcdc_status += mode as u8;
    }

    fn lcd_ison(&self) -> bool {
        self.lcdc_control & 0b10000000 > 0
    }
}

//Frame creation
impl Ppu {
    fn frame_gen(&mut self) {
        self.frame = Vec::new();

        self.ly = 0;

        while self.ly < VIEWPORT_SIZE_Y as u8 {
            let mut pixs_line = self.get_line_pixs(self.ly);
            self.frame.append(&mut pixs_line);

            self.ly += 1;
        }

        //Should be in V-Blank
    }

    fn get_line_pixs(&self, line: u8) -> Vec<u32> {
        let mut pixs = Vec::with_capacity(VIEWPORT_SIZE_X);

        let bg_y = (self.scy + line) / 8;
        for x in 0..VIEWPORT_SIZE_X {
            let col = x as u8; //TODO add SCX
            let bg_x = col / 8;

            let tile_nb = self.bg_map_get_tile_number(bg_x, bg_y);
            pixs.push(self.tile_get_pix(tile_nb, col % 8, line % 8));
        }

        pixs
    }

    fn tile_get_pix(&self, num: u8, x: u8, y: u8) -> u32 {
        let addr = self.tile_addr(num);
        let i = addr + y as u16 * 2;
        let lsb = self.vram[i as usize];
        let msb = self.vram[(i + 1) as usize];

        let msb_color = ((msb & (1 << 7 - x) > 0) as u8) << 1;
        let lsb_color = (lsb & (1 << 7 - x) > 0) as u8;
        let color_num = msb_color + lsb_color;

        self.pix_find_color(color_num) as u32
    }

    fn tile_addr(&self, num: u8) -> u16 {
        num as u16 * TILE_SIZE
    }

    fn pix_find_color(&self, color: u8) -> Colors {
        match color {
            0b00 => Colors::WHITE,
            _ => Colors::BLACK,
        }
    }

    fn bg_map_get_tile_number(&self, x: u8, y: u8) -> u8 {
        let index = 0x9800 - 0x8000 + (y as u16 * BG_LINE_SIZE + x as u16);
        self.vram[index as usize]
    }
}

//Debug
#[allow(dead_code)]
impl Ppu {
    pub fn tile_print(&self, num: u8) {
        for j in 0..8 {
            for i in 0..8 {
                let pix_color = self.tile_get_pix(num, i, j);
                if pix_color == Colors::BLACK as u32 {
                    print!("1");
                } else {
                    print!(" ");
                }
            }

            println!("");
        }
    }

    pub fn frame_print(&mut self) {
        self.frame_gen();

        for j in 0..VIEWPORT_SIZE_Y {
            for x in 0..VIEWPORT_SIZE_X {
                if self.frame[(j * VIEWPORT_SIZE_X + x) as usize] == 0 {
                    print!("1");
                } else {
                    print!(" ");
                }
            }

            println!();
        }
    }
}
