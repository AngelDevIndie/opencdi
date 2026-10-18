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

use std::env;
use std::path::{Path, PathBuf};
use m68k::{CpuCore, CpuType, AddressBus};

use crate::bus::CdiBus;
use crate::mcd212::{SCREEN_WIDTH, SCREEN_HEIGHT, CYCLES_PER_SCANLINE};

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

fn save_bmp(fb: &[u32], path: &str) -> std::io::Result<()> {
    let width = SCREEN_WIDTH as i32;
    let height = SCREEN_HEIGHT as i32;
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

    for &pixel in fb {
        let b = (pixel & 0xFF) as u8;
        let g = ((pixel >> 8) & 0xFF) as u8;
        let r = ((pixel >> 16) & 0xFF) as u8;
        bmp.push(b);
        bmp.push(g);
        bmp.push(r);
        bmp.push(0xFF);
    }
    std::fs::write(path, &bmp)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = env_logger::try_init();
    let args: Vec<String> = env::args().collect();
    let target = if args.len() > 1 { args[1].as_str() } else { "gateway" };
    let max_frames: usize = if args.len() > 2 { args[2].parse().unwrap_or(2500) } else { 2500 };

    println!("=== OpenCDi - Test Video CD ===");
    println!("Target: {} | Max Frames: {}", target, max_frames);

    let mut bus = CdiBus::new();

    let bios_path = find_file("bios/cdi220b.rom").expect("cdi220b.rom not found");
    bus.load_rom(&bios_path)?;
    println!("Loaded BIOS: {:?}", bios_path);

    if let Some(nv) = find_file("bios/nvram_mk48t08.bin") {
        let _ = bus.nvram.load_from_file(&nv);
        println!("Loaded NVRAM");
    }

    let dvc_path = find_file("dvc/vmpega.rom").expect("dvc/vmpega.rom not found");
    bus.load_dvc_rom(&dvc_path)?;
    println!("Loaded DVC ROM: {:?}", dvc_path);

    let disc_subpath = match target {
        "gateway" => "videocd/Gateway to the Mind/Gateway to the Mind.cue",
        "kilby" => "videocd/CD-i See It Hear It Feel It (USA) (Kilby Predicts) (Rev 1)/CD-i See It Hear It Feel It (USA) (Kilby Predicts) (Rev 1).cue",
        other => other,
    };

    let disc_path = find_file(disc_subpath).expect("Disc not found");
    bus.load_cdrom(&disc_path)?;
    println!("Loaded Disc: {:?}", disc_path);

    let mut cpu = CpuCore::new();
    cpu.set_cpu_type(CpuType::SCC68070);
    cpu.reset(&mut bus);
    println!("CPU Reset OK - Initial PC: 0x{:08X}", cpu.pc);

    let cyc_per_line = CYCLES_PER_SCANLINE;
    let mut cycle_debt: i32 = 0;
    let mut disc_launched = false;
    let mut shell_ready_count = 0;
    let mut prev_cdic_cmd = 0u16;
    let mut prev_cdic_lba = 0u32;

    let brain_dir = "C:/Users/Gian/.gemini/antigravity/brain/3c367666-9892-4238-9468-c77864b7ddc9";

    for frame in 1..=max_frames {
        let total_lines = bus.mcd212.video_mode.total_scanlines();

        for line in 0..total_lines {
            bus.tick_scanline(line);

            if bus.mcd212.vblank_irq {
                bus.scc_timer.lir |= 0x80;
                bus.mcd212.vblank_irq = false;
            }

            let int1_lvl = if (bus.scc_timer.lir & 0x80) != 0 {
                let prio = (bus.scc_timer.lir >> 4) & 7;
                if prio != 0 { prio } else { 5 }
            } else { 0 };

            let timer_lvl = if bus.scc_timer.timer_irq {
                bus.scc_timer.picr1 & 7
            } else { 0 };

            let cdic_lvl = if bus.in4_pending() { 4 } else { 0 };
            let slave_lvl = if bus.slave.irq_pending { 2 } else { 0 };

            let highest_ipl = int1_lvl.max(timer_lvl).max(cdic_lvl).max(slave_lvl);
            cpu.set_irq(highest_ipl);

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
                        break;
                    }
                    _ => {}
                }

                line_cycles += batch_cycles;
                bus.tick_timer(batch_cycles as u32);
            }

            cycle_debt = (line_cycles - cyc_per_line as i32).clamp(0, 100);



            bus.render_scanline(line);
            bus.tick_slave(cyc_per_line);
            bus.tick_cdic(cyc_per_line);
            bus.tick_dvc(cyc_per_line);
        }

        if frame >= 850 && (bus.cdic.command != prev_cdic_cmd || (bus.cdic.curr_lba != prev_cdic_lba && bus.cdic.curr_lba % 100 == 0)) {
            if bus.cdic.command != prev_cdic_cmd {
                println!("[Frame {} CDIC] Command changed: 0x{:02X} -> 0x{:02X} at LBA={}", frame, prev_cdic_cmd, bus.cdic.command, bus.cdic.curr_lba);
                prev_cdic_cmd = bus.cdic.command;
            }
            prev_cdic_lba = bus.cdic.curr_lba;
        }

        // Auto-launch from BIOS shell
        if bus.cdrom.is_some() && !disc_launched {
            let (_effective_shell_base, shell_state) = if bus.dvc.is_fitted() {
                let s_dvc = bus.read_long(0x00D08090u32.wrapping_add(0xFFFF9172));
                if s_dvc >= 2 {
                    (0x00D08090u32, s_dvc)
                } else {
                    (0x0000C000u32, bus.read_long(0x0000C000u32.wrapping_add(0xFFFF9172)))
                }
            } else {
                (0x0000C000u32, bus.read_long(0x0000C000u32.wrapping_add(0xFFFF9172)))
            };

            if shell_state >= 2 {
                shell_ready_count += 1;
                if shell_ready_count == 1 || shell_ready_count == 150 {
                    bus.slave.update_mouse(575, 335, 0);
                } else if shell_ready_count == 30 || shell_ready_count == 180 {
                    println!(">>> [Frame {}] Shell BIOS ready (state {}): Click PLAY CD-I...", frame, shell_state);
                    bus.slave.update_mouse(575, 335, 1);
                } else if shell_ready_count == 50 || shell_ready_count == 200 {
                    println!(">>> [Frame {}] Release Click PLAY CD-I", frame);
                    bus.slave.update_mouse(575, 335, 0);
                }
            }

            if ((cpu.pc >= 0x00010000 && cpu.pc < 0x00300000) || (cpu.pc >= 0x00D00000 && cpu.pc < 0x00E00000)) && frame > 250 {
                println!(">>> [Frame {}] Disc application executing in RAM (PC=0x{:06X})!", frame, cpu.pc);
                disc_launched = true;
                bus.slave.update_mouse(384, 280, 0);
            }
        }



        if frame == 1600 {
            let mx = bus.slave.device_mouse_x;
            let my = bus.slave.device_mouse_y;
            println!(">>> [Frame {}] Clicking VCD menu at ({}, {})", frame, mx, my);
            bus.slave.update_mouse(mx, my, 1);
        } else if frame == 1615 {
            let mx = bus.slave.device_mouse_x;
            let my = bus.slave.device_mouse_y;
            println!(">>> [Frame {}] Releasing click at ({}, {})", frame, mx, my);
            bus.slave.update_mouse(mx, my, 0);
        }

        if (frame >= 1340 && frame <= 1500 && frame % 10 == 0) || (frame >= 2490 && frame <= 2550 && frame % 5 == 0) || frame % 100 == 0 || frame == max_frames {
            let plane_a_nz = bus.mcd212.plane_a.iter().take(0x40000).filter(|&&b| b != 0).count();
            let plane_b_nz = bus.mcd212.plane_b.iter().take(0x40000).filter(|&&b| b != 0).count();
            println!(
                "[Frame {:4}] PC=0x{:06X} | DVC: play={}, arm={}, dec={}, aud={}, uflow={}, dec_cnt={}, 5C=0x{:04X}, fmv_isr=0x{:04X}/ier=0x{:04X}, fma_isr=0x{:04X}/ier=0x{:04X} | CDIC: LBA={}, CMD=0x{:02X}, file=0x{:04X}, ch=0x{:08X}, ach=0x{:04X} | MCD212: icm=0x{:06X}, tc=0x{:06X} | PA={}, PB={}",
                frame, cpu.pc,
                bus.dvc.video_playing, bus.dvc.video_armed, bus.dvc.decoder_enabled, bus.dvc.audio_enabled, bus.dvc.underflow_count,
                bus.dvc.frames_decoded_count,
                bus.dvc.get_fmv_word(0x5C),
                bus.dvc.get_fmv_word(crate::dvc::FMV_ISR), bus.dvc.get_fmv_word(crate::dvc::FMV_IER),
                bus.dvc.get_fma_word(crate::dvc::FMA_ISR), bus.dvc.get_fma_word(crate::dvc::FMA_IER),
                bus.cdic.curr_lba, bus.cdic.command, bus.cdic.file, bus.cdic.channel, bus.cdic.audio_channel,
                bus.mcd212.image_coding_method, bus.mcd212.transparency_control,
                plane_a_nz, plane_b_nz
            );
        }

        if frame == 850 || frame == 1200 || frame == 1600 || frame == 2000 || frame == max_frames {
            let path = format!("{}/vcd_{}_frame_{}.bmp", brain_dir, target, frame);
            let _ = save_bmp(&bus.mcd212.framebuffer[..], &path);
            println!("Saved screenshot: {}", path);
        }
    }

    Ok(())
}
