use std::path::{Path, PathBuf};
use opencdi::bus::CdiBus;
use m68k::{CpuCore, CpuType};

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
    let mut cpu = CpuCore::new();
    cpu.set_cpu_type(CpuType::SCC68070);

    let bios_path = find_file("bios/cdi220b.rom").expect("cdi220b.rom not found");
    bus.load_rom(&bios_path)?;
    if let Some(nvram) = find_file("bios/nvram_mk48t08.bin") {
        let _ = bus.nvram.load_from_file(&nvram);
    }
    bus.reset_system();
    cpu.reset(&mut bus);

    let cyc_per_line = bus.mcd212.video_mode.cycles_per_scanline();
    let num_lines = bus.mcd212.video_mode.total_scanlines();

    let mut pad_x = 384;
    let mut pad_y = 280;
    let mut pad_x_f = 384.0f32;
    let mut pad_y_f = 280.0f32;
    const PAD_SPEED: f32 = 2.5;

    println!("Booting standalone BIOS...");
    for frame in 0..1100 {
        let mut dir_x = 0;
        let mut dir_y = 0;

        // At frame 700..720: UP
        if frame >= 700 && frame < 720 {
            dir_y = -1;
        }
        // At frame 740..760: DOWN
        if frame >= 740 && frame < 760 {
            dir_y = 1;
        }
        // At frame 780..800: LEFT
        if frame >= 780 && frame < 800 {
            dir_x = -1;
        }
        // At frame 820..840: RIGHT
        if frame >= 820 && frame < 840 {
            dir_x = 1;
        }

        if (frame >= 595 && frame <= 605) || (frame >= 695 && frame <= 730) {
            bus.slave.debug_log = true;
        } else {
            bus.slave.debug_log = false;
        }

        if bus.slave.device_mouse_updated {
            println!(">>> [Frame {}] BIOS/OS-9 WARPED MOUSE to ({}, {})! Prev pad=({}, {})",
                frame, bus.slave.device_mouse_x, bus.slave.device_mouse_y, pad_x, pad_y);
            pad_x = bus.slave.device_mouse_x;
            pad_y = bus.slave.device_mouse_y;
            pad_x_f = pad_x as f32;
            pad_y_f = pad_y as f32;
            bus.slave.device_mouse_updated = false;
        }

        let mut moved = false;
        if dir_x != 0 {
            pad_x_f = (pad_x_f + (dir_x.clamp(-1, 1) as f32) * PAD_SPEED).clamp(0.0, 767.0);
            pad_x = pad_x_f.round() as i32;
            moved = true;
        }
        if dir_y != 0 {
            pad_y_f = (pad_y_f + (dir_y.clamp(-1, 1) as f32) * PAD_SPEED).clamp(0.0, 559.0);
            pad_y = pad_y_f.round() as i32;
            moved = true;
        }

        if moved {
            bus.slave.update_mouse(pad_x, pad_y, 0);
        }

        for line in 0..num_lines {
            bus.tick_scanline(line);
            if bus.mcd212.vblank_irq {
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
            let cdic_lvl = if bus.in4_pending() { 4 } else { 0 };
            let slave_lvl = if bus.slave.irq_pending { 2 } else { 0 };
            let highest_ipl = int1_lvl.max(timer_lvl).max(cdic_lvl).max(slave_lvl);
            cpu.set_irq(highest_ipl);

            let mut line_cycles = 0;
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
                        break;
                    }
                    _ => {}
                }
                line_cycles += batch_cycles;
                bus.tick_timer(batch_cycles as u32);
            }
            bus.render_scanline(line);
            bus.tick_slave(cyc_per_line);
            bus.tick_cdic(cyc_per_line);
            bus.tick_dvc(cyc_per_line);
        }

        if frame % 100 == 0 || (frame >= 698 && frame <= 850) {
            println!("Frame {:4}: pad=({}, {}) dir=({}, {}) PC=0x{:06X}",
                frame, pad_x, pad_y, dir_x, dir_y, cpu.pc);
        }
    }

    Ok(())
}
