use std::path;

use crate::mmu::Mmu;
use crate::regs::*;

pub struct Cpu {
    regs: Registers,
    mode_flags: ModeChangeFlags,

    interrupts: bool,
    cycles: u8,

    mmu: Mmu,
}

struct ModeChangeFlags {
    change_intrpt_mode_on_next_instr: bool,
}

impl Cpu {
    pub fn new(path: &path::Path) -> Self {
        Cpu {
            regs: Registers::new(),
            mode_flags: ModeChangeFlags::new(),

            interrupts: true,
            cycles: 0,

            mmu: Mmu::new(path),
        }
    }

    pub fn run(&mut self) {
        loop {
            if self.cycles != 0 {
                self.cycles -= 1;
            }

            let instr = self.mmu.read(self.regs.PC);
            self.regs.inc_PC();
            self.cycles = self.exec_instr(instr);
        }
    }

    fn exec_instr(&mut self, instr: u8) -> u8 {
        let addr = self.regs.PC - 1;

        let cycles = match instr {
            //NOP
            0x00 => {
                println!(
                    "Addr:0x{:04X}\tOp:0x{:X}\tTime:{}\t{}",
                    addr, instr, 4, "NOP"
                );
                4
            }

            //DEC B
            0x05 => {
                self.regs.B = self.dec_u8(self.regs.B);

                println!(
                    "Addr:0x{:04X}\tOp:0x{:X}\tTime:{}\t{} {}",
                    addr, instr, 4, "DEC", "B"
                );
                4
            }

            //LD B u8
            0x06 => {
                let data = self.get_imu8();
                self.regs.B = data;

                println!(
                    "Addr:0x{:04X}\tOp:0x{:X}\tTime:{}\t{} {} 0x{:X}",
                    addr, instr, 8, "LD", "B,", data
                );
                8
            }

            //DEC D
            0x0D => {
                self.regs.C = self.dec_u8(self.regs.C);

                println!(
                    "Addr:0x{:04X}\tOp:0x{:X}\tTime:{}\t{} {}",
                    addr, instr, 4, "DEC", "C"
                );
                4
            }

            //LD C u8
            0x0E => {
                let data = self.get_imu8();
                self.regs.C = data;

                println!(
                    "Addr:0x{:04X}\tOp:0x{:X}\tTime:{}\t{} {} 0x{:X}",
                    addr, instr, 8, "LD", "C,", data
                );
                8
            }

            //JR NZ
            0x20 => {
                let jmp_addr = self.get_imu8();
                if !self.regs.get_flag(FlagsMasks::Z) {
                    self.regs.PC = jmp_addr as u16;
                }

                println!(
                    "Addr:0x{:04X}\tOp:0x{:X}\tTime:{}\t{} {} 0x{:04X}",
                    addr, instr, 8, "JR", "NZ", jmp_addr
                );
                8
            }

            //LD HL u16
            0x21 => {
                let data = self.get_imu16();
                self.regs.set_HL(data);

                println!(
                    "Addr:0x{:04X}\tOp:0x{:X}\tTime:{}\t{} {} 0x{:X}",
                    addr, instr, 12, "LD", "HL,", data
                );
                12
            }

            //LDD HL u16
            0x32 => {
                let addr = self.regs.HL();
                self.mmu.write(addr, self.regs.A);
                self.regs.set_HL(addr - 1);

                println!(
                    "Addr:0x{:04X}\tOp:0x{:X}\tTime:{}\t{} {:04X}, {}",
                    addr, instr, 8, "LDD", addr, "A"
                );
                8
            }

            //LD A imu8
            0x3E => {
                self.regs.A = self.get_imu8();

                println!(
                    "Addr:0x{:04X}\tOp:0x{:X}\tTime:{}\t{} {} 0x{:X}",
                    addr, instr, 8, "LD", "A,", self.regs.A
                );
                8
            }

            //XOR A
            0xAF => {
                self.xor(self.regs.A);

                println!(
                    "Addr:0x{:04X}\tOp:0x{:X}\tTime:{}\t{} {}",
                    addr, instr, 4, "XOR", "A"
                );
                4
            }

            //JMP u16
            0xC3 => {
                self.regs.PC = self.get_imu16();

                println!(
                    "Addr:0x{:04X}\tOp:0x{:X}\tTime:{}\t{} 0x{:04X}",
                    addr, instr, 16, "JMP", self.regs.PC
                );
                16
            }

            //RST 0x18
            0xDF => {
                self.rst(0x18);

                println!(
                    "Addr:{:04X}\tOp:0x{:X}\tTime:{}\t{} 0x{:X}",
                    addr, instr, 32, "RST", 0x18
                );
                32
            }

            //DI
            0xF3 => {
                //Mode flag set at end of func

                println!("Addr:{:04X}\tOp:0x{:X}\tTime:{}\t{}", addr, instr, 4, "DI");
                4
            }

            //RST 0x36
            0xFF => {
                self.rst(0x36);

                println!(
                    "Addr:{:04X}\tOp:{}\tTime:{}\t{} 0x{:X}",
                    addr, instr, 32, "RST", 0x36
                );
                32
            }

            _ => {
                self.panic_dump(instr);

                0
            }
        };

        if instr == 0xF3 {
            self.mode_flags.change_intrpt_mode_on_next_instr = true;
        } else if self.mode_flags.change_intrpt_mode_on_next_instr {
            self.interrupts = !self.interrupts;
        }

        cycles
    }

