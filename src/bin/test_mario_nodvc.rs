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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut bus = CdiBus::new();
    let bios_path = find_file("bios/cdi220b.rom").expect("cdi220b.rom missing");
    bus.load_rom(&bios_path)?;

    if let Some(nvram_path) = find_file("bios/nvram_mk48t08.bin") {
        let _ = bus.nvram.load_from_file(&nvram_path);
    }

    let disc_path = find_file("giochi/Hotel Mario (USA).chd").expect("Mario missing");
    bus.load_cdrom(&disc_path)?;

    bus.set_video_mode(crate::mcd212::VideoMode::Ntsc);

    let mut cpu = CpuCore::new();
    cpu.set_cpu_type(CpuType::SCC68070);
    cpu.reset(&mut bus);

    let total_lines = bus.mcd212.video_mode.total_scanlines();
    let cyc_per_line = bus.mcd212.video_mode.cycles_per_scanline();

    for frame in 1..=650 {
        let mut cycle_debt: i32 = 0;
        for line in 0..total_lines {
            bus.mcd212.tick_scanline(line);

            if bus.mcd212.vblank_irq {
                bus.scc_timer.lir |= 0x80;
            }
            let int1_lvl = if (bus.scc_timer.lir & 0x80) != 0 {
                let lvl = (bus.scc_timer.lir >> 4) & 7;
                if lvl != 0 { lvl } else { 5 }
            } else { 0 };
            let timer_lvl = if bus.scc_timer.timer_irq {
                let lvl = bus.scc_timer.picr1 & 7;
                if lvl != 0 { lvl } else { 6 }
            } else { 0 };
            let cdic_lvl = if bus.cdic.irq_pending || bus.dvc.irq_pending() { 4 } else { 0 };
            let slave_lvl = if bus.slave.irq_pending { 2 } else { 0 };
            let uart_lvl = if bus.scc_uart_tx_irq {
                let lvl = bus.scc_timer.picr2 & 7;
                if lvl != 0 { lvl } else { 5 }
            } else { 0 };
            let highest_ipl = int1_lvl.max(timer_lvl).max(cdic_lvl).max(slave_lvl).max(uart_lvl);
            cpu.set_irq(highest_ipl);

            let mut line_cycles = -cycle_debt;
            while line_cycles < cyc_per_line as i32 {
                let remaining = (cyc_per_line as i32 - line_cycles).max(1);
                let budget = remaining.min(32);
                let old_pc = cpu.pc;
                let old_sp = cpu.a(7);

                let batch = cpu.run_for_cycles(&mut bus, budget);
                line_cycles += batch.cycles;

                if frame >= 540 && frame <= 570 {
                    if cpu.pc < 0x1000 || cpu.a(7) > 0xE0000000 || cpu.a(7) < 0x1000 {
                        println!("[F{} L{}] PC: 0x{:08X}->0x{:08X}, SP: 0x{:08X}->0x{:08X}, IPL={}",
                            frame, line, old_pc, cpu.pc, old_sp, cpu.a(7), highest_ipl);
                    }
                }

                match batch.exit {
                    m68k::CycleBatchExit::TrapInstruction { trap_num } => {
                        let trap_cyc = cpu.take_trap_exception(&mut bus, trap_num);
                        line_cycles += trap_cyc;
                    }
                    m68k::CycleBatchExit::AlineTrap { .. } => {
                        let trap_cyc = cpu.take_aline_exception(&mut bus);
                        line_cycles += trap_cyc;
                    }
                    m68k::CycleBatchExit::FlineTrap { .. } => {
                        let trap_cyc = cpu.take_fline_exception(&mut bus);
                        line_cycles += trap_cyc;
                    }
                    m68k::CycleBatchExit::IllegalInstruction { .. } => {
                        let trap_cyc = cpu.take_illegal_exception(&mut bus);
                        line_cycles += trap_cyc;
                    }
                    m68k::CycleBatchExit::Breakpoint { .. } => {
                        let trap_cyc = cpu.take_bkpt_exception(&mut bus);
                        line_cycles += trap_cyc;
                    }
                    _ => {}
                }
            }
            cycle_debt = (line_cycles - cyc_per_line as i32).clamp(0, 100);

            bus.tick_slave(cyc_per_line);
            bus.tick_cdic(cyc_per_line);
            bus.tick_dvc(cyc_per_line);
            bus.tick_timer(cyc_per_line);
        }
    }
    Ok(())
}
