use std::env;
use std::fs;
use m68k::AddressBus;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        println!("Usage: disasm_tool <hex_addr> <num_instructions>");
        return;
    }
    let addr = u32::from_str_radix(args[1].trim_start_matches("0x"), 16).expect("invalid addr");
    let count: usize = args[2].parse().expect("invalid count");

    let rom = fs::read("../file utili/bios/cdi220b.rom").expect("read rom");
    let base = 0x400000u32;

    struct RomBus<'a> {
        rom: &'a [u8],
        base: u32,
    }
    impl<'a> m68k::AddressBus for RomBus<'a> {
        fn read_byte(&mut self, addr: u32) -> u8 {
            if addr >= self.base && (addr as usize - self.base as usize) < self.rom.len() {
                self.rom[(addr - self.base) as usize]
            } else {
                0
            }
        }
        fn read_word(&mut self, addr: u32) -> u16 {
            let b0 = self.read_byte(addr) as u16;
            let b1 = self.read_byte(addr + 1) as u16;
            (b0 << 8) | b1
        }
        fn read_long(&mut self, addr: u32) -> u32 {
            let w0 = self.read_word(addr) as u32;
            let w1 = self.read_word(addr + 2) as u32;
            (w0 << 16) | w1
        }
        fn write_byte(&mut self, _addr: u32, _val: u8) {}
        fn write_word(&mut self, _addr: u32, _val: u16) {}
        fn write_long(&mut self, _addr: u32, _val: u32) {}
    }

    let mut bus = RomBus { rom: &rom, base };
    let mut pc = addr;
    for _ in 0..count {
        let op = bus.read_word(pc);
        let (dis, len) = m68k::dasm::disassemble(pc, op, m68k::CpuType::SCC68070);
        let mut bytes_str = String::new();
        for i in 0..len {
            bytes_str.push_str(&format!("{:02X} ", bus.read_byte(pc + i as u32)));
        }
        println!("{:06X}: {:<16} {}", pc, bytes_str, dis);
        pc += len as u32;
    }
}