    fn stack_push(&mut self, data: u16) {
        self.mmu.write(self.regs.SP, (data >> 8) as u8);
        self.regs.SP -= 1;
        self.mmu.write(self.regs.SP, (data & 0x0F) as u8);
        self.regs.SP -= 1;
    }

    fn get_imu8(&mut self) -> u8 {
        let val: u8 = self.mmu.read(self.regs.PC);
        self.regs.inc_PC();
        val
    }

    fn get_imu16(&mut self) -> u16 {
        let mut val: u16 = self.get_imu8() as u16;
        val |= (self.get_imu8() as u16) << 8;
        val
    }

    fn panic_dump(&self, instr: u8) {
        println!();
        println!(
            "Addr: 0x{:04X}\t Opcode 0x{:02X} not implemented",
            self.regs.PC - 1,
            instr
        );
        println!("Register dump:");
        println!("-AF: 0x{:04X}", self.regs.AF());
        println!("-BC: 0x{:04X}", self.regs.BC());
        println!("-DE: 0x{:04X}", self.regs.DE());
        println!("-HL: 0x{:04X}", self.regs.HL());
        println!("-SP: 0x{:04X}", self.regs.SP);
        println!("-PC: 0x{:04X}", self.regs.PC);
        println!();
        panic!();
    }
}

//ALU
impl Cpu {
    fn xor(&mut self, reg: u8) {
        self.regs.A = reg ^ self.regs.A;

        if self.regs.A == 0 {
            self.regs.set_flag(FlagsMasks::Z, true);
        }
    }

    fn dec_u8(&mut self, reg: u8) -> u8 {
        let res = reg.overflowing_sub(1).0;

        self.regs.set_flag(FlagsMasks::N, true);
        if res == 0 {
            self.regs.set_flag(FlagsMasks::Z, true);
        }

        if res & 0xF0 == 0 {
            self.regs.set_flag(FlagsMasks::H, true);
        } else {
            self.regs.set_flag(FlagsMasks::H, false);
        }

        res
    }
}

//JMP
impl Cpu {
    fn rst(&mut self, offset: u8) {
        self.stack_push(self.regs.PC);
        self.regs.PC = 0x0000 + offset as u16;
    }
}

impl ModeChangeFlags {
    fn new() -> Self {
        ModeChangeFlags {
            change_intrpt_mode_on_next_instr: false,
        }
    }
}
