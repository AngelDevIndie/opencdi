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
use log::info;
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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    println!("=== OpenCDi - Test Boot Disc (Hotel Mario) ===");

    let mut bus = CdiBus::new();

    // 1. Load BIOS ROM
    let bios_path = find_file("bios/cdi220b.rom").expect("BIOS cdi220b.rom not found");
    bus.load_rom(&bios_path)?;
    println!("Loaded BIOS: {:?}", bios_path);

    // 2. Load NVRAM
    if let Some(nvram_path) = find_file("bios/nvram_mk48t08.bin") {
        let _ = bus.nvram.load_from_file(&nvram_path);
        println!("Loaded NVRAM: {:?}", nvram_path);
    }

    // 3. Load DVC ROM (Disabled for base CDI test)
    // if let Some(dvc_path) = find_file("dvc/vmpega.rom") {
    //     let _ = bus.load_dvc_rom(&dvc_path);
    //     println!("Loaded DVC ROM: {:?}", dvc_path);
    // }

    // 4. Load Game Disc (Hotel Mario)
    let game_path = find_file("giochi/Hotel Mario (USA).chd").expect("Game disc not found");
    bus.load_cdrom(&game_path)?;
    println!("Loaded Game Disc: {:?}", game_path);

    // 5. Initialize CPU
    let mut cpu = CpuCore::new();
    cpu.set_cpu_type(CpuType::SCC68070);
    cpu.reset(&mut bus);
    println!("CPU Reset OK - Initial PC: 0x{:08X}", cpu.pc);

    let trap0_vec = u32::from_be_bytes([
        bus.mcd212.plane_a[0x80],
        bus.mcd212.plane_a[0x81],
        bus.mcd212.plane_a[0x82],
        bus.mcd212.plane_a[0x83],
    ]);
    println!("Initial TRAP #0 vector: 0x{:08X}", trap0_vec);

    let mut trap_count = 0;
    let mut total_instructions: u64 = 0;

    let mut history: Vec<(u32, u16, String)> = Vec::new();

    // Run for 1200 frames (20 seconds)
    let mut stopped = false;
    let mut ring: Vec<String> = Vec::with_capacity(100);
    let mut cdic_irq_trace = 0;
    let mut prev_cdic_irq = false;
    let mut prev_slave_irq = false;

    let mut prev_vblank_irq = false;
    let mut click_frame = 0;
    for frame in 1..=4400 {
        let play_a6 = 0x0000C000u32;
        let mode_807e = bus.read_long(play_a6.wrapping_add(0xFFFF807E));
        let c390 = bus.read_long(play_a6.wrapping_add(0xFFFFC390));
        let state_9172 = bus.read_long(play_a6.wrapping_add(0xFFFF9172));

        if frame >= 800 && (frame % 30 == 0 || (c390 == 1 && click_frame == 0)) {
            let a576 = bus.read_long(play_a6.wrapping_add(0xFFFFA576));
            let a57a = bus.read_long(play_a6.wrapping_add(0xFFFFA57A));
            let val_918e = bus.read_long(play_a6.wrapping_add(0xFFFF918E));
            let val_932a = bus.read_word(play_a6.wrapping_add(0xFFFF932A));
            let val_aba6 = bus.read_long(play_a6.wrapping_add(0xFFFFABA6));
            println!(
                "[Frame {}] Play state: 807E=0x{:08X}, C390=0x{:08X}, 9172=0x{:08X}, A576=0x{:08X}, A57A=0x{:08X}, 918E=0x{:08X}, 932A=0x{:04X}, ABA6=0x{:08X}",
                frame, mode_807e, c390, state_9172, a576, a57a, val_918e, val_932a, val_aba6
            );
        }


        if frame == 820 {
            println!(">>> [Frame 820] Saving Main Player Framebuffer!");
            println!("MCD212 STATE: ICM=0x{:08X}, TP_CTRL=0x{:08X}, ORDER={}, DCR=[0x{:04X}, 0x{:04X}], DDR=[0x{:04X}, 0x{:04X}], DCP=[0x{:06X}, 0x{:06X}], VSR=[0x{:06X}, 0x{:06X}]",
                bus.mcd212.image_coding_method, bus.mcd212.transparency_control, bus.mcd212.plane_order,
                bus.mcd212.dcr[0], bus.mcd212.dcr[1], bus.mcd212.ddr[0], bus.mcd212.ddr[1],
                bus.mcd212.dcp[0], bus.mcd212.dcp[1], bus.mcd212.vsr[0], bus.mcd212.vsr[1]);
            println!("CLUT[0..8]: {:08X?}", &bus.mcd212.clut[0..8]);
            let nonzero_a = bus.mcd212.plane_a.iter().filter(|&&b| b != 0).count();
            let nonzero_b = bus.mcd212.plane_b.iter().filter(|&&b| b != 0).count();
            println!("Plane A nonzero bytes: {}, Plane B nonzero bytes: {}", nonzero_a, nonzero_b);
            println!("Plane A at 0x400..0x450:");
            for off in (0x400..0x450).step_by(4) {
                let w = u32::from_be_bytes([bus.mcd212.plane_a[off], bus.mcd212.plane_a[off+1], bus.mcd212.plane_a[off+2], bus.mcd212.plane_a[off+3]]);
                println!("  0x{:04X}: 0x{:08X}", off, w);
            }
            println!("Plane A at 0x7FFB0..0x80000:");
            for off in (0x7FFB0..0x80000).step_by(4) {
                let w = u32::from_be_bytes([bus.mcd212.plane_a[off], bus.mcd212.plane_a[off+1], bus.mcd212.plane_a[off+2], bus.mcd212.plane_a[off+3]]);
                println!("  0x{:05X}: 0x{:08X}", off, w);
            }
            let _ = save_bmp(&bus, "C:/Users/Gian/.gemini/antigravity/brain/47421117-6e2a-4536-8928-519a0c9b76ed/framebuffer_main.bmp");
            bus.slave.update_mouse(575, 335, 0);
        }

        if frame == 840 {
            println!(">>> [Frame 840] MOUSE CLICK ON PLAY CD-I BUTTON!");
            bus.slave.update_mouse(575, 335, 1);
        }

        if frame == 855 {
            println!(">>> [Frame 855] MOUSE RELEASE!");
            bus.slave.update_mouse(575, 335, 0);
        }

        if frame == 860 {
            println!(">>> [Frame 860] Saving Disc Ready Framebuffer!");
            let _ = save_bmp(&bus, "C:/Users/Gian/.gemini/antigravity/brain/47421117-6e2a-4536-8928-519a0c9b76ed/framebuffer_disc_ready.bmp");
        }

        if frame == 865 {
            println!(">>> [Frame 865] TRIGGERING DISC LAUNCH: C364 = 0x0500, 9172 = 2, 91A6 = 1 and C344 = 29 (Action 0x1D)!");
            bus.write_long(play_a6.wrapping_add(0xFFFFC364), 0x00000500);
            bus.write_long(play_a6.wrapping_add(0xFFFF9172), 2);
            bus.write_long(play_a6.wrapping_add(0xFFFF91A6), 1);
            bus.write_long(play_a6.wrapping_add(0xFFFFC344), 29);
        }

        if frame == 950 {
            println!(">>> [Frame 950] Saving Sysgo/Launcher Framebuffer!");
            let _ = save_bmp(&bus, "C:/Users/Gian/.gemini/antigravity/brain/47421117-6e2a-4536-8928-519a0c9b76ed/framebuffer_sysgo_launcher.bmp");
        }

        if frame == 1200 {
            println!(">>> [Frame 1200] Saving Mario Boot Framebuffer!");
            println!("FRAME 1200 MCD212 STATE: ICM=0x{:08X}, TP_CTRL=0x{:08X}, ORDER={}, DCR=[0x{:04X}, 0x{:04X}], DDR=[0x{:04X}, 0x{:04X}], DCP=[0x{:06X}, 0x{:06X}], VSR=[0x{:06X}, 0x{:06X}]",
                bus.mcd212.image_coding_method, bus.mcd212.transparency_control, bus.mcd212.plane_order,
                bus.mcd212.dcr[0], bus.mcd212.dcr[1], bus.mcd212.ddr[0], bus.mcd212.ddr[1],
                bus.mcd212.dcp[0], bus.mcd212.dcp[1], bus.mcd212.vsr[0], bus.mcd212.vsr[1]);
            let _ = save_bmp(&bus, "C:/Users/Gian/.gemini/antigravity/brain/47421117-6e2a-4536-8928-519a0c9b76ed/framebuffer_mario_boot.bmp");
        }

        if frame == 1600 {
            println!(">>> [Frame 1600] Saving Mario Bumper Anim Framebuffer!");
            let _ = save_bmp(&bus, "C:/Users/Gian/.gemini/antigravity/brain/47421117-6e2a-4536-8928-519a0c9b76ed/framebuffer_mario_anim.bmp");
        }

        if frame == 1650 {
            println!(">>> [Frame 1650] SIMULATING CONTROLLER BUTTON 1 PRESS to skip/advance intro!");
            bus.slave.update_mouse(384, 280, 1);
        }

        if frame == 1660 {
            println!(">>> [Frame 1660] RELEASING CONTROLLER BUTTON 1!");
            bus.slave.update_mouse(384, 280, 0);
        }

        if frame == 1700 {
            println!(">>> [Frame 1700] Saving Mario Framebuffer 1700!");
            let _ = save_bmp(&bus, "C:/Users/Gian/.gemini/antigravity/brain/47421117-6e2a-4536-8928-519a0c9b76ed/framebuffer_mario_1700.bmp");
        }

        if frame == 1800 {
            println!(">>> [Frame 1800] Saving Mario Framebuffer 1800!");
            let _ = save_bmp(&bus, "C:/Users/Gian/.gemini/antigravity/brain/47421117-6e2a-4536-8928-519a0c9b76ed/framebuffer_mario_1800.bmp");
        }

        if frame == 1900 {
            println!(">>> [Frame 1900] Saving Mario Framebuffer 1900!");
            let _ = save_bmp(&bus, "C:/Users/Gian/.gemini/antigravity/brain/47421117-6e2a-4536-8928-519a0c9b76ed/framebuffer_mario_1900.bmp");
        }

        if frame == 2000 {
            println!(">>> [Frame 2000] Saving Mario Framebuffer 2000!");
            let _ = save_bmp(&bus, "C:/Users/Gian/.gemini/antigravity/brain/47421117-6e2a-4536-8928-519a0c9b76ed/framebuffer_mario_2000.bmp");
        }

        if frame == 2100 {
            println!(">>> [Frame 2100] SIMULATING CONTROLLER BUTTON 1 PRESS!");
            bus.slave.update_mouse(384, 280, 1);
        }

        if frame == 2110 {
            println!(">>> [Frame 2110] RELEASING CONTROLLER BUTTON 1!");
            bus.slave.update_mouse(384, 280, 0);
        }

        if frame == 2200 {
            println!(">>> [Frame 2200] Saving Mario Framebuffer 2200!");
            let _ = save_bmp(&bus, "C:/Users/Gian/.gemini/antigravity/brain/47421117-6e2a-4536-8928-519a0c9b76ed/framebuffer_mario_2200.bmp");
        }

        if frame == 2400 {
            println!(">>> [Frame 2400] Saving Mario Framebuffer 2400!");
            let _ = save_bmp(&bus, "C:/Users/Gian/.gemini/antigravity/brain/47421117-6e2a-4536-8928-519a0c9b76ed/framebuffer_mario_2400.bmp");
        }

        if frame == 2600 {
            println!(">>> [Frame 2600] Total audio sectors decoded: {}", bus.cdic.audio.total_sectors_decoded);
            println!(">>> [Frame 2600] Saving Mario Framebuffer 2600!");
            let _ = save_bmp(&bus, "C:/Users/Gian/.gemini/antigravity/brain/47421117-6e2a-4536-8928-519a0c9b76ed/framebuffer_mario_2600.bmp");
        }

        if frame == 2650 {
            println!(">>> [Frame 2650] PRESSING BUTTON 1 ON '1 PLAYER' TO ENTER HOTEL 1!");
            bus.slave.update_mouse(384, 280, 1);
        }

        if frame == 2670 {
            println!(">>> [Frame 2670] RELEASING BUTTON 1!");
            bus.slave.update_mouse(384, 280, 0);
        }

        if frame == 2700 {
            println!(">>> [Frame 2700] Saving Framebuffer 2700!");
            let _ = save_bmp(&bus, "C:/Users/Gian/.gemini/antigravity/brain/47421117-6e2a-4536-8928-519a0c9b76ed/framebuffer_mario_2700.bmp");
        }

        if frame == 2750 {
            println!(">>> [Frame 2750] PRESSING BUTTON 1 ON 'NEW' TO START NEW GAME!");
            bus.slave.update_mouse(384, 280, 1);
        }

        if frame == 2770 {
            println!(">>> [Frame 2770] RELEASING BUTTON 1!");
            bus.slave.update_mouse(384, 280, 0);
        }

        if frame == 2800 {
            println!(">>> [Frame 2800] Saving Framebuffer 2800!");
            let _ = save_bmp(&bus, "C:/Users/Gian/.gemini/antigravity/brain/47421117-6e2a-4536-8928-519a0c9b76ed/framebuffer_mario_2800.bmp");
        }

        if frame == 2900 {
            println!(">>> [Frame 2900] Saving Framebuffer 2900!");
            let _ = save_bmp(&bus, "C:/Users/Gian/.gemini/antigravity/brain/47421117-6e2a-4536-8928-519a0c9b76ed/framebuffer_mario_2900.bmp");
        }

        if frame == 3000 {
            println!(">>> [Frame 3000] Saving Framebuffer 3000!");
            let _ = save_bmp(&bus, "C:/Users/Gian/.gemini/antigravity/brain/47421117-6e2a-4536-8928-519a0c9b76ed/framebuffer_mario_3000.bmp");
        }

        if frame == 3100 {
            println!(">>> [Frame 3100] Saving Framebuffer 3100!");
            let _ = save_bmp(&bus, "C:/Users/Gian/.gemini/antigravity/brain/47421117-6e2a-4536-8928-519a0c9b76ed/framebuffer_mario_3100.bmp");
        }

        if frame == 3200 {
            println!(">>> [Frame 3200] Saving Framebuffer 3200!");
            let _ = save_bmp(&bus, "C:/Users/Gian/.gemini/antigravity/brain/47421117-6e2a-4536-8928-519a0c9b76ed/framebuffer_mario_3200.bmp");
        }

        if frame == 3300 {
            println!(">>> [Frame 3300] Saving Framebuffer 3300!");
            let _ = save_bmp(&bus, "C:/Users/Gian/.gemini/antigravity/brain/47421117-6e2a-4536-8928-519a0c9b76ed/framebuffer_mario_3300.bmp");
        }

        if frame == 3500 {
            println!(">>> [Frame 3500] Saving Framebuffer 3500!");
            let _ = save_bmp(&bus, "C:/Users/Gian/.gemini/antigravity/brain/47421117-6e2a-4536-8928-519a0c9b76ed/framebuffer_mario_3500.bmp");
        }

        if frame == 3800 {
            println!(">>> [Frame 3800] Saving Framebuffer 3800!");
            let _ = save_bmp(&bus, "C:/Users/Gian/.gemini/antigravity/brain/47421117-6e2a-4536-8928-519a0c9b76ed/framebuffer_mario_3800.bmp");
        }

        if frame == 3850 {
            println!(">>> [Frame 3850] PRESSING BUTTON 1 ON 'Press Button One To Begin!' TO START STAGE 1 GAMEPLAY!");
            bus.slave.update_mouse(384, 280, 1);
        }

        if frame == 3870 {
            println!(">>> [Frame 3870] RELEASING BUTTON 1!");
            bus.slave.update_mouse(384, 280, 0);
        }

        if frame >= 4000 && frame <= 4004 {
            let path = format!("C:/Users/Gian/.gemini/antigravity/brain/47421117-6e2a-4536-8928-519a0c9b76ed/mario_flicker_{}.bmp", frame);
            println!(">>> [Frame {}] Saving Framebuffer to {}!", frame, path);
            let _ = save_bmp(&bus, &path);
            let mcd = &bus.mcd212;
            println!("MCD at {}: ICM=0x{:08X}, TP=0x{:08X}, ORD={}, VSR=[0x{:06X}, 0x{:06X}], DCP=[0x{:06X}, 0x{:06X}]",
                frame, mcd.image_coding_method, mcd.transparency_control, mcd.plane_order,
                mcd.get_vsr::<0>(), mcd.get_vsr::<1>(), mcd.dcp[0], mcd.dcp[1]);
        }
        if frame == 4005 {
            break;
        }

        for _line in 0..TOTAL_SCANLINES {
            if bus.mcd212.vblank_irq && !prev_vblank_irq && frame >= 1500 && frame <= 1620 {
                println!(">>> [VBLANK IRQ DETECTED] at frame {}, line {}", frame, _line);
            }
            prev_vblank_irq = bus.mcd212.vblank_irq;

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
            let cdic_lvl = if bus.cdic.irq_pending { 4 } else { 0 };
            let slave_lvl = if bus.slave.irq_pending { 2 } else { 0 };
            let highest_ipl = int1_lvl.max(timer_lvl).max(cdic_lvl).max(slave_lvl);
            cpu.set_irq(highest_ipl);

            let mut line_cycles = 0;
            while line_cycles < CYCLES_PER_SCANLINE as i32 {
                let single_step = false;
                let budget = if single_step { 1 } else { (CYCLES_PER_SCANLINE as i32 - line_cycles).max(1) };
                let old_pc = cpu.pc;
                bus.current_pc = old_pc;
                if (865..=880).contains(&frame) && (
                    old_pc == 0x4665AC || old_pc == 0x466984 || old_pc == 0x464FAC || 
                    old_pc == 0x4618DE || old_pc == 0x461894 || old_pc == 0x41650A || 
                    old_pc == 0x416520 || (old_pc >= 0x464EAC && old_pc <= 0x464EBA) || 
                    (old_pc >= 0x466570 && old_pc <= 0x4665B0) || (old_pc >= 0x466990 && old_pc <= 0x4669E4) ||
                    (old_pc >= 0x465636 && old_pc <= 0x465660) || (old_pc >= 0x47059A && old_pc <= 0x4705C0)
                ) {
                    let op = bus.read_word(old_pc);
                    let (dis, _) = m68k::dasm::disassemble(old_pc, op, m68k::CpuType::SCC68070);
                    println!("  [PLAY-F{}] PC=0x{:06X} {:<20} | D0={:08X} D1={:08X} D4={:08X} D5={:08X} SP={:08X}",
                        frame, old_pc, dis, cpu.d(0), cpu.d(1), cpu.d(4), cpu.d(5), cpu.a(7));
                }

                if (1601..=1620).contains(&frame) && old_pc >= 0x200000 && old_pc < 0x300000 {
                    let op = bus.read_word(old_pc);
                    let (dis, _) = m68k::dasm::disassemble(old_pc, op, m68k::CpuType::SCC68070);
                    println!("  [LAUNCHER-F{} L{:3}] PC=0x{:06X} {:<24} | D0={:08X} D1={:08X} D2={:08X} D3={:08X} A0={:08X} A1={:08X} SP={:08X}",
                        frame, _line, old_pc, dis, cpu.d(0), cpu.d(1), cpu.d(2), cpu.d(3), cpu.a(0), cpu.a(1), cpu.a(7));
                }

                let batch = cpu.run_for_cycles(&mut bus, budget);
                total_instructions += batch.instructions as u64;
                line_cycles += batch.cycles;

                if old_pc == 0x4687F6 || old_pc == 0x4695DA || old_pc == 0x46961A || old_pc == 0x469676 || old_pc == 0x469726 || old_pc == 0x46975E || old_pc == 0x469764 || old_pc == 0x469780 || old_pc == 0x469788 || old_pc == 0x4627AC || old_pc == 0x4627E8 || old_pc == 0x4627F0 || old_pc == 0x464F30 || old_pc == 0x464F60 || old_pc == 0x464F9C || old_pc == 0x465DE8 || old_pc == 0x468A14 || old_pc == 0x465EA0 || old_pc == 0x465E5E || old_pc == 0x465E62 || old_pc == 0x465E6A || old_pc == 0x465E74 || old_pc == 0x465E88 || old_pc == 0x465E92 || old_pc == 0x4665AC || old_pc == 0x466984 || old_pc == 0x464FAC || old_pc == 0x4618DE || old_pc == 0x461894 || old_pc == 0x41650A || old_pc == 0x416520 || old_pc == 0x47059A || old_pc == 0x4771F6 || old_pc == 0x47720E {
                    println!("[F{} L{:3}] >>> HIT ROUTINE PC=0x{:06X} | D0={:08X} D1={:08X} A6={:08X} (807E={:08X}, 91B6={:08X}, 932A={:04X}, C390={:08X})",
                        frame, _line, old_pc, cpu.d(0), cpu.d(1), cpu.a(6),
                        bus.read_long(cpu.a(6).wrapping_add(0xFFFF807E)),
                        bus.read_long(cpu.a(6).wrapping_add(0xFFFF91B6)),
                        bus.read_word(cpu.a(6).wrapping_add(0xFFFF932A)),
                        bus.read_long(cpu.a(6).wrapping_add(0xFFFFC390)),
                    );
                }

                let mut trace_fork = false;
                match batch.exit {
                    m68k::CycleBatchExit::TrapInstruction { trap_num } => {
                        let call_pc = cpu.ppc;
                        let sys_code = bus.read_word(call_pc + 2);
                        if (535..=665).contains(&frame) || (frame >= 740 && sys_code != 0x000A) {
                            let mut str_a0 = String::new();
                            for i in 0..20 {
                                let b = bus.read_byte(cpu.a(0).wrapping_add(i));
                                if b >= 0x20 && b <= 0x7E { str_a0.push(b as char); } else { break; }
                            }
                            let a6 = cpu.a(6);
                            println!("[F{} L{:3}] TRAP #{}: code=0x{:04X}, PC=0x{:06X}, SP=0x{:08X}, A6=0x{:08X} (807E={:04X}, C390={:04X}), D0=0x{:08X}, D1=0x{:08X}, A0=0x{:08X} ('{}')",
                                frame, _line, trap_num, sys_code, call_pc, cpu.a(7), a6,
                                bus.read_word(a6.wrapping_add(0x807E)),
                                bus.read_word(a6.wrapping_add(0xC390)),
                                cpu.d(0), cpu.d(1), cpu.a(0), str_a0);
                        }
                        if frame >= 1600 && frame <= 1605 {
                            let mut str_a0 = String::new();
                            for i in 0..32 {
                                let b = bus.read_byte(cpu.a(0).wrapping_add(i));
                                if b >= 0x20 && b <= 0x7E { str_a0.push(b as char); } else { break; }
                            }
                            let mut str_a1 = String::new();
                            for i in 0..32 {
                                let b = bus.read_byte(cpu.a(1).wrapping_add(i));
                                if b >= 0x20 && b <= 0x7E { str_a1.push(b as char); } else { break; }
                            }
                            println!("  >>> [TRAP DETAIL F{} L{:3}] code=0x{:04X}, PC=0x{:06X}, D0=0x{:08X}, D1=0x{:08X}, D2=0x{:08X}, D3=0x{:08X}, A0=0x{:08X} ('{}'), A1=0x{:08X} ('{}')",
                                frame, _line, sys_code, call_pc, cpu.d(0), cpu.d(1), cpu.d(2), cpu.d(3), cpu.a(0), str_a0, cpu.a(1), str_a1);
                        }
                        if sys_code == 0x0003 { // F$Fork
                            let mut prog = String::new();
                            for i in 0..32 {
                                let b = bus.read_byte(cpu.a(0) + i);
                                if b == 0 || b == b' ' || b == b'\r' { break; }
                                prog.push(b as char);
                            }
                            let mut params = String::new();
                            for i in 0..32 {
                                let b = bus.read_byte(cpu.a(1) + i);
                                if b == 0 || b == b'\r' { break; }
                                params.push(b as char);
                            }
                            println!("[F{} L{:3}] >>> F$FORK: prog='{}', params='{}' (caller PC=0x{:06X})",
                                frame, _line, prog, params, call_pc);
                            if prog == "play" {
                                trace_fork = true;
                            }
                        } else if sys_code == 0x0006 { // F$Exit
                            println!("[F{} L{:3}] >>> F$EXIT: status=0x{:04X} (caller PC=0x{:06X})",
                                frame, _line, cpu.d(1) as u16, call_pc);
                        } else if sys_code == 0x0004 { // F$Wait
                            println!("[F{} L{:3}] >>> F$WAIT: caller PC=0x{:06X}", frame, _line, call_pc);
                        } else if sys_code == 0x0008 { // F$Send
                            println!("[F{} L{:3}] >>> F$SEND: caller PC=0x{:06X}, D0=0x{:08X}, D1=0x{:08X}", frame, _line, call_pc, cpu.d(0), cpu.d(1));
                        } else if sys_code == 0x008E && call_pc == 0x475912 {
                            println!("[F{} L{:3}] >>> I$SETSTT 0x52: caller PC=0x{:06X}, D0=0x{:08X}, D1=0x{:08X}, D2=0x{:08X}, D3=0x{:08X}",
                                frame, _line, call_pc, cpu.d(0), cpu.d(1), cpu.d(2), cpu.d(3));
                        } else if sys_code == 0x008E && call_pc == 0x47578E {
                            let ret_caller = bus.read_long(cpu.a(7) + 24);
                            println!("[F{} L{:3}] >>> I$SETSTT 0x56: caller PC=0x{:06X}, ret_to=0x{:06X}, D0=0x{:08X}, D1=0x{:08X}, D2=0x{:08X}, D3=0x{:08X}, D4=0x{:08X}, D5=0x{:08X}",
                                frame, _line, call_pc, ret_caller, cpu.d(0), cpu.d(1), cpu.d(2), cpu.d(3), cpu.d(4), cpu.d(5));
                        }
                        let trap_cyc = cpu.take_trap_exception(&mut bus, trap_num);
                        line_cycles += trap_cyc;

                        if trace_fork {
                            println!("=== TRACING TRAP (caller=0x{:06X}) ===", call_pc);
                            let mut step = 0;
                            while step < 10000 {
                                let pc = cpu.pc;
                                bus.current_pc = pc;
                                let sp = cpu.a(7);
                                let sr = cpu.get_sr();
                                if pc == call_pc + 4 || pc == 0x40A0E6 || pc == 0x475916 || pc == 0x42AC64 || pc == 0xE000E8 || pc == 0xE00170 || pc == 0xE00172 {
                                    println!("--- TRAP RETURN/IDLE AT PC=0x{:06X} (step {}) ---", pc, step);
                                    println!("D: {:08X} {:08X} {:08X} {:08X} {:08X} {:08X} {:08X} {:08X}",
                                        cpu.d(0), cpu.d(1), cpu.d(2), cpu.d(3), cpu.d(4), cpu.d(5), cpu.d(6), cpu.d(7));
                                    println!("A: {:08X} {:08X} {:08X} {:08X} {:08X} {:08X} {:08X} {:08X}",
                                        cpu.a(0), cpu.a(1), cpu.a(2), cpu.a(3), cpu.a(4), cpu.a(5), cpu.a(6), cpu.a(7));
                                    println!("SR: 0x{:04X} (Carry={})", sr, (sr & 1) != 0);
                                    break;
                                }
                                let op = bus.read_word(pc);
                                let (dis, _) = m68k::dasm::disassemble(pc, op, m68k::CpuType::SCC68070);
                                if step <= 500 {
                                    println!("  [{:4}] PC=0x{:06X} {:04X} {:<24} | D0={:08X} D1={:08X} A0={:08X} A1={:08X} SP={:08X} SR={:04X}",
                                        step, pc, op, dis, cpu.d(0), cpu.d(1), cpu.a(0), cpu.a(1), sp, sr);
                                }
                                let b = cpu.run_for_cycles(&mut bus, 1);
                                match b.exit {
                                    m68k::CycleBatchExit::TrapInstruction { trap_num } => {
                                        cpu.take_trap_exception(&mut bus, trap_num);
                                    }
                                    m68k::CycleBatchExit::AlineTrap { .. } => { cpu.take_aline_exception(&mut bus); }
                                    m68k::CycleBatchExit::FlineTrap { .. } => { cpu.take_fline_exception(&mut bus); }
                                    m68k::CycleBatchExit::IllegalInstruction { .. } => { cpu.take_illegal_exception(&mut bus); }
                                    m68k::CycleBatchExit::Breakpoint { .. } => { cpu.take_bkpt_exception(&mut bus); }
                                    _ => {}
                                }
                                step += 1;
                            }
                        }
                    }
                    m68k::CycleBatchExit::AlineTrap { .. } => {
                        println!(">>> [ALINE TRAP] at PC=0x{:06X}", old_pc);
                        let trap_cyc = cpu.take_aline_exception(&mut bus);
                        line_cycles += trap_cyc;
                    }
                    m68k::CycleBatchExit::FlineTrap { .. } => {
                        println!(">>> [FLINE TRAP] at PC=0x{:06X}, opcode=0x{:04X}", old_pc, bus.read_word(old_pc));
                        let trap_cyc = cpu.take_fline_exception(&mut bus);
                        line_cycles += trap_cyc;
                    }
                    m68k::CycleBatchExit::IllegalInstruction { .. } => {
                        println!(">>> [ILLEGAL INSTRUCTION] at PC=0x{:06X}, opcode=0x{:04X}", old_pc, bus.read_word(old_pc));
                        let trap_cyc = cpu.take_illegal_exception(&mut bus);
                        line_cycles += trap_cyc;
                    }
                    m68k::CycleBatchExit::Breakpoint { .. } => {
                        println!(">>> [BREAKPOINT] at PC=0x{:06X}", old_pc);
                        let trap_cyc = cpu.take_bkpt_exception(&mut bus);
                        line_cycles += trap_cyc;
                    }
                    m68k::CycleBatchExit::Stopped => {
                        line_cycles = CYCLES_PER_SCANLINE as i32;
                        break;
                    }
                    _ => {}
                }

                if cpu.a(7) > 0xF000_0000 {
                    println!(">>> DETECTED SP corruption: SP=0x{:08X} at frame {}, line {}, PC=0x{:06X}", cpu.a(7), frame, _line, cpu.pc);
                    stopped = true;
                    break;
                }
            }

            let spent = (line_cycles as u32).max(CYCLES_PER_SCANLINE);
            bus.mcd212.tick_cycles(spent);
            bus.tick_cdic(spent);
            bus.tick_timer(spent);

            if stopped {
                break;
            }
        }
        if stopped {
            break;
        }

        if frame <= 10 || frame % 30 == 0 {
            let trap0 = u32::from_be_bytes([
                bus.mcd212.plane_a[0x80],
                bus.mcd212.plane_a[0x81],
                bus.mcd212.plane_a[0x82],
                bus.mcd212.plane_a[0x83],
            ]);
            println!(
                "Frame {:3}: PC=0x{:06X}, SP=0x{:08X}, Instr={:8}, TRAP#0=0x{:08X}",
                frame, cpu.pc, cpu.a(7), total_instructions, trap0
            );
        }
    }

    println!("=== Test Completed Successfully ===");
    println!("Final PC: 0x{:08X}, Total Instructions: {}", cpu.pc, total_instructions);
    print!("D: ");
    for i in 0..8 { print!("{:08X} ", cpu.d(i)); }
    println!();
    print!("A: ");
    for i in 0..8 { print!("{:08X} ", cpu.a(i)); }
    println!();
    println!("SP: 0x{:08X}", cpu.a(7));

    println!("RAM at 0x000060 (Autovectors):");
    for row in 0..4 {
        let base = 0x60 + row * 16;
        print!("  0x{:06X}: ", base);
        for b in &bus.mcd212.plane_a[base..base + 16] {
            print!("{:02X} ", b);
        }
        println!();
    }

    println!("RAM at 0x0000E0 (Vectors 0x38..0x43, including 0x3D at 0xF4, 0x3E at 0xF8):");
    for row in 0..4 {
        let base = 0x0E0 + row * 16;
        print!("  0x{:06X}: ", base);
        for b in &bus.mcd212.plane_a[base..base + 16] {
            print!("{:02X} ", b);
        }
        println!();
    }

    let sp = cpu.a(7) as usize;
    println!("Stack around SP (0x{:08X}):", cpu.a(7));
    if sp >= 32 && sp + 64 <= bus.mcd212.plane_a.len() {
        for row in 0..6 {
            let base = (sp - 16) + row * 16;
            print!("  0x{:06X}: ", base);
            for b in &bus.mcd212.plane_a[base..base + 16] {
                print!("{:02X} ", b);
            }
            println!();
        }
    }

    println!("RAM at 0x00062C (TRAP #0 entry point):");
    for row in 0..4 {
        let base = 0x62C + row * 16;
        print!("  0x{:06X}: ", base);
        for b in &bus.mcd212.plane_a[base..base + 16] {
            print!("{:02X} ", b);
        }
        println!();
    }

    println!("RAM at 0x002500:");
    for row in 0..4 {
        let base = 0x2500 + row * 16;
        print!("  0x{:06X}: ", base);
        for b in &bus.mcd212.plane_a[base..base + 16] {
            print!("{:02X} ", b);
        }
        println!();
    }

    println!("RAM at 0x003D00:");
    for row in 0..4 {
        let base = 0x3D00 + row * 16;
        print!("  0x{:06X}: ", base);
        for b in &bus.mcd212.plane_a[base..base + 16] {
            print!("{:02X} ", b);
        }
        println!();
    }
    println!("RAM at 0x003FA0 (Free list heads):");
    for row in 0..4 {
        let base = 0x3FA0 + row * 16;
        print!("  0x{:06X}: ", base);
        for b in &bus.mcd212.plane_a[base..base + 16] {
            print!("{:02X} ", b);
        }
        println!();
    }

    println!("DVC RAM2 at 0x00E80000:");
    for row in 0..4 {
        let base = row * 16;
        print!("  0x{:06X}: ", 0xE80000 + base);
        for b in &bus.dvc.ram2[base..base + 16] {
            print!("{:02X} ", b);
        }
        println!();
    }

    println!("DVC RAM1 at 0x00D00000:");
    for row in 0..4 {
        let base = row * 16;
        print!("  0x{:06X}: ", 0xD00000 + base);
        for b in &bus.dvc.ram1[base..base + 16] {
            print!("{:02X} ", b);
        }
        println!();
    }

    for i in (0..bus.mcd212.plane_a.len() - 4).step_by(2) {
        let val = bus.read_long(i as u32);
        if val == 0x00417020 {
            println!("Found 0x00417020 in plane_a at 0x{:06X}", i);
        }
        if val == 0x00417EB8 {
            println!("Found ISR 0x00417EB8 in plane_a at 0x{:06X}", i);
        }
    }
    for i in (0..bus.mcd212.plane_b.len() - 4).step_by(2) {
        let val = bus.read_long(0x200000 + i as u32);
        if val == 0x00417EB8 {
            println!("Found ISR 0x00417EB8 in plane_b at 0x{:06X}", 0x200000 + i);
        }
    }
    for i in (0..bus.sys_ram.len() - 4).step_by(2) {
        let val = bus.read_long(0x500000 + i as u32);
        if val == 0x00417EB8 {
            println!("Found ISR 0x00417EB8 in sys_ram at 0x{:06X}", 0x500000 + i);
        }
    }
    println!("RAM at A6 (0x{:08X}):", cpu.a(6));
    let a6 = cpu.a(6) as usize;
    if a6 + 128 <= bus.mcd212.plane_a.len() {
        for row in 0..6 {
            let base = a6 + row * 16;
            print!("  0x{:06X}: ", base);
            for b in &bus.mcd212.plane_a[base..base + 16] {
                print!("{:02X} ", b);
            }
            println!();
        }
    }

    let play_a6 = 0x0000C000u32;
    println!("Play state at A6=0xC000: 807E={:08X}, 8022={:08X}, C390={:08X}, 9172={:08X}, 932A={:04X}",
        bus.read_long(play_a6.wrapping_add(0xFFFF807E)),
        bus.read_long(play_a6.wrapping_add(0xFFFF8022)),
        bus.read_long(play_a6.wrapping_add(0xFFFFC390)),
        bus.read_long(play_a6.wrapping_add(0xFFFF9172)),
        bus.read_word(play_a6.wrapping_add(0xFFFF932A)),
    );

    // Save framebuffer as BMP in artifact directory so view_file can display it
    let artifact_bmp_path = "C:/Users/Gian/.gemini/antigravity/brain/47421117-6e2a-4536-8928-519a0c9b76ed/framebuffer_boot.bmp";
    save_bmp(&bus, artifact_bmp_path)?;

    Ok(())
}

fn save_bmp(bus: &bus::CdiBus, path: &str) -> std::io::Result<()> {
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
    bmp.extend_from_slice(&1u16.to_le_bytes()); // planes
    bmp.extend_from_slice(&32u16.to_le_bytes()); // 32 bpp
    bmp.extend_from_slice(&0u32.to_le_bytes()); // BI_RGB
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
