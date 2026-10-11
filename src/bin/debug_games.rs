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
use crate::cdrom::CdRom;
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
    let _ = env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).try_init();
    let args: Vec<String> = std::env::args().collect();
    let game_name = if args.len() > 1 {
        &args[1]
    } else {
        "giochi/Dark Fables of Aesop The (Europe).chd"
    };

    println!("=== OpenCDi Debug Games: {} ===", game_name);

    let mut bus = CdiBus::new();
    let bios_path = find_file("bios/cdi220b.rom").expect("BIOS cdi220b.rom not found");
    bus.load_rom(&bios_path)?;

    if let Some(nvram_path) = find_file("bios/nvram_mk48t08.bin") {
        let _ = bus.nvram.load_from_file(&nvram_path);
    }
    if let Some(dvc_path) = find_file("dvc/vmpega.rom") {
        let _ = bus.load_dvc_rom(&dvc_path);
    }

    let games = [
        "giochi/7th Guest, The (USA).chd",
        "giochi/Dark Fables of Aesop The (Europe).chd",
        "giochi/Dragon's Lair (USA).chd",
        "giochi/Hotel Mario (USA).chd",
        "giochi/Lost Eden (Europe).chd",
        "giochi/Rise of the Robots (Europe).chd",
        "giochi/Striker Pro (USA).chd",
        "giochi/Tangram - The Ultimate Chinese Game (Europe).chd",
    ];

    println!("=== Checking Sector 16 for all games ===");
    for g in &games {
        if let Some(p) = find_file(g) {
            if let Ok(mut cd) = CdRom::open(&p) {
                let mut sec = [0u8; 2352];
                if cd.read_sector(16, &mut sec) {
                    let is_swapped = sec[0] == 0xFF && sec[1] == 0x00;
                    let mut s = sec;
                    if is_swapped {
                        for i in (0..2352).step_by(2) { s.swap(i, i+1); }
                    }
                    let is_sync = s[0] == 0x00 && s[1..11].iter().all(|&b| b == 0xFF) && s[11] == 0x00;
                    let msf_mode_raw = &s[12..16];
                    let mut desc = s;
                    for i in 12..2352 { desc[i] ^= bus.cdic.scramble_table[i]; }
                    let msf_mode_desc = &desc[12..16];
                    let ascii_raw: String = s[24..80].iter().map(|&c| if c >= 32 && c <= 126 { c as char } else { '.' }).collect();
                    let ascii_desc: String = desc[24..80].iter().map(|&c| if c >= 32 && c <= 126 { c as char } else { '.' }).collect();
                    println!("{: <40} | swap: {:<5} | sync: {:<5} | raw MSF: {:02X?} | desc MSF: {:02X?}", g, is_swapped, is_sync, msf_mode_raw, msf_mode_desc);
                    println!("    raw text:  {}", ascii_raw);
                    println!("    desc text: {}", ascii_desc);
                }
            }
        }
    }

    let game_path = find_file(game_name).expect("Game disc not found");
    bus.load_cdrom(&game_path)?;
    println!("Loaded CD-ROM for emulation: {:?}", game_path);

    if game_name.contains("USA") || game_name.contains("Japan") || game_name.contains("NTSC") {
        println!(">>> Setting VideoMode::Ntsc (60 Hz / 262 scanlines)");
        bus.set_video_mode(crate::mcd212::VideoMode::Ntsc);
    } else {
        println!(">>> Setting VideoMode::Pal (50 Hz / 312 scanlines)");
        bus.set_video_mode(crate::mcd212::VideoMode::Pal);
    }

    println!("Disassembly around 0x0042E8C0..0x0042E8F0:");
    let mut p = 0x0042E8C0u32;
    while p < 0x0042E8F0 {
        let op = bus.read_word(p);
        let (dis, len) = m68k::dasm::disassemble(p, op, m68k::CpuType::SCC68070);
        let bytes: Vec<String> = (0..len).map(|k| format!("{:02X}", bus.read_byte(p + k as u32))).collect();
        println!("0x{:06X}: {:<16} {:<24}", p, bytes.join(" "), dis);
        p += len as u32;
    }

    let mut cpu = CpuCore::new();
    cpu.set_cpu_type(CpuType::SCC68070);
    cpu.reset(&mut bus);

    let mut pc_history: Vec<(u32, u16)> = Vec::with_capacity(50);
    let mut panicked = false;
    let mut logged_dma = false;

    let is_mario = game_name.to_lowercase().contains("mario");
    let mut shell_ready_count: u32 = 0;

    let max_frames = if is_mario { 4200 } else if game_name.contains("Striker") { 7800 } else if game_name.contains("7th") { 7000 } else { 3500 };
    let game_tag = if is_mario { "mario" } else if game_name.contains("Striker") { "striker" } else if game_name.contains("Robots") { "robots" } else if game_name.contains("Tangram") { "tangram" } else if game_name.contains("Eden") { "eden" } else if game_name.contains("Aesop") { "aesop" } else if game_name.contains("7th") { "7th" } else { "game" };

    for frame in 1..=max_frames {
        if panicked {
            break;
        }
        let play_a6 = 0x0000C000u32;
        let shell_state = bus.read_long(play_a6.wrapping_add(0xFFFF9172));

        if shell_state == 2 {
            shell_ready_count += 1;
            if shell_ready_count == 30 {
                println!(">>> [Frame {}] Dynamic Mouse Click Down on PLAY CD-I at (575, 335)", frame);
                bus.slave.update_mouse(575, 335, 1);
            } else if shell_ready_count == 50 {
                println!(">>> [Frame {}] Dynamic Mouse Click Up on PLAY CD-I at (575, 335)", frame);
                bus.slave.update_mouse(575, 335, 0);
            } else if shell_ready_count >= 120 {
                println!(">>> [Frame {}] Fallback OS-9: Trigger Action 29 to launch disc application", frame);
                bus.write_long(play_a6.wrapping_add(0xFFFFC344), 29);
                shell_ready_count = 0;
            }
        }

        if bus.cdic.curr_lba >= 31540 && frame % 20 == 0 {
            println!(">>> [Frame {}] LBA={}, PC=0x{:08X}, CDIC cmd=0x{:02X}, XBUF=0x{:04X}, DBUF=0x{:04X}",
                frame, bus.cdic.curr_lba, cpu.pc, bus.cdic.command, bus.cdic.x_buffer, bus.cdic.data_buffer);
        }

        if frame == 7310 {
            println!("=== Interrupt Status at frame 7310 ===");
            println!("  timer_irq: {}, tsr: 0x{:02X}, tcr: 0x{:02X}, picr1: 0x{:02X}, picr2: 0x{:02X}",
                bus.scc_timer.timer_irq, bus.scc_timer.tsr, bus.scc_timer.tcr, bus.scc_timer.picr1, bus.scc_timer.picr2);
            println!("  cdic.irq_pending: {}, x_buffer: 0x{:04X}, audio_buffer: 0x{:04X}",
                bus.cdic.irq_pending, bus.cdic.x_buffer, bus.cdic.audio_buffer);
            println!("  slave.irq_pending: {}", bus.slave.irq_pending);
            println!("  mcd212.vblank_irq: {}", bus.mcd212.vblank_irq);
        }

        if is_mario {
            // Click to skip intro cutscene, advance title screen, and start stage 1
            if frame == 2200 { println!(">>> [Frame 2200] Mario skip cutscene"); bus.slave.update_mouse(384, 280, 1); }
            if frame == 2220 { bus.slave.update_mouse(384, 280, 0); }

            if frame == 2800 { println!(">>> [Frame 2800] Mario start game from title"); bus.slave.update_mouse(384, 280, 1); }
            if frame == 2820 { bus.slave.update_mouse(384, 280, 0); }

            if frame == 3900 { println!(">>> [Frame 3900] Mario enter stage 1"); bus.slave.update_mouse(384, 280, 1); }
            if frame == 3920 { bus.slave.update_mouse(384, 280, 0); }
        }

        if bus.cdic.dma_control == 0x8A04 && !logged_dma {
            logged_dma = true;
            println!(">>> [Frame {}] DMA control is 0x8A04! PC=0x{:08X}", frame, cpu.pc);
            println!("    DMA Ch0 TC={}, MAC=0x{:08X}, OCR=0x{:02X}, CSR=0x{:02X}, CCR=0x{:02X}",
                bus.dma.channels[0].transfer_counter,
                bus.dma.channels[0].memory_address_counter,
                bus.dma.channels[0].operation_control,
                bus.dma.channels[0].channel_status,
                bus.dma.channels[0].channel_control
            );
        }

        bus.slave.video_mode = bus.mcd212.video_mode;
        let total_lines = bus.mcd212.video_mode.total_scanlines();
        let cyc_per_line = bus.mcd212.video_mode.cycles_per_scanline();
        let mut cycle_debt: i32 = 0;
        for _line in 0..total_lines {
            bus.mcd212.tick_scanline(_line);

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

            let mut line_cycles = -cycle_debt;
            while line_cycles < cyc_per_line as i32 {
                let budget = (cyc_per_line as i32 - line_cycles).max(1);
                bus.current_pc = cpu.pc;
                if cpu.pc >= 0x004004B8 && cpu.pc <= 0x00400560 && cpu.pc != 0x004004B8 {
                    println!("\n>>> HIT ROM BOOT/ERROR HANDLER at PC=0x{:08X} at frame {} line {}", cpu.pc, frame, _line);
                    for i in 0..8 { print!("D{}: {:08X} ", i, cpu.d(i)); }
                    println!();
                    for i in 0..8 { print!("A{}: {:08X} ", i, cpu.a(i)); }
                    println!();
                    let sp = cpu.sp();
                    println!("SP: 0x{:08X}", sp);
                    for off in 0..16 { print!("{:04X} ", bus.read_word(sp.wrapping_add(off * 2))); }
                    println!("\n");
                }
                
                if cpu.pc == 0x00412756 || cpu.pc == 0x004129D8 {
                    println!("\n>>> HIT F$Send at PC=0x{:08X}! Target PID=D0={}, Signal=D1={}, SP=0x{:08X}",
                        cpu.pc, cpu.d(0), cpu.d(1), cpu.sp());
                    let sp = cpu.sp();
                    for off in 0..8 { print!("{:04X} ", bus.read_word(sp.wrapping_add(off * 2))); }
                    println!();
                }

                if cpu.pc == 0x00062C {
                    println!(">>> At 0x00062C: SP=0x{:08X}, SR=0x{:04X}, words: {:04X} {:04X} {:04X}",
                        cpu.sp(), cpu.get_sr(), bus.read_word(0x62C), bus.read_word(0x62E), bus.read_word(0x630));
                }

                if cpu.pc == 0x0042E8CC || cpu.pc == 0x0042E8D4 {
                    let ret = bus.read_long(cpu.sp());
                    println!("\n>>> HIT 0x42E8CC/D4! Return PC=0x{:08X}, D0=0x{:08X}, D1=0x{:08X}, A0=0x{:08X}", ret, cpu.d(0), cpu.d(1), cpu.a(0));
                }

                if (cpu.pc == 0x00400636 || cpu.pc == 0x00409452) && !panicked {
                    panicked = true;
                    println!("\n!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!");
                    println!("HIT CRASH/PANIC at PC=0x{:08X} at frame {} line {}", cpu.pc, frame, _line);
                    println!("Registers:");
                    for i in 0..8 {
                        print!("D{}: {:08X} ", i, cpu.d(i));
                    }
                    println!();
                    for i in 0..8 {
                        print!("A{}: {:08X} ", i, cpu.a(i));
                    }
                    println!("SP: {:08X}", cpu.sp());
                    println!("Stack content around SP:");
                    let sp = cpu.sp();
                    for off in 0..32 {
                        let addr = sp.wrapping_add(off * 2);
                        print!("{:04X} ", bus.read_word(addr));
                    }
                    println!();
                    println!("Last 30 instructions:");
                    for &(hist_pc, op) in &pc_history {
                        let (dis, _) = m68k::dasm::disassemble(hist_pc, op, m68k::CpuType::SCC68070);
                        println!("  0x{:06X}: {}", hist_pc, dis);
                    }
                    println!("!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!\n");
                    break;
                }

                let cur_op = bus.read_word(cpu.pc);
                if pc_history.len() >= 200 {
                    pc_history.remove(0);
                }
                pc_history.push((cpu.pc, cur_op));

                let batch = cpu.run_for_cycles(&mut bus, budget);
                line_cycles += batch.cycles;

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
                    m68k::CycleBatchExit::Stopped => {
                        line_cycles = cyc_per_line as i32;
                        break;
                    }
                    _ => {}
                }
            }

            let actual_cycles = (line_cycles + cycle_debt).max(0) as u32;
            cycle_debt = (line_cycles - cyc_per_line as i32).clamp(0, 100);

            bus.tick_cdic(actual_cycles);
            bus.tick_timer(actual_cycles);
        }

        if frame % 500 == 0 || frame == max_frames || (bus.cdic.curr_lba >= 31545 && frame % 100 == 0) {
            println!("[Frame {}] PC=0x{:08X}, shell_state={}, CDIC cmd=0x{:02X}, LBA={}",
                frame, cpu.pc, shell_state, bus.cdic.command, bus.cdic.curr_lba);
            let path = format!("C:/Users/Gian/.gemini/antigravity/brain/47421117-6e2a-4536-8928-519a0c9b76ed/debug_{}_{}.bmp", game_tag, frame);
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
            bmp.extend_from_slice(&(-height).to_le_bytes());
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
            let _ = std::fs::write(&path, &bmp);
        }
    }

    Ok(())
}
