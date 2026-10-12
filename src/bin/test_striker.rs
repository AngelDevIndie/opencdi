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

fn save_bmp(bus: &CdiBus, path: &str) -> std::io::Result<()> {
    let width = mcd212::SCREEN_WIDTH as i32;
    let height = mcd212::SCREEN_HEIGHT as i32;
    let file_size = 54 + (width * height * 4) as u32;

    let mut bmp = Vec::with_capacity(file_size as usize);
    bmp.extend_from_slice(b"BM");
    bmp.extend_from_slice(&file_size.to_le_bytes());
    bmp.extend_from_slice(&[0, 0, 0, 0]);
    bmp.extend_from_slice(&54u32.to_le_bytes());

    bmp.extend_from_slice(&40u32.to_le_bytes());
    bmp.extend_from_slice(&width.to_le_bytes());
    bmp.extend_from_slice(&(-height).to_le_bytes()); // top-down
    bmp.extend_from_slice(&1u16.to_le_bytes());
    bmp.extend_from_slice(&32u16.to_le_bytes());
    bmp.extend_from_slice(&0u32.to_le_bytes());
    bmp.extend_from_slice(&((width * height * 4) as u32).to_le_bytes());
    bmp.extend_from_slice(&2835u32.to_le_bytes());
    bmp.extend_from_slice(&2835u32.to_le_bytes());
    bmp.extend_from_slice(&0u32.to_le_bytes());
    bmp.extend_from_slice(&0u32.to_le_bytes());

    for &pixel in &bus.mcd212.framebuffer[..] {
        let b = (pixel & 0xFF) as u8;
        let g = ((pixel >> 8) & 0xFF) as u8;
        let r = ((pixel >> 16) & 0xFF) as u8;
        bmp.push(b);
        bmp.push(g);
        bmp.push(r);
        bmp.push(0xFF);
    }
    std::fs::write(path, &bmp)?;
    println!("Saved framebuffer BMP to {:?}", path);
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== OpenCDi - Test Striker Pro (USA) ===");

    let mut bus = CdiBus::new();
    let bios_path = find_file("bios/cdi220b.rom").expect("cdi220b.rom missing");
    bus.load_rom(&bios_path)?;

    if let Some(nvram_path) = find_file("bios/nvram_mk48t08.bin") {
        let _ = bus.nvram.load_from_file(&nvram_path);
    }

    let disc_path = find_file("giochi/Striker Pro (USA).chd").expect("Striker missing");
    bus.load_cdrom(&disc_path)?;

    // Keep PAL as default
    // bus.set_video_mode(crate::mcd212::VideoMode::Ntsc);

    let mut cpu = CpuCore::new();
    cpu.set_cpu_type(CpuType::SCC68070);
    cpu.reset(&mut bus);

    bus.watch_addr1 = 0x004FFFE4;
    bus.watch_addr2 = 0x00012671;
    bus.watch_enabled = true;

    let mut click_frame = 0;
    let mut disc_launched = false;

    let total_lines = bus.mcd212.video_mode.total_scanlines();
    let cyc_per_line = bus.mcd212.video_mode.cycles_per_scanline();

    let mut shell_ready = 0;
    for frame in 1..=9500 {
        let play_a6 = 0x0000C000u32;
        let state_9172 = bus.read_long(play_a6.wrapping_add(0xFFFF9172));

        if state_9172 >= 2 && !disc_launched {
            shell_ready += 1;
            if shell_ready == 30 || shell_ready == 180 {
                println!(">>> [Frame {}] State 9172 is {}, shell_ready={} -> CLICK PLAY CD-I at (575, 335)!", frame, state_9172, shell_ready);
                bus.slave.update_mouse(575, 335, 1);
            } else if shell_ready == 60 || shell_ready == 210 {
                println!(">>> [Frame {}] RELEASE PLAY CD-I CLICK!", frame);
                bus.slave.update_mouse(575, 335, 0);
            }
        }

        if !disc_launched && cpu.pc >= 0x00010000 && cpu.pc < 0x00300000 && frame > 250 {
            println!(">>> [Frame {}] Striker launched in RAM! PC=0x{:08X}", frame, cpu.pc);
            disc_launched = true;
            bus.emulate_wait_states = false;
        }

        // Advance through title / language screens
        if frame == 1600 {
            println!(">>> [Frame {}] Simulating Button 1 Press to skip intro...", frame);
            bus.slave.update_mouse(384, 280, 1);
        }
        if frame == 1620 {
            bus.slave.update_mouse(384, 280, 0);
        }

        // Language Screen appears ~5500: click ENGLISH at (384, 180)
        if frame == 5600 {
            println!(">>> [Frame {}] Clicking ENGLISH at (384, 180)...", frame);
            bus.slave.update_mouse(384, 180, 1);
        }
        if frame == 5620 {
            bus.slave.update_mouse(384, 180, 0);
        }

        // Main mode select appears ~5750: click Friendly match (gloves) at (600, 160)
        if frame == 5900 {
            println!(">>> [Frame {}] Clicking Friendly match at (600, 160)...", frame);
            bus.slave.update_mouse(600, 160, 1);
        }
        if frame == 5920 {
            bus.slave.update_mouse(600, 160, 0);
        }

        // Click green square button at (415, 200) to select ARGENTINA as player team
        if frame == 7100 {
            println!(">>> [Frame {}] Clicking GREEN SELECT BUTTON at (415, 200)...", frame);
            bus.slave.update_mouse(415, 200, 1);
        }
        if frame == 7120 {
            bus.slave.update_mouse(415, 200, 0);
        }

        // Scroll down at (415, 260)
        if frame == 7400 {
            println!(">>> [Frame {}] Scrolling down at (415, 260)...", frame);
            bus.slave.update_mouse(415, 260, 1);
        }
        if frame == 7420 {
            bus.slave.update_mouse(415, 260, 0);
        }

        // Select second team at (415, 200)
        if frame == 7700 {
            println!(">>> [Frame {}] Clicking GREEN SELECT BUTTON at (415, 200)...", frame);
            bus.slave.update_mouse(415, 200, 1);
        }
        if frame == 7720 {
            bus.slave.update_mouse(415, 200, 0);
        }

        // Click table / start icon at (370, 390)
        if frame == 8100 || frame == 8500 || frame == 8900 {
            println!(">>> [Frame {}] Clicking table/start at (370, 390)...", frame);
            bus.slave.update_mouse(370, 390, 1);
        }
        if frame == 8120 || frame == 8520 || frame == 8920 {
            bus.slave.update_mouse(370, 390, 0);
        }

        if frame % 250 == 0 && frame >= 6500 {
            let _ = save_bmp(&bus, &format!("striker_menu_{}.bmp", frame));
        }

        if frame == 5490 {
            println!("=== DUMPING ICA AT 0x27C7D0 (byte 0x07C7D0) ===");
            let plane = &bus.mcd212.plane_b;
            let mut a = 0x07C7D0usize;
            for _ in 0..60 {
                if a + 4 > plane.len() { break; }
                let cmd = u32::from_be_bytes([plane[a], plane[a+1], plane[a+2], plane[a+3]]);
                let op = (cmd >> 24) as u8;
                let arg = cmd & 0x00FF_FFFF;
                if op != 0x10 {
                    print!("  0x{:06X}: 0x{:08X} -> ", a, cmd);
                    match op {
                        0x00..=0x0F => println!("STOP"),
                        0x10..=0x1F => println!("NOP"),
                        0x20..=0x2F => println!("DCP(0x{:06X})", arg),
                        0x30..=0x3F => println!("DCP_STOP(0x{:06X})", arg),
                        0x40..=0x4F => println!("ICA_JUMP(0x{:06X})", arg),
                        0x50..=0x5F => println!("VSR_STOP(0x{:06X})", arg),
                        0x60..=0x6F => println!("INT"),
                        0x78..=0x7F => println!("DISPPARAM(0x{:02X})", arg),
                        _ => println!("REG_{:02X}(0x{:06X})", op, arg),
                    }
                }
                a += 4;
                if op <= 0x0F || op == 0x30 || op == 0x50 { break; }
            }

            println!("=== DUMPING DCA AT 0x27D328 (byte 0x07D328) ===");
            let mut b = 0x07D328usize;
            for _ in 0..40 {
                if b + 4 > plane.len() { break; }
                let cmd = u32::from_be_bytes([plane[b], plane[b+1], plane[b+2], plane[b+3]]);
                let op = (cmd >> 24) as u8;
                let arg = cmd & 0x00FF_FFFF;
                print!("  0x{:06X}: 0x{:08X} -> ", b, cmd);
                match op {
                    0x00..=0x0F => println!("STOP"),
                    0x10..=0x1F => println!("NOP"),
                    0x20..=0x2F => println!("DCP(0x{:06X})", arg),
                    0x30..=0x3F => println!("DCP_STOP(0x{:06X})", arg),
                    0x40..=0x4F => println!("VSR(0x{:06X})", arg),
                    0x50..=0x5F => println!("VSR_STOP(0x{:06X})", arg),
                    0x60..=0x6F => println!("INT"),
                    0x78..=0x7F => println!("DISPPARAM(0x{:02X})", arg),
                    _ => println!("REG_{:02X}(0x{:06X})", op, arg),
                }
                b += 4;
                if op <= 0x0F || op == 0x30 || op == 0x50 { break; }
            }
        }

        let mut prev_watch_val = bus.read_byte(0x00012671);
        let mut writes_this_frame = 0;
        let mut cycle_debt: i32 = 0;
        let mut frame_total_cycles = 0;
        let mut frame_total_wait = 0;
        for line in 0..total_lines {
            bus.tick_scanline(line);

            if bus.mcd212.vblank_irq {
                if frame >= 5490 && frame <= 5495 {
                    println!("  [F{} L{}] vblank_irq is TRUE", frame, line);
                }
                bus.scc_timer.lir |= 0x80;
                bus.mcd212.vblank_irq = false;
            }

            let int1_lvl = if (bus.scc_timer.lir & 0x80) != 0 {
                let lvl = (bus.scc_timer.lir >> 4) & 7;
                if lvl != 0 { lvl } else { 5 }
            } else { 0 };

            let timer_lvl = if bus.scc_timer.timer_irq {
                bus.scc_timer.picr1 & 7
            } else { 0 };

            let cdic_lvl = if bus.cdic.irq_pending || bus.dvc.irq_pending() { 4 } else { 0 };
            let slave_lvl = if bus.slave.irq_pending { 2 } else { 0 };

            let highest_ipl = int1_lvl.max(timer_lvl).max(cdic_lvl).max(slave_lvl);
            cpu.set_irq(highest_ipl);

            if line == 35 && frame >= 5490 && frame <= 5502 {
                println!("  [F{} L35] VSR=[0x{:06X}, 0x{:06X}] DCR=[0x{:04X}, 0x{:04X}]",
                    frame, bus.mcd212.get_vsr::<0>(), bus.mcd212.get_vsr::<1>(),
                    bus.mcd212.dcr[0], bus.mcd212.dcr[1]);
            }

            let mut line_cycles = cycle_debt;
            while line_cycles < cyc_per_line as i32 {
                let remaining = (cyc_per_line as i32 - line_cycles).max(1);
                let budget = remaining.min(32);

                bus.current_pc = cpu.pc;
                bus.current_line_cycle = line_cycles.max(0) as u32;

                let batch = cpu.run_for_cycles(&mut bus, budget);
                let wait = bus.consume_wait_cycles();
                let mut batch_cycles = batch.cycles + wait;
                frame_total_cycles += batch_cycles;
                frame_total_wait += wait;

                match batch.exit {
                    m68k::CycleBatchExit::TrapInstruction { trap_num } => {
                        let trap_cyc = cpu.take_trap_exception(&mut bus, trap_num);
                        batch_cycles += trap_cyc;
                    }
                    m68k::CycleBatchExit::AlineTrap { .. } => {
                        let trap_cyc = cpu.take_aline_exception(&mut bus);
                        batch_cycles += trap_cyc;
                    }
                    m68k::CycleBatchExit::FlineTrap { .. } => {
                        let trap_cyc = cpu.take_fline_exception(&mut bus);
                        batch_cycles += trap_cyc;
                    }
                    m68k::CycleBatchExit::IllegalInstruction { .. } => {
                        let trap_cyc = cpu.take_illegal_exception(&mut bus);
                        batch_cycles += trap_cyc;
                    }
                    m68k::CycleBatchExit::Breakpoint { .. } => {
                        let trap_cyc = cpu.take_bkpt_exception(&mut bus);
                        batch_cycles += trap_cyc;
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
                let timer_lvl = if bus.scc_timer.timer_irq {
                    let lvl = bus.scc_timer.picr1 & 7;
                    if lvl != 0 { lvl } else { 6 }
                } else { 0 };
                let cdic_lvl = if bus.cdic.irq_pending || bus.dvc.irq_pending() { 4 } else { 0 };
                let slave_lvl = if bus.slave.irq_pending { 2 } else { 0 };
                let highest_ipl = int1_lvl.max(timer_lvl).max(cdic_lvl).max(slave_lvl);
                cpu.set_irq(highest_ipl);
            }

            cycle_debt = (line_cycles - cyc_per_line as i32).clamp(0, 100);
            bus.render_scanline(line);

            let cur_val = bus.read_byte(0x00012671);
            if cur_val == 0xFF && prev_watch_val != 0xFF {
                writes_this_frame += 1;
            }
            prev_watch_val = cur_val;

            bus.tick_slave(cyc_per_line);
            bus.tick_cdic(cyc_per_line);
            bus.tick_dvc(cyc_per_line);
        }
        if frame >= 5489 && frame <= 5492 {
            println!("  [Frame {}] frame_total_cycles={}, frame_total_wait={}", frame, frame_total_cycles, frame_total_wait);
        }
        if frame >= 5493 && frame <= 5498 {
            let bmp_path = format!("striker_game_{}.bmp", frame);
            let _ = save_bmp(&bus, &bmp_path);
            println!("[Frame {}] Saved {}", frame, bmp_path);
        }
    }

    for &addr in &[0x0024EC60u32, 0x00261C30u32, 0x00261DF0u32, 0x00263700u32, 0x002632D0u32] {
        println!("=== DISASSEMBLY AT 0x{:08X} ===", addr);
        let mut pc = addr;
        for _ in 0..20 {
            let op = bus.read_word(pc);
            let (dis, _) = m68k::dasm::disassemble(pc, op, CpuType::SCC68070);
            println!("  0x{:08X}: {:04X}  {}", pc, op, dis);
            pc += 2;
        }
    }

    Ok(())
}
