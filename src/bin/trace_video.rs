//! trace_video: esegue un disco in modalita' headless replicando il loop di main.rs
//! e registra ogni cambiamento dello stato video MCD212 (con frame e riga), oltre a
//! un riepilogo per field. Non modifica il comportamento dell'emulatore.
//!
//! Uso:
//!   trace_video <disco> [--from N] [--to M] [--out DIR] [--dump f1,f2,..] [--autodump]
//!               [--press frame:btn:durata,...] [--quiet-lines]
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

use crate::bus::CdiBus;
use m68k::{AddressBus, CpuCore, CpuType};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

fn find_file(relative_subpath: &str) -> Option<PathBuf> {
    let candidates = [
        Path::new(relative_subpath).to_path_buf(),
        Path::new("..").join(relative_subpath),
        Path::new("file utili").join(relative_subpath),
        Path::new("..").join("file utili").join(relative_subpath),
    ];
    candidates.into_iter().find(|c| c.exists())
}

fn is_dvc_title(name: &str) -> bool {
    let u = name.to_uppercase();
    u.contains("LAIR") || u.contains("GUEST") || u.contains("EDEN") || u.contains("MAD DOG")
        || u.contains("SPACE ACE") || u.contains("BRAIN DEAD")
}

fn save_bmp(fb: &[u32], path: &str) {
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
    for &p in fb {
        bmp.push((p & 0xFF) as u8);
        bmp.push(((p >> 8) & 0xFF) as u8);
        bmp.push(((p >> 16) & 0xFF) as u8);
        bmp.push(0xFF);
    }
    let _ = std::fs::write(path, &bmp);
}

