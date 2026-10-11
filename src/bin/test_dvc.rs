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
    println!("Saved framebuffer BMP: {:?}", path);
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();

    let args: Vec<String> = env::args().collect();
    let target = args.get(1).map(|s| s.as_str()).unwrap_or("7th");
    let max_frames: u32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(2500);

    println!("==================================================");
    println!("=== OpenCDi - Test DVC (Digital Video Cartridge) ===");
    println!("=== Target: {} | Max Frames: {} ===", target, max_frames);
    println!("==================================================");

    let mut bus = CdiBus::new();

    // 1. BIOS ROM
    let bios_path = find_file("bios/cdi220b.rom").expect("cdi220b.rom non trovata");
    bus.load_rom(&bios_path)?;
    println!("Caricato BIOS: {:?}", bios_path);
    println!("ROM[0..8]: {:02X?}", &bus.rom[0..8]);
    println!("plane_a[0..8]: {:02X?}", &bus.mcd212.plane_a[0..8]);
    let target_rom_off = 0x02A44Au32;
    for p in (0..target_rom_off as usize).rev() {
        if bus.rom[p] == 0x4A && bus.rom[p + 1] == 0xFC {
            let name_off = ((bus.rom[p + 12] as usize) << 24) | ((bus.rom[p + 13] as usize) << 16) | ((bus.rom[p + 14] as usize) << 8) | (bus.rom[p + 15] as usize);
            let name_start = p + name_off;
            let mut name = String::new();
            for k in 0..32 {
                if name_start + k >= bus.rom.len() { break; }
                let b = bus.rom[name_start + k];
                name.push((b & 0x7F) as char);
                if (b & 0x80) != 0 { break; }
            }
            println!("Module containing 0x42A44A: '{}' at 0x{:06X}", name, 0x400000 + p);
            break;
        }
    }

    // 2. NVRAM
    if let Some(nvram_path) = find_file("bios/nvram_mk48t08.bin") {
        let _ = bus.nvram.load_from_file(&nvram_path);
        println!("Caricata NVRAM: {:?}", nvram_path);
    }

    // 3. DVC ROM (vmpega.rom)
    let dvc_path = find_file("dvc/vmpega.rom").expect("dvc/vmpega.rom non trovata");
    bus.load_dvc_rom(&dvc_path)?;
    println!("Caricata DVC ROM: {:?}", dvc_path);

    // 4. Game Disc
    let game_subpath = match target {
        "7th" | "guest" => Some("giochi/7th Guest, The (USA).chd"),
        "lair" | "dragon" => Some("giochi/Dragon's Lair (USA).chd"),
        "eden" | "lost" => Some("giochi/Lost Eden (Europe).chd"),
        "mario" => Some("giochi/Hotel Mario (USA).chd"),
        "none" | "bios" => None,
        other => Some(other),
    };

    if let Some(gp) = game_subpath {
        let game_path = find_file(gp).unwrap_or_else(|| PathBuf::from(gp));
        if game_path.exists() {
            bus.load_cdrom(&game_path)?;
            println!("Caricato disco gioco: {:?}", game_path);
            // Keep default PAL (cdi220b is a European PAL BIOS)
        } else {
            eprintln!("ATTENZIONE: Gioco {:?} non trovato!", game_path);
        }
    }

    // 5. Init CPU
    let mut cpu = CpuCore::new();
    cpu.set_cpu_type(CpuType::SCC68070);
    cpu.reset(&mut bus);
    println!("CPU Reset OK - PC iniziale: 0x{:08X}", cpu.pc);

    let cyc_per_line = CYCLES_PER_SCANLINE;
    let mut cycle_debt: i32 = 0;
    let mut play_a6 = 0x0000C000u32;
    let mut disc_launched = false;
    let mut shell_ready_count = 0;
    let mut loop_disassembled = false;
    bus.watch_addr1 = 0x00005172;
    bus.watch_addr2 = 0x00005174;
    bus.watch_enabled = true;

    for frame in 1..=max_frames {
        // Frame-based clicks commented out in favor of dynamic autoboot

        if bus.mcd212.plane_a[0] != 0 {
            println!("ALERT: plane_a[0] is 0x{:02X} at frame {}", bus.mcd212.plane_a[0], frame);
        }
        let total_lines = bus.mcd212.video_mode.total_scanlines();
        let cyc_per_line = bus.mcd212.video_mode.cycles_per_scanline();
        let mut cycle_debt: i32 = 0;
        for _line in 0..total_lines {
            bus.tick_scanline(_line);

            if bus.mcd212.vblank_irq {
                bus.scc_timer.lir |= 0x80;
            }

            let int1_lvl = if (bus.scc_timer.lir & 0x80) != 0 {
                let prio = (bus.scc_timer.lir >> 4) & 7;
                if prio != 0 { prio } else { 5 }
            } else {
                0
            };

            let timer_lvl = if bus.scc_timer.timer_irq {
                bus.scc_timer.picr1 & 7
            } else {
                0
            };

            let cdic_lvl = if bus.cdic.irq_pending || bus.dvc.irq_pending() {
                if frame == 1335 && _line == 0 {
                    println!("[Frame 1335] DVC IRQ: cdic={}, dvc_fma_isr=0x{:04X} fma_ier=0x{:04X} ivec=0x{:04X} | fmv_isr=0x{:04X} fmv_ier=0x{:04X} ivec=0x{:04X}",
                        bus.cdic.irq_pending, bus.dvc.fma_isr, bus.dvc.fma_ier, bus.dvc.fma_ivec,
                        bus.dvc.fmv_isr, bus.dvc.fmv_ier, bus.dvc.fmv_ivec);
                }
                4
            } else { 0 };
            let slave_lvl = if bus.slave.irq_pending { 2 } else { 0 };

            let highest_ipl = int1_lvl.max(timer_lvl).max(cdic_lvl).max(slave_lvl);
            cpu.set_irq(highest_ipl);

            let mut line_cycles = -cycle_debt;

            while line_cycles < cyc_per_line as i32 {
                let remaining = (cyc_per_line as i32 - line_cycles).max(1);
                let budget = remaining.min(32);

                bus.current_pc = cpu.pc;
                bus.current_line_cycle = line_cycles.max(0) as u32;

                if frame >= 1353 && frame <= 1354 && cpu.pc >= 0x200000 && cpu.pc < 0x300000 {
                    let opcode = bus.read_word(cpu.pc);
                    let (text, _) = m68k::dasm::disassemble(cpu.pc, opcode, m68k::CpuType::SCC68070);
                    println!("[Trace {}] PC=0x{:06X}: {}", frame, cpu.pc, text);
                }
                if frame >= 1840 && frame <= 1844 && (cpu.pc == 0x42A442 || cpu.pc == 0x42A446 || cpu.pc == 0x42A454 || cpu.pc == 0x42A4DE || cpu.pc == 0x42A520 || cpu.pc == 0x42A52A || cpu.pc == 0x42A53A) {
                    println!("[Frame {}] PC=0x{:06X}: D0=0x{:08X}, A2=0x{:08X} ($94=0x{:04X}), A4=0x{:08X} ($04=0x{:08X}, $01=0x{:02X})",
                        frame, cpu.pc, cpu.d(0), cpu.a(2),
                        if cpu.a(2) < 0xE00000 { bus.read_word(cpu.a(2) + 0x94) } else { 0 },
                        cpu.a(4),
                        if cpu.a(4) < 0xE00000 { bus.read_long(cpu.a(4) + 4) } else { 0 },
                        if cpu.a(4) < 0xE00000 { bus.read_byte(cpu.a(4) + 1) } else { 0 },
                    );
                }
                if frame >= 1845 && (cpu.pc >= 0x2729A0 && cpu.pc <= 0x2729E0) && !loop_disassembled {
                    loop_disassembled = true;
                    println!("=== DISASSEMBLY OF LOOP AT 0x2729AA (Frame {}) ===", frame);
                    println!("PC=0x{:06X}", cpu.pc);
                    println!("D: {:08X} {:08X} {:08X} {:08X} {:08X} {:08X} {:08X} {:08X}",
                        cpu.d(0), cpu.d(1), cpu.d(2), cpu.d(3), cpu.d(4), cpu.d(5), cpu.d(6), cpu.d(7));
                    println!("A: {:08X} {:08X} {:08X} {:08X} {:08X} {:08X} {:08X} {:08X}",
                        cpu.a(0), cpu.a(1), cpu.a(2), cpu.a(3), cpu.a(4), cpu.a(5), cpu.a(6), cpu.a(7));
                    let mut p = 0x272990;
                    while p < 0x272A00 {
                        let op = bus.read_word(p);
                        let (dis, len) = m68k::dasm::disassemble(p, op, m68k::CpuType::SCC68070);
                        let mut bytes_str = String::new();
                        let l = (len as u32).max(2);
                        for i in 0..l {
                            bytes_str.push_str(&format!("{:02X} ", bus.read_byte(p + i)));
                        }
                        println!("  0x{:06X}: {:<16} {}", p, bytes_str, dis);
                        p += l;
                    }
                    println!("--- 50 STEP TRACE ---");
                    for step in 0..50 {
                        let opcode = bus.read_word(cpu.pc);
                        let (text, _) = m68k::dasm::disassemble(cpu.pc, opcode, m68k::CpuType::SCC68070);
                        println!("  step {:02}: PC=0x{:06X} ({}) | D0=0x{:X} D1=0x{:X} A0=0x{:X} A1=0x{:X} A2=0x{:X} A3=0x{:X} A4=0x{:X}",
                            step, cpu.pc, text, cpu.d(0), cpu.d(1), cpu.a(0), cpu.a(1), cpu.a(2), cpu.a(3), cpu.a(4));
                        let b = cpu.run_for_cycles(&mut bus, 1);
                        if let m68k::CycleBatchExit::TrapInstruction { trap_num: t } = b.exit {
                            println!("    TRAP #{}", t);
                            cpu.take_trap_exception(&mut bus, t);
                        }
                    }
                }
                let prev_pc = cpu.pc;
                let batch = cpu.run_for_cycles(&mut bus, budget);
                line_cycles += batch.cycles + bus.consume_wait_cycles();

                if cpu.pc >= 0x00461614 && cpu.pc <= 0x00479C0E {
                    let cur_a6 = cpu.a(6);
                    if cur_a6 != play_a6 && cur_a6 > 0x1000 {
                        play_a6 = cur_a6;
                    }
                }

                match batch.exit {
                    m68k::CycleBatchExit::TrapInstruction { trap_num } => {
                        let next_word = bus.read_word(cpu.pc);
                        if (target == "eden" && frame >= 400 && frame <= 1200) || (frame >= 1350 && frame <= 1600) {
                            if trap_num == 0 {
                                let mut name_str = String::new();
                                if next_word == 0x0080 || next_word == 0x0081 || next_word == 0x0082 || next_word == 0x0084 || next_word == 0x0000 {
                                    let a0 = cpu.a(0);
                                    for i in 0..32 {
                                        let b = bus.read_byte(a0 + i);
                                        let c = (b & 0x7F) as char;
                                        if b == 0 || b == b' ' || b == b'\r' || b == b'\n' { break; }
                                        name_str.push(c);
                                        if (b & 0x80) != 0 { break; }
                                    }
                                }
                                if next_word == 0x0081 && name_str.is_empty() {
                                    let mut dump = String::new();
                                    for i in 0..16 {
                                        dump.push_str(&format!("{:02X} ", bus.read_byte(cpu.a(0) + i)));
                                    }
                                    name_str = format!("hex: {}", dump);
                                }
                                let sp = cpu.a(7);
                                let ret_addr = bus.read_long(sp);
                                println!("[Frame {}] TRAP #{}: code=0x{:04X}, PC=0x{:06X}, ret=0x{:06X}, D0=0x{:X}, D1=0x{:X}, A0=0x{:X} ('{}'), A1=0x{:X}, A2=0x{:X}",
                                    frame, trap_num, next_word, cpu.pc, ret_addr, cpu.d(0), cpu.d(1), cpu.a(0), name_str, cpu.a(1), cpu.a(2));
                            }
                        }
                        let trap_cyc = cpu.take_trap_exception(&mut bus, trap_num);
                        line_cycles += trap_cyc;
                        if (frame == 553 || frame == 855) && next_word == 0x000A {
                            println!(">>> [Frame {}] RAM[0x78] (Autovector 6) = 0x{:08X}, RAM[0xF8] (Vector 62) = 0x{:08X}",
                                frame, bus.read_long(0x78), bus.read_long(0xF8));
                            println!(">>> Tracing 100 instructions from F$Sleep at frame 855, PC=0x{:06X}", cpu.pc);
                            for step in 0..100 {
                                let opcode = bus.read_word(cpu.pc);
                                let (text, _) = m68k::dasm::disassemble(cpu.pc, opcode, m68k::CpuType::SCC68070);
                                println!("  [{:02}] PC=0x{:06X}: {} | D0={:X} D1={:X} SP={:X}", step, cpu.pc, text, cpu.d(0), cpu.d(1), cpu.a(7));
                                let b = cpu.run_for_cycles(&mut bus, 1);
                                if let m68k::CycleBatchExit::TrapInstruction { trap_num: t } = b.exit {
                                    println!("    nested TRAP #{}", t);
                                    cpu.take_trap_exception(&mut bus, t);
                                }
                            }
                        }
                        if frame == 1353 && next_word == 0x0056 {
                            println!(">>> Tracing 60 instructions of OS-9 I$SetStt handler from PC=0x{:06X}", cpu.pc);
                            for step in 0..60 {
                                let opcode = bus.read_word(cpu.pc);
                                let (text, _) = m68k::dasm::disassemble(cpu.pc, opcode, m68k::CpuType::SCC68070);
                                println!("  [{:02}] PC=0x{:06X}: {}", step, cpu.pc, text);
                                let b = cpu.run_for_cycles(&mut bus, 1);
                                if let m68k::CycleBatchExit::TrapInstruction { trap_num: t } = b.exit {
                                    println!("    nested TRAP #{}", t);
                                    cpu.take_trap_exception(&mut bus, t);
                                }
                            }
                        }
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

            cycle_debt = (line_cycles - cyc_per_line as i32).clamp(0, 100);

            bus.tick_slave(cyc_per_line);
            bus.tick_cdic(cyc_per_line);
            bus.tick_dvc(cyc_per_line);
            bus.tick_timer(cyc_per_line);
        }

        bus.trace_fmv = frame >= 1838 && frame <= 1845;

        if frame >= 1838 && frame <= 1848 {
            println!(
                "[Frame {:4}] PC=0x{:06X} | CDIC: LBA={}, disc_cmd=0x{:02X}, cmd=0x{:04X}, xbuf=0x{:04X} | DVC: isr=0x{:04X}, ier=0x{:04X}, fifo={}, decoded={}",
                frame, cpu.pc, bus.cdic.curr_lba, bus.cdic.disc_command, bus.cdic.command, bus.cdic.x_buffer,
                bus.dvc.fmv_isr, bus.dvc.fmv_ier, bus.dvc.dvc_stream_fifo.len(), bus.dvc.frames_decoded_count
            );
        }

        if target == "eden" && frame >= 855 && frame <= 950 && frame % 10 == 0 {
            println!("[Frame {:4}] PC=0x{:06X} | timer_irq={}, picr1=0x{:02X}, timer0=0x{:04X}, tsr=0x{:02X}, LIR=0x{:02X}, SR=0x{:04X}",
                frame, cpu.pc, bus.scc_timer.timer_irq, bus.scc_timer.picr1, bus.scc_timer.timer0, bus.scc_timer.tsr, bus.scc_timer.lir, cpu.get_sr());
        }

        // Periodic Status Logging
        if frame % 100 == 0 || frame == 865 {
            let state_9172 = bus.read_long(play_a6.wrapping_add(0xFFFF9172));
            let c390 = bus.read_long(play_a6.wrapping_add(0xFFFFC390));
            println!(
                "[Frame {:4}] PC=0x{:06X} play_a6=0x{:08X} | 9172=0x{:X}, C390=0x{:X} | DVC: reg_w={}, reg_r={}, fifo_len={}, decoded={} | MCD212: ICM=0x{:08X}, TCR=0x{:08X}, ord={}, vsr=[0x{:04X}, 0x{:04X}]",
                frame, cpu.pc, play_a6, state_9172, c390,
                bus.dvc.reg_write_count, bus.dvc.reg_read_count, bus.dvc.dvc_stream_fifo.len(), bus.dvc.frames_decoded_count,
                bus.mcd212.image_coding_method, bus.mcd212.transparency_control, bus.mcd212.plane_order,
                bus.mcd212.vsr[0], bus.mcd212.vsr[1]
            );
            if !bus.dvc.dvc_stream_fifo.is_empty() {
                let dump_len = bus.dvc.dvc_stream_fifo.len().min(64);
                println!(">>> DVC STREAM FIFO FIRST {} BYTES: {:02X?}", dump_len, &bus.dvc.dvc_stream_fifo[..dump_len]);
            }
            if frame == 1400 {
                let opcode = bus.read_word(cpu.pc);
                let (text, _) = m68k::dasm::disassemble(cpu.pc, opcode, m68k::CpuType::SCC68070);
                println!("Frame 1400: PC=0x{:06X}, SP=0x{:08X}, instr: {}", cpu.pc, cpu.a(7), text);
                println!("D: {:08X} {:08X} {:08X} {:08X} {:08X} {:08X} {:08X} {:08X}",
                    cpu.d(0), cpu.d(1), cpu.d(2), cpu.d(3), cpu.d(4), cpu.d(5), cpu.d(6), cpu.d(7));
                println!("A: {:08X} {:08X} {:08X} {:08X} {:08X} {:08X} {:08X} {:08X}",
                    cpu.a(0), cpu.a(1), cpu.a(2), cpu.a(3), cpu.a(4), cpu.a(5), cpu.a(6), cpu.a(7));
                println!("plane_a[0..4] = {:02X?}", &bus.mcd212.plane_a[0..4]);
                println!("bus.read_long(0) = 0x{:08X}", bus.read_long(0));
                println!("Stack[SP..SP+16]: {:08X} {:08X} {:08X} {:08X}",
                    bus.read_long(cpu.a(7)), bus.read_long(cpu.a(7) + 4),
                    bus.read_long(cpu.a(7) + 8), bus.read_long(cpu.a(7) + 12));
                println!("Dump of 0x00DFE500:");
                for row in 0..4 {
                    let base = 0x00DFE500 + row * 16;
                    print!("  0x{:08X}: ", base);
                    for col in 0..4 {
                        print!("{:08X} ", bus.read_long(base + col * 4));
                    }
                    println!();
                }
                println!("Code around 0x406DC0:");
                for p in (0x406DC0..0x406DF4).step_by(2) {
                    print!("{:04X} ", bus.read_word(p));
                }
                println!();
            }
            if frame == 1200 {
                let off_b = (bus.mcd212.vsr[1] as usize) & 0x7FFFF;
                let off_a = (bus.mcd212.vsr[0] as usize) & 0x7FFFF;
                println!("Plane A at 0x{:04X}: {:02X?}", off_a, &bus.mcd212.plane_a[off_a..off_a + 32]);
                println!("Plane B at 0x{:04X}: {:02X?}", off_b, &bus.mcd212.plane_b[off_b..off_b + 32]);
                println!("Weight factors: wa[0]={}, wb[0]={}", bus.mcd212.weight_factor[0][0], bus.mcd212.weight_factor[1][0]);
                println!("CLUT [0..8]: {:08X?}", &bus.mcd212.clut[..8]);
                println!("Transparency control: 0x{:08X}, Plane Order: 0x{:08X}", bus.mcd212.transparency_control, bus.mcd212.plane_order);
                let nz_ram2 = bus.dvc.ram2.iter().filter(|&&b| b != 0).count();
                println!("DVC Picture DRAM (ram2) non-zero bytes: {} / {}", nz_ram2, bus.dvc.ram2.len());
                println!("DVC fmv_playback_active={}, fmv_decoder_active={}, fifo_len={}",
                    bus.dvc.fmv_playback_active, bus.dvc.fmv_decoder_active, bus.dvc.dvc_stream_fifo.len());
            }
        }

            if frame == 2000 && target == "7th" {
                println!("=== Disassembly at Frame 2000 (PC=0x{:06X}) ===", cpu.pc);
                let mut p = cpu.pc.saturating_sub(16);
                while p < cpu.pc + 40 {
                    let opcode = bus.read_word(p);
                    let (text, len) = m68k::dasm::disassemble(p, opcode, m68k::CpuType::SCC68070);
                    let l = len.max(2);
                    println!("  {}{:06X}: {}", if p == cpu.pc { "-> " } else { "   " }, p, text);
                    p += l as u32;
                }
            }
        if bus.cdrom.is_some() && !disc_launched {
            let (effective_shell_base, shell_state) = if bus.dvc.is_fitted() {
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
                    println!(">>> [Frame {}] Shell BIOS pronta (stato {}): click PLAY CD-I...", frame, shell_state);
                    bus.slave.update_mouse(575, 335, 1);
                } else if shell_ready_count == 50 || shell_ready_count == 200 {
                    println!(">>> [Frame {}] Releasing click at (575, 335)", frame);
                    bus.slave.update_mouse(575, 335, 0);
                }
            }
            if ((cpu.pc >= 0x00010000 && cpu.pc < 0x00300000) || (cpu.pc >= 0x00D00000 && cpu.pc < 0x00E00000)) && frame > 250 {
                println!(">>> [Frame {}] Applicazione disco già in RAM (PC=0x{:06X})!", frame, cpu.pc);
                disc_launched = true;
                bus.slave.update_mouse(384, 280, 0);
            }
        }

        if frame == 2700 && target == "lair" {
            println!(">>> [Frame {}] USER PRESSES 'BUTTON 1' ON TITLE SCREEN!", frame);
            bus.slave.update_mouse(384, 280, 1);
        } else if frame == 2720 && target == "lair" {
            println!(">>> [Frame {}] USER RELEASES 'BUTTON 1'!", frame);
            bus.slave.update_mouse(384, 280, 0);
        }

        if frame == 2100 && target == "7th" {
            println!(">>> [Frame {}] USER CLICKS MOUSE TO SKIP/PROCEED IN 7TH GUEST!", frame);
            bus.slave.update_mouse(384, 280, 1);
        } else if frame == 2120 && target == "7th" {
            println!(">>> [Frame {}] USER RELEASES MOUSE CLICK IN 7TH GUEST!", frame);
            bus.slave.update_mouse(384, 280, 0);
        }

        if frame == 3500 && target == "eden" {
            println!(">>> [Frame {}] USER CLICKS MOUSE TO SKIP INTRO IN LOST EDEN!", frame);
            bus.slave.update_mouse(384, 280, 1);
        } else if frame == 3520 && target == "eden" {
            println!(">>> [Frame {}] USER RELEASES MOUSE CLICK IN LOST EDEN!", frame);
            bus.slave.update_mouse(384, 280, 0);
        }

        if frame == 5600 && target == "eden" {
            println!(">>> [Frame {}] USER CLICKS MOUSE ON TITLE SCREEN IN LOST EDEN!", frame);
            bus.slave.update_mouse(384, 280, 1);
        } else if frame == 5620 && target == "eden" {
            println!(">>> [Frame {}] USER RELEASES MOUSE CLICK IN LOST EDEN!", frame);
            bus.slave.update_mouse(384, 280, 0);
        }

        // Save screenshots at checkpoints
        if frame == 800 || frame == 1200 || frame == 1600 || (frame >= 1780 && frame <= 1850 && frame % 10 == 0) || frame == 2000 || frame == 2100 || frame == 2150 || frame == 2200 || frame == 2300 || frame == 2400 || frame == 2600 || frame == 2800 || frame == 3000 || frame == 3200 || frame == 3400 || frame == 3600 || frame == 3800 || frame == 4000 || frame == 4500 || frame == 5000 || frame == 5500 || frame == 6000 || frame == 6500 || frame == 7000 || frame == max_frames {
            let bmp_path = format!(
                "C:/Users/Gian/.gemini/antigravity/brain/47421117-6e2a-4536-8928-519a0c9b76ed/dvc_{}_{}.bmp",
                target, frame
            );
            let _ = save_bmp(&bus, &bmp_path);
        }
    }

    let dump_ranges = [
        ("Shell state write around 0x004619AC", 0x00461980, 0x00461A00),
        ("FMV ISR poll loop around 0x00407FA8", 0x00407F90, 0x00407FE0),
        ("DVC timecode check around 0x00E539AE", 0x00E53970, 0x00E53A20),
        ("CDIC driver stop around 0x0042A454", 0x0042A410, 0x0042A4B0),
        ("CDIC read handler around 0x0042B05C", 0x0042B040, 0x0042B0D0),
        ("Stop/Reset around 0x00E51F3E", 0x00E51F20, 0x00E51F80),
        ("ISR Handler around 0x00E52F36", 0x00E52F20, 0x00E52F70),
        ("DMA complete / IER around 0x00E53D68", 0x00E53D50, 0x00E53DA0),
    ];
    for (title, start, end) in dump_ranges {
        println!("=== {} ===", title);
        let mut p = start;
        while p < end {
            let opcode = bus.read_word(p);
            let (text, len) = m68k::dasm::disassemble(p, opcode, m68k::CpuType::SCC68070);
            let l = len.max(2);
            print!("  0x{:06X}: ", p);
            for k in 0..(l as usize / 2) {
                print!("{:04X} ", bus.read_word(p + k as u32 * 2));
            }
            println!("  {}", text);
            p += l as u32;
        }
    }
    Ok(())
}
