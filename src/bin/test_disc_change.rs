#[path = "../audio.rs"]
mod audio;
#[path = "../bus.rs"]
mod bus;
#[path = "../cdic.rs"]
mod cdic;
#[path = "../cdrom.rs"]
mod cdrom;
#[path = "../dma.rs"]
mod dma;
#[path = "../dvc.rs"]
mod dvc;
#[path = "../mcd212.rs"]
mod mcd212;
#[path = "../nvram.rs"]
mod nvram;
#[path = "../scc_timer.rs"]
mod scc_timer;
#[path = "../slave.rs"]
mod slave;

use std::path::{Path, PathBuf};
use m68k::{CpuCore, CpuType, AddressBus};
use crate::bus::CdiBus;
use crate::mcd212::{TOTAL_SCANLINES, CYCLES_PER_SCANLINE};

fn find_file(relative_subpath: &str) -> Option<PathBuf> {
    let candidates = [
        Path::new(relative_subpath).to_path_buf(),
        Path::new("..").join(relative_subpath),
        Path::new("file utili").join(relative_subpath),
        Path::new("..").join("file utili").join(relative_subpath),
    ];
    for c in &candidates {
        if c.exists() {
            return Some(c.clone());
        }
    }
    None
}

fn run_frames(cpu: &mut CpuCore, bus: &mut CdiBus, count: usize) {
    for _ in 0..count {
        for _line in 0..TOTAL_SCANLINES {
            if bus.scc_timer.timer_irq {
                cpu.set_irq(6);
            } else if bus.cdic.irq_pending {
                cpu.set_irq(4);
            } else if bus.slave.irq_pending {
                cpu.set_irq(2);
            } else if bus.mcd212.vblank_irq {
                cpu.set_irq(1);
            } else {
                cpu.set_irq(0);
            }

            let mut line_cycles = 0;
            while line_cycles < CYCLES_PER_SCANLINE as i32 {
                let budget = (CYCLES_PER_SCANLINE as i32 - line_cycles).max(1);
                bus.current_pc = cpu.pc;
                let batch = cpu.run_for_cycles(bus, budget);
                line_cycles += batch.cycles;

                match batch.exit {
                    m68k::CycleBatchExit::TrapInstruction { trap_num } => {
                        let trap_cyc = cpu.take_trap_exception(bus, trap_num);
                        line_cycles += trap_cyc;
                    }
                    m68k::CycleBatchExit::AlineTrap { .. } => {
                        let trap_cyc = cpu.take_aline_exception(bus);
                        line_cycles += trap_cyc;
                    }
                    m68k::CycleBatchExit::FlineTrap { .. } => {
                        let trap_cyc = cpu.take_fline_exception(bus);
                        line_cycles += trap_cyc;
                    }
                    m68k::CycleBatchExit::IllegalInstruction { .. } => {
                        let trap_cyc = cpu.take_illegal_exception(bus);
                        line_cycles += trap_cyc;
                    }
                    m68k::CycleBatchExit::Breakpoint { .. } => {
                        let trap_cyc = cpu.take_bkpt_exception(bus);
                        line_cycles += trap_cyc;
                    }
                    m68k::CycleBatchExit::Stopped => {
                        line_cycles = CYCLES_PER_SCANLINE as i32;
                        break;
                    }
                    _ => {}
                }
            }

            let spent = (line_cycles as u32).max(CYCLES_PER_SCANLINE);
            bus.mcd212.tick_cycles(spent);
            bus.tick_cdic(spent);
            bus.tick_timer(spent);
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Test Hot Disc Change & System Reset ===");

    let mut bus = CdiBus::new();
    let bios_path = find_file("bios/cdi220b.rom").expect("BIOS not found");
    bus.load_rom(&bios_path)?;

    // 1. Initial boot with Hotel Mario
    let mario_path = find_file("giochi/Hotel Mario (USA).chd").expect("Hotel Mario not found");
    bus.load_cdrom(&mario_path)?;

    let mut cpu = CpuCore::new();
    cpu.set_cpu_type(CpuType::SCC68070);
    cpu.reset(&mut bus);

    let initial_pc = cpu.pc;
    println!("1. Booting Hotel Mario... Initial PC = 0x{:08X}", initial_pc);
    for frame in 1..=1200 {
        if frame == 865 {
            let play_a6 = 0x0000C000u32;
            bus.write_long(play_a6.wrapping_add(0xFFFFC364), 0x00000500);
            bus.write_long(play_a6.wrapping_add(0xFFFF9172), 2);
            bus.write_long(play_a6.wrapping_add(0xFFFF91A6), 1);
            bus.write_long(play_a6.wrapping_add(0xFFFFC344), 29);
        }
        run_frames(&mut cpu, &mut bus, 1);
    }
    println!("   After 1200 frames of Mario: PC = 0x{:08X}, CDIC cmd=0x{:02X}", cpu.pc, bus.cdic.command);

    // 2. Change disc to Dragon's Lair!
    let dragons_path = find_file("giochi/Dragon's Lair (USA).chd").expect("Dragon's Lair not found");
    println!("2. Changing disc to Dragon's Lair: {:?}", dragons_path);
    bus.load_cdrom(&dragons_path)?;
    bus.reset_system();
    cpu.reset(&mut bus);
    println!("   Reset PC = 0x{:08X}", cpu.pc);
    assert_eq!(cpu.pc, initial_pc, "Reset PC must match initial ROM reset vector");

    for frame in 1..=1000 {
        run_frames(&mut cpu, &mut bus, 1);
        if frame % 200 == 0 {
            let play_a6 = 0x0000C000u32;
            let state_9172 = bus.read_long(play_a6.wrapping_add(0xFFFF9172));
            println!("   [DL Frame {}] PC=0x{:08X}, state_9172={}, CDIC cmd=0x{:02X}, lba={}",
                frame, cpu.pc, state_9172, bus.cdic.command, bus.cdic.curr_lba);
        }
    }

    println!("SUCCESS! Disc change and system reset tested!");
    Ok(())
}
