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
    candidates.into_iter().find(|c| c.exists())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut bus = CdiBus::new();
    let bios_path = find_file("bios/cdi220b.rom").expect("cdi220b.rom missing");
    bus.load_rom(&bios_path)?;
    if let Some(p) = find_file("bios/nvram_mk48t08.bin") {
        let _ = bus.nvram.load_from_file(&p);
    }
    let disc_path = find_file("giochi/Hotel Mario (USA).chd").expect("Mario missing");
    bus.load_cdrom(&disc_path)?;
    bus.set_video_mode(crate::mcd212::VideoMode::Ntsc);

    let mut cpu = CpuCore::new();
    cpu.set_cpu_type(CpuType::SCC68070);
    cpu.reset(&mut bus);
    bus.emulate_wait_states = false;

    let total_lines = bus.mcd212.video_mode.total_scanlines();
    let cyc_per_line = bus.mcd212.video_mode.cycles_per_scanline();

    let presses: Vec<(u64, i32, u64)> = vec![
        (4120, 1, 20), (4250, 1, 20), (4350, 1, 20), (4450, 1, 20),
        (4700, 1, 20), (4800, 1, 20), (4900, 1, 20), (5000, 1, 20),
        (5100, 1, 20), (5250, 1, 20), (5350, 1, 20), (5450, 1, 20),
        (5550, 1, 20),
    ];

    let mut pad_x = 384;
    let mut disc_launched = false;
    let mut shell_ready_count = 0;

    for frame in 0..5820 {
        if bus.cdrom.is_some() && !disc_launched {
            let (_b, shell_state) = (0x0000C000u32, bus.read_long(0x0000C000u32.wrapping_add(0xFFFF9172)));
            if shell_state >= 2 {
                shell_ready_count += 1;
                if shell_ready_count == 1 || shell_ready_count == 150 {
                    bus.slave.update_mouse(575, 335, 0);
                } else if shell_ready_count == 30 || shell_ready_count == 180 {
                    bus.slave.update_mouse(575, 335, 1);
                } else if shell_ready_count == 50 || shell_ready_count == 200 {
                    bus.slave.update_mouse(575, 335, 0);
                }
            }
            if cpu.pc >= 0x00010000 && cpu.pc < 0x00300000 && frame > 250 {
                disc_launched = true;
                bus.slave.update_mouse(384, 280, 0);
            }
        }

        if bus.slave.device_mouse_updated {
            pad_x = bus.slave.device_mouse_x;
            bus.slave.device_mouse_updated = false;
        }

        if disc_launched {
            let mut btn: u8 = 0;
            for &(f, b, len) in &presses {
                if frame >= f && frame < f + len { btn |= b as u8; }
            }
            if frame >= 5700 {
                pad_x = (pad_x + 8).clamp(0, 767);
            }
            bus.slave.update_mouse(pad_x, 280, btn);
        }

        bus.watch_addr1 = 0x25200C; // line 194, x=280
        bus.watch_addr2 = 0x25248C; // line 197, x=280
        bus.watch_enabled = frame >= 5793 && frame <= 5798;

        let mut cycle_debt = 0;
        for line in 0..total_lines {
            bus.tick_scanline(line);

            if bus.mcd212.vblank_irq {
                bus.scc_timer.lir |= 0x80;
                bus.mcd212.vblank_irq = false;
            }

            let int1_lvl = if (bus.scc_timer.lir & 0x80) != 0 {
                let lvl = (bus.scc_timer.lir >> 4) & 7;
                if lvl != 0 { lvl } else { 5 }
            } else { 0 };
            let timer_lvl = if bus.scc_timer.timer_irq { bus.scc_timer.picr1 & 7 } else { 0 };
            let cdic_lvl = if bus.cdic.irq_pending || bus.dvc.irq_pending() { 4 } else { 0 };
            let slave_lvl = if bus.slave.irq_pending { 2 } else { 0 };
            cpu.set_irq(int1_lvl.max(timer_lvl).max(cdic_lvl).max(slave_lvl));

            let mut line_cycles = cycle_debt;
            while line_cycles < cyc_per_line as i32 {
                let remaining = (cyc_per_line as i32 - line_cycles).max(1);
                let budget = remaining.min(32);
                bus.current_pc = cpu.pc;
                bus.current_line_cycle = line_cycles.max(0) as u32;
                let batch = cpu.run_for_cycles(&mut bus, budget);
                let mut batch_cycles = batch.cycles + bus.consume_wait_cycles();

                match batch.exit {
                    m68k::CycleBatchExit::TrapInstruction { trap_num } => {
                        batch_cycles += cpu.take_trap_exception(&mut bus, trap_num);
                    }
                    m68k::CycleBatchExit::AlineTrap { .. } => {
                        batch_cycles += cpu.take_aline_exception(&mut bus);
                    }
                    m68k::CycleBatchExit::FlineTrap { .. } => {
                        batch_cycles += cpu.take_fline_exception(&mut bus);
                    }
                    m68k::CycleBatchExit::IllegalInstruction { .. } => {
                        batch_cycles += cpu.take_illegal_exception(&mut bus);
                    }
                    m68k::CycleBatchExit::Breakpoint { .. } => {
                        batch_cycles += cpu.take_bkpt_exception(&mut bus);
                    }
                    m68k::CycleBatchExit::Stopped => {
                        let remaining_line = (cyc_per_line as i32 - line_cycles).max(0) as u32;
                        bus.tick_timer(remaining_line);
                        line_cycles = cyc_per_line as i32;
                        break;
                    }
                    _ => {}
                }

                line_cycles += batch_cycles;
                bus.tick_timer(batch_cycles as u32);



                let int1_lvl = if (bus.scc_timer.lir & 0x80) != 0 {
                    let lvl = (bus.scc_timer.lir >> 4) & 7;
                    if lvl != 0 { lvl } else { 5 }
                } else { 0 };
                let timer_lvl = if bus.scc_timer.timer_irq { bus.scc_timer.picr1 & 7 } else { 0 };
                let cdic_lvl = if bus.cdic.irq_pending || bus.dvc.irq_pending() { 4 } else { 0 };
                let slave_lvl = if bus.slave.irq_pending { 2 } else { 0 };
                cpu.set_irq(int1_lvl.max(timer_lvl).max(cdic_lvl).max(slave_lvl));
            }
            cycle_debt = (line_cycles - cyc_per_line as i32).clamp(0, 100);

            bus.render_scanline(line);
            bus.tick_slave(cyc_per_line);
            bus.tick_cdic(cyc_per_line);
            bus.tick_dvc(cyc_per_line);
        }



        if frame >= 5780 && frame <= 5800 {
            // Let's find Mario's red cap (0xFFF80000 or red palette) around y=420..445
            let width = mcd212::SCREEN_WIDTH;
            let mut mario_x = 0;
            let mut mario_y = 0;
            'find: for y in 425..440 {
                for x in 100..700 {
                    let p = bus.mcd212.framebuffer[y * width + x] & 0xFFFFFF;
                    // Red cap in Mario
                    let r = (p >> 16) & 0xFF;
                    let g = (p >> 8) & 0xFF;
                    let b = p & 0xFF;
                    if r > 180 && g < 40 && b < 40 {
                        mario_x = x;
                        mario_y = y;
                        break 'find;
                    }
                }
            }
            let is_odd = (bus.mcd212.csrr[0] & crate::mcd212::CSR1R_PA) == 0;
            println!("FRAME {}: field={} mario_head=({}, {}) FB[428,280]=0x{:06X}", frame, if is_odd { "ODD" } else { "EVEN" }, mario_x, mario_y, bus.mcd212.framebuffer[428 * width + 280] & 0xFFFFFF);
            // Print a 10x8 slice around Mario's nose (mario_x + 5, mario_y + 6)
            if mario_x > 0 {
                let nx = mario_x;
                let ny = mario_y;
                println!("  Mario full sprite (x={}..{}, y={}..{}):", nx - 4, nx + 20, ny, ny + 14);
                for y in ny..=(ny + 14) {
                    print!("    y={:3}: ", y);
                    for x in (nx - 4)..(nx + 20) {
                        let p = bus.mcd212.framebuffer[y * width + x] & 0xFFFFFF;
                        let r = (p >> 16) & 0xFF;
                        let g = (p >> 8) & 0xFF;
                        let b = p & 0xFF;
                        // Print character representation for visual clarity:
                        // 'R' for red (cap/shirt)
                        // 'N' for nose/skin (F77B7B, F7A496)
                        // 'B' for blue (overalls)
                        // 'W' for white (eyes/gloves)
                        // '.' for background (161108, 000000)
                        if p == 0x000000 || p == 0x161108 || p == 0x773729 || p == 0x5C300C || p == 0x28140C || p == 0x946448 {
                            print!(". ");
                        } else if r > 180 && g < 50 && b < 50 {
                            print!("R ");
                        } else if p == 0xF77B7B || p == 0xF7A496 || p == 0xF76D6D {
                            print!("N ");
                        } else if p == 0x007BF7 || p == 0x0056B5 {
                            print!("B ");
                        } else if p == 0xF7F7F7 || p == 0xEDEDED {
                            print!("W ");
                        } else {
                            print!("? ");
                        }
                    }
                    println!();
                }
            }
        }
    }

    Ok(())
}