fn fnv(data: impl Iterator<Item = u32>) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for v in data {
        h ^= v as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

#[derive(Clone, PartialEq)]
struct Sample {
    dcr: [u16; 2],
    ddr: [u16; 2],
    icm: u32,
    tcr: u32,
    order: u32,
    wf: [u8; 2],
    backdrop: u32,
    mosaic: [u32; 2],
    transparent_color: [u32; 2],
    mask_color: [u32; 2],
    clut_hash: u64,
    matte_hash: u64,
}

fn sample(b: &CdiBus) -> Sample {
    let m = &b.mcd212;
    Sample {
        dcr: [m.dcr[0] & 0xFFC0, m.dcr[1] & 0xFFC0],
        ddr: [m.ddr[0] & 0xFFC0, m.ddr[1] & 0xFFC0],
        icm: m.image_coding_method,
        tcr: m.transparency_control,
        order: m.plane_order,
        wf: [m.weight_factor[0][0], m.weight_factor[1][0]],
        backdrop: m.backdrop_color,
        mosaic: m.mosaic_hold,
        transparent_color: m.transparent_color,
        mask_color: m.mask_color,
        clut_hash: fnv(m.clut.iter().copied()),
        matte_hash: fnv(m.matte_control.iter().copied()),
    }
}

fn diff(a: &Sample, b: &Sample) -> String {
    let mut s = String::new();
    if a.dcr[0] != b.dcr[0] {
        s += &format!(" DCR1 {:04X}->{:04X}", a.dcr[0], b.dcr[0]);
        if (a.dcr[0] ^ b.dcr[0]) & 0x8000 != 0 {
            s += &format!(" [DE={}]", (b.dcr[0] >> 15) & 1);
        }
        if (a.dcr[0] ^ b.dcr[0]) & 0x0200 != 0 {
            s += &format!(" [ICA1={}]", (b.dcr[0] >> 9) & 1);
        }
        if (a.dcr[0] ^ b.dcr[0]) & 0x0100 != 0 {
            s += &format!(" [DCA1={}]", (b.dcr[0] >> 8) & 1);
        }
    }
    if a.dcr[1] != b.dcr[1] {
        s += &format!(" DCR2 {:04X}->{:04X}", a.dcr[1], b.dcr[1]);
    }
    if a.ddr != b.ddr {
        s += &format!(" DDR {:04X}/{:04X}->{:04X}/{:04X}", a.ddr[0], a.ddr[1], b.ddr[0], b.ddr[1]);
    }
    if a.icm != b.icm {
        s += &format!(" ICM {:06X}->{:06X}", a.icm, b.icm);
    }
    if a.tcr != b.tcr {
        s += &format!(" TCR {:06X}->{:06X}", a.tcr, b.tcr);
    }
    if a.order != b.order {
        s += &format!(" ORDER {}->{}", a.order, b.order);
    }
    if a.wf != b.wf {
        s += &format!(" WF {:?}->{:?}", a.wf, b.wf);
    }
    if a.backdrop != b.backdrop {
        s += &format!(" BACKDROP {:X}->{:X}", a.backdrop, b.backdrop);
    }
    if a.mosaic != b.mosaic {
        s += " MOSAIC";
    }
    if a.transparent_color != b.transparent_color || a.mask_color != b.mask_color {
        s += " TRCOLOR/MASK";
    }
    if a.clut_hash != b.clut_hash {
        s += " CLUT";
    }
    if a.matte_hash != b.matte_hash {
        s += " MATTE";
    }
    s
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("uso: trace_video <disco> [--from N] [--to M] [--out DIR] [--dump a,b] [--autodump] [--press f:btn:len,...] [--quiet-lines]");
        return;
    }
    let disc_arg = args[1].clone();
    let mut from = 0u64;
    let mut to = 2500u64;
    let mut out_dir = String::from("traces");
    let mut dump: BTreeSet<u64> = BTreeSet::new();
    let mut autodump = false;
    let mut quiet_lines = false;
    let mut presses: Vec<(u64, i32, u64)> = Vec::new();
    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--from" => { from = args[i + 1].parse().unwrap(); i += 1; }
            "--to" => { to = args[i + 1].parse().unwrap(); i += 1; }
            "--out" => { out_dir = args[i + 1].clone(); i += 1; }
            "--dump" => {
                for t in args[i + 1].split(',') { if let Ok(v) = t.parse() { dump.insert(v); } }
                i += 1;
            }
            "--press" => {
                for t in args[i + 1].split(',') {
                    let p: Vec<&str> = t.split(':').collect();
                    if p.len() == 3 {
                        presses.push((p[0].parse().unwrap(), p[1].parse().unwrap(), p[2].parse().unwrap()));
                    }
                }
                i += 1;
            }
            "--autodump" => autodump = true,
            "--quiet-lines" => quiet_lines = true,
            _ => {}
        }
        i += 1;
    }
    let _ = std::fs::create_dir_all(&out_dir);

    let mut bus = CdiBus::new();
    let bios_path = find_file("bios/cdi220b.rom").expect("cdi220b.rom non trovata");
    bus.load_rom(&bios_path).unwrap();
    if let Some(p) = find_file("bios/nvram_mk48t08.bin") {
        let _ = bus.nvram.load_from_file(&p);
    }
    let disc_path = find_file(&disc_arg).or_else(|| {
        let p = PathBuf::from(&disc_arg);
        if p.exists() { Some(p) } else { None }
    }).expect("disco non trovato");
    bus.load_cdrom(&disc_path).unwrap();
    let game_name = disc_path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
    if is_dvc_title(&game_name) {
        if let Some(p) = find_file("dvc/vmpega.rom") {
            let _ = bus.load_dvc_rom(&p);
        }
    }
    let up = disc_path.to_string_lossy().to_uppercase();
    let mode = if up.contains("USA") || up.contains("JAPAN") || up.contains("NTSC") || up.contains("(US)") {
        mcd212::VideoMode::Ntsc
    } else {
        mcd212::VideoMode::Pal
    };
    bus.set_video_mode(mode);
    println!("# trace_video: {} mode={:?} frames {}..{}", game_name, mode, from, to);

    let mut cpu = CpuCore::new();
    cpu.set_cpu_type(CpuType::SCC68070);
    cpu.reset(&mut bus);
    bus.emulate_wait_states = true;

    let mut disc_launched = false;
    let mut shell_ready_count: u32 = 0;
    let mut pad_x: i32 = 575;
    let mut pad_y: i32 = 335;
    let mut frame_count: u64 = 0;
    let mut cycle_debt: i32;
    let mut last = sample(&bus);
    let mut prev_fb_hash = 0u64;
    let mut dump_after: BTreeSet<u64> = BTreeSet::new();
    let mut autodumps = 0;

    while frame_count < to {
        // --- auto boot (replica di main.rs) ---
        if bus.cdrom.is_some() && !disc_launched {
            let (_b, shell_state) = if bus.dvc.is_fitted() {
                let s = bus.read_long(0x00D08090u32.wrapping_add(0xFFFF9172));
                if s >= 2 { (0x00D08090u32, s) } else { (0x0000C000u32, bus.read_long(0x0000C000u32.wrapping_add(0xFFFF9172))) }
            } else {
                (0x0000C000u32, bus.read_long(0x0000C000u32.wrapping_add(0xFFFF9172)))
            };
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
            if ((cpu.pc >= 0x00010000 && cpu.pc < 0x00300000) || (cpu.pc >= 0x00D00000 && cpu.pc < 0x00E00000)) && frame_count > 250 {
                println!("# [F{}] disco in esecuzione in RAM PC=0x{:06X}", frame_count, cpu.pc);
                disc_launched = true;
                bus.emulate_wait_states = true;
                pad_x = 384;
                pad_y = 280;
                bus.slave.update_mouse(384, 280, 0);
            }
        }

        // input scriptato
        if disc_launched {
            let mut btn: u8 = 0;
            for &(f, b, len) in &presses {
                if frame_count >= f && frame_count < f + len { btn |= b as u8; }
            }
            bus.slave.update_mouse(pad_x, pad_y, btn);
        }

        bus.slave.video_mode = bus.mcd212.video_mode;
        let total_lines = bus.mcd212.video_mode.total_scanlines();
        let cyc_per_line = bus.mcd212.video_mode.cycles_per_scanline();
        cycle_debt = 0;
        let mut events: Vec<String> = Vec::new();
        let mut vsr_l0 = (0u32, 0u32);
        let mut int1_taken = 0u32;
        let mut vblank_asserts = 0u32;

        for line in 0..total_lines {
            bus.tick_scanline(line);
            if line == 0 {
                vsr_l0 = (bus.mcd212.get_vsr::<0>(), bus.mcd212.get_vsr::<1>());
            }
            let s = sample(&bus);
            if s != last {
                if frame_count >= from {
                    events.push(format!("  [F{} L{}]{}", frame_count, line, diff(&last, &s)));
                }
                last = s;
            }

            if bus.mcd212.vblank_irq {
                bus.scc_timer.lir |= 0x80;
                bus.mcd212.vblank_irq = false;
                vblank_asserts += 1;
                if frame_count >= from {
                    events.push(format!("  [F{} L{}] INT1 asserito", frame_count, line));
                }
            }
            let lir_before = bus.scc_timer.lir;

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
            if (lir_before & 0x80) != 0 && (bus.scc_timer.lir & 0x80) == 0 {
                int1_taken += 1;
                if frame_count >= from {
                    events.push(format!("  [F{} L{}] INT1 servito (PC=0x{:06X})", frame_count, line, cpu.pc));
                }
            }
            cycle_debt = (line_cycles - cyc_per_line as i32).clamp(0, 100);
            bus.render_scanline(line);
            bus.tick_slave(cyc_per_line);
            bus.tick_cdic(cyc_per_line);
            bus.tick_dvc(cyc_per_line);
        }

        let fb_hash = fnv(bus.mcd212.framebuffer.iter().copied());
        let changed_fb = fb_hash != prev_fb_hash;
        prev_fb_hash = fb_hash;

        if frame_count >= from {
            let has_state_event = events.iter().any(|e| !e.contains("INT1"));
            if !quiet_lines {
                for e in &events { println!("{}", e); }
            } else {
                for e in events.iter().filter(|e| !e.contains("INT1")) { println!("{}", e); }
            }
            println!(
                "F{:5} PC={:06X} vsr@L0=({:06X},{:06X}) dcr1={:04X} icm={:06X} tcr={:06X} wf=[{},{}] int1(ass/srv)={}/{} fb={:016X}{}",
                frame_count, cpu.pc, vsr_l0.0, vsr_l0.1, bus.mcd212.dcr[0], bus.mcd212.image_coding_method,
                bus.mcd212.transparency_control, bus.mcd212.weight_factor[0][0], bus.mcd212.weight_factor[1][0],
                vblank_asserts, int1_taken, fb_hash, if changed_fb { "" } else { " (=)" }
            );
            if autodump && has_state_event && autodumps < 80 {
                for k in 0..4 { dump_after.insert(frame_count + k); }
                dump.insert(frame_count);
                autodumps += 1;
            }
            if dump.contains(&frame_count) || dump_after.contains(&frame_count) {
                save_bmp(&bus.mcd212.framebuffer[..], &format!("{}/f{:05}.bmp", out_dir, frame_count));
            }
        }
        frame_count += 1;
    }
}
