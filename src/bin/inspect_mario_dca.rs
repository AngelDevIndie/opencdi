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
    bus.emulate_wait_states = true;

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

    std::fs::create_dir_all("traces").ok();
    for frame in 0..5855 {
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

        let mut cycle_debt = 0;
        for line in 0..total_lines {
            bus.tick_scanline(line);

            if frame == 5700 && line == 0 {
                let dcp0 = (bus.mcd212.get_dcp::<0>() & 0x0007_FFFC) as usize;
                let dcp1 = (bus.mcd212.get_dcp::<1>() & 0x0007_FFFC) as usize;
                println!("=== DCA TABLE DUMP Plane A (DCP0=0x{:06X}) ===", dcp0);
                for row in 0..245 {
                    let r_addr = dcp0 + row * 64;
                    let mut instrs = Vec::new();
                    for off in (r_addr..r_addr + 64).step_by(4) {
                        if off + 4 <= bus.mcd212.plane_a.len() {
                            let cmd = u32::from_be_bytes([
                                bus.mcd212.plane_a[off],
                                bus.mcd212.plane_a[off + 1],
                                bus.mcd212.plane_a[off + 2],
                                bus.mcd212.plane_a[off + 3],
                            ]);
                            let op = (cmd >> 24) as u8;
                            if op != 0x10 { instrs.push((off - r_addr, cmd, op)); }
                            if op <= 0x0F || op == 0x30 || op == 0x50 { break; }
                        }
                    }
                    if !instrs.is_empty() {
                        print!("  Row {:3} (0x{:06X}): ", row, r_addr);
                        for (off, cmd, op) in instrs { print!("[+{:02X}: 0x{:08X} op=0x{:02X}] ", off, cmd, op); }
                        println!();
                    }
                }
                println!("=== DCA TABLE DUMP Plane B (DCP1=0x{:06X}) ===", dcp1);
                for row in 0..245 {
                    let r_addr = dcp1 + row * 64;
                    let mut instrs = Vec::new();
                    for off in (r_addr..r_addr + 64).step_by(4) {
                        if off + 4 <= bus.mcd212.plane_b.len() {
                            let cmd = u32::from_be_bytes([
                                bus.mcd212.plane_b[off],
                                bus.mcd212.plane_b[off + 1],
                                bus.mcd212.plane_b[off + 2],
                                bus.mcd212.plane_b[off + 3],
                            ]);
                            let op = (cmd >> 24) as u8;
                            if op != 0x10 { instrs.push((off - r_addr, cmd, op)); }
                            if op <= 0x0F || op == 0x30 || op == 0x50 { break; }
                        }
                    }
                    if !instrs.is_empty() {
                        print!("  Row {:3} (0x{:06X}): ", row, r_addr);
                        for (off, cmd, op) in instrs { print!("[+{:02X}: 0x{:08X} op=0x{:02X}] ", off, cmd, op); }
                        println!();
                    }
                }
            }

            if bus.mcd212.vblank_irq {
                if frame >= 5700 && frame <= 5702 {
                    println!("[F{} L{}] vblank_irq ASSERTED! csrr[1]=0x{:02X} dca0=0x{:06X} dca1=0x{:06X}",
                        frame, line, bus.mcd212.csrr[1], bus.mcd212.dca[0], bus.mcd212.dca[1]);
                }
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

            let start_pc = cpu.pc;
            let mut line_cycles = cycle_debt;
            while line_cycles < cyc_per_line as i32 {
                let remaining = (cyc_per_line as i32 - line_cycles).max(1);
                let budget = remaining.min(32);
                let batch = cpu.run_for_cycles(&mut bus, budget);
                let mut batch_cycles = batch.cycles + bus.consume_wait_cycles();

                if frame == 5700 && line >= 211 && line <= 225 {
                    if bus.mcd212.plane_a[0x072946] != 0 {
                        // check if something modified plane_a
                    }
                }

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

        if frame >= 5700 && frame <= 5702 {
            println!("FRAME {}: DCR=[0x{:04X}, 0x{:04X}] DCP=[0x{:06X}, 0x{:06X}] VSR=[0x{:06X}, 0x{:06X}] cursor_ctrl=0x{:08X} cursor_pos=0x{:08X}",
                frame, bus.mcd212.dcr[0], bus.mcd212.dcr[1],
                bus.mcd212.dcp[0], bus.mcd212.dcp[1],
                bus.mcd212.get_vsr::<0>(), bus.mcd212.get_vsr::<1>(),
                bus.mcd212.cursor_control, bus.mcd212.cursor_position);
        }

        if frame >= 5700 && frame <= 5710 {
            let width = mcd212::SCREEN_WIDTH;
            // Let's sample Mario's nose row y=434 and hat row y=428
            let hat_sample: Vec<u32> = (195..220).map(|x| bus.mcd212.framebuffer[428 * width + x] & 0xFFFFFF).collect();
            let nose_sample: Vec<u32> = (195..220).map(|x| bus.mcd212.framebuffer[434 * width + x] & 0xFFFFFF).collect();
            let mouth_sample: Vec<u32> = (195..220).map(|x| bus.mcd212.framebuffer[440 * width + x] & 0xFFFFFF).collect();
            let hat_hash: u64 = hat_sample.iter().fold(0, |acc, &p| acc.wrapping_mul(31).wrapping_add(p as u64));
            let nose_hash: u64 = nose_sample.iter().fold(0, |acc, &p| acc.wrapping_mul(31).wrapping_add(p as u64));
            let mouth_hash: u64 = mouth_sample.iter().fold(0, |acc, &p| acc.wrapping_mul(31).wrapping_add(p as u64));
            println!("FRAME {}: hat_h=0x{:08X} nose_h=0x{:08X} mouth_h=0x{:08X}", frame, hat_hash, nose_hash, mouth_hash);
        }

        if frame >= 5700 && (frame % 10 == 0 || frame == 5850) {
            let width = mcd212::SCREEN_WIDTH as i32;
            let height = mcd212::SCREEN_HEIGHT as i32;
            let size = (width * height * 4) as u32;
            let mut bmp = Vec::with_capacity(54 + size as usize);
            bmp.extend_from_slice(b"BM");
            bmp.extend_from_slice(&(54 + size).to_le_bytes());
            bmp.extend_from_slice(&[0, 0, 0, 0]);
            bmp.extend_from_slice(&54u32.to_le_bytes());
            bmp.extend_from_slice(&40u32.to_le_bytes());
            bmp.extend_from_slice(&width.to_le_bytes());
            bmp.extend_from_slice(&(-height).to_le_bytes());
            bmp.extend_from_slice(&1u16.to_le_bytes());
            bmp.extend_from_slice(&32u16.to_le_bytes());
            bmp.extend_from_slice(&0u32.to_le_bytes());
            bmp.extend_from_slice(&size.to_le_bytes());
            bmp.extend_from_slice(&2835u32.to_le_bytes());
            bmp.extend_from_slice(&2835u32.to_le_bytes());
            bmp.extend_from_slice(&0u32.to_le_bytes());
            bmp.extend_from_slice(&0u32.to_le_bytes());
            for &p in &bus.mcd212.framebuffer[..] {
                bmp.push((p & 0xFF) as u8);
                bmp.push(((p >> 8) & 0xFF) as u8);
                bmp.push(((p >> 16) & 0xFF) as u8);
                bmp.push(0xFF);
            }
            let _ = std::fs::write(format!("traces/mario_step_{}.bmp", frame), &bmp);
            println!("Saved traces/mario_step_{}.bmp", frame);
        }

        if frame == 5701 {
            println!("Reached frame 5701! Exiting early.");
            break;
        }
    }

    Ok(())
}
