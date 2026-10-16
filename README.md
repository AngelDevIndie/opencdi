# OpenCDi

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Language: Rust](https://img.shields.io/badge/language-Rust-orange.svg)](https://www.rust-lang.org/)

[Versione Italiana](README_IT.md)

OpenCDi is a modern, standalone, open-source **Philips CD-i** emulator written in **Rust**.
The goal of the project is to accurately preserve and run the software and multimedia library of the Philips CD-i system with a clean, modular, and fast architecture.

---

## Architecture & Features

- **CPU:** Philips SCC68070 (full 68000/68010 instruction set, autovectored interrupts, on-chip DMA channels, timers, and UART).
- **Video:** Philips MCD-212 Video Display Controller:
  - Dual-plane rendering (Plane A & Plane B) with hardware backdrop.
  - Video coding methods: CLUT4, CLUT7, CLUT8, Run-Length (7-bit and 8-bit), DYUV (Delta-YUV), and RGB555.
  - Line-by-line Instruction Control Area (ICA) and Dynamic Control Area (DCA) display lists execution.
- **CD-ROM & Audio:**
  - CDIC (CD-i Controller) with sector processing for Mode 2 Form 1 and Form 2.
  - Hardware CD-DA and CD-i ADPCM audio decoding (Level A, B, and C, stereo and mono).
  - Native support for compressed CHD disc images (`chd-rs`) and raw 2352-byte sector images.
- **Slave Microcontroller (68HC05 HLE):**
  - High-Level Emulation for CD-i keyboard, mouse, and gamepads.
  - Real-Time Clock (RTC) and Non-Volatile RAM (NVRAM) state persistence.
- **Digital Video Cartridge (DVC / MPEG-1):**
  - Hardware FMV (Full Motion Video) cartridge emulation.
  - Integrated MPEG-1 system stream demuxer (video and audio packets).
  - Video decoder with YUV-to-RGB planar compositing onto the MCD-212 external video plane.
  - Accurate presentation pacing and end-of-sequence stream drain detection.
- **Frontend GUI:**
  - Interactive 50/60 Hz display window powered by `minifb`.
  - Configurable keyboard, mouse, and gamepad controls.

---

## Verified Game Compatibility

| Game | Graphics / DVC Mode | Current Status |
|---|---|---|
| **Hotel Mario** | MCD-212 / CDIC ADPCM | Fully playable in-game with complete animations and sound |
| **Striker Pro** | MCD-212 / CDIC ADPCM | Menus and in-game match playable with synchronized audio |
| **Dragon's Lair (USA)** | DVC FMV MPEG-1 | Continuous attract mode and interactive gameplay with active MPEG video |
| **The 7th Guest** | DVC FMV MPEG-1 | Boots, plays intro cutscenes, and advances through scenes |
| **Lost Eden** | DVC FMV MPEG-1 / CDIC | Boots, cutscenes, character dialogue, and exploration |
| **Rise of the Robots** | MCD-212 / DVC | Boots and plays intro sequences |
| **Burn:Cycle** | DVC FMV MPEG-1 | Boots and plays video sequences |

---

## Building and Running

### Prerequisites
- [Rust & Cargo](https://rustup.rs/) (version 1.70 or newer).

### Building from Source
```bash
git clone https://github.com/sigsantilio/opencdi.git
cd opencdi
cargo build --release
```

The optimized binary will be built in `target/release/opencdi.exe`.

### BIOS & DVC ROM Setup
Place the following system ROMs inside the `bios/` folder or alongside the executable:
- `cdi220b.rom` (Philips CD-i 220 BIOS)
- `vmpega.rom` (Digital Video Cartridge ROM for titles with MPEG video)

### Running a Game
```bash
cargo run --release -- "path/to/game.chd"
```
Or drag and drop any `.chd` CD-i disc image directly onto `opencdi.exe`.

---

## Contributing

This project is driven by a passion for CD-i preservation and hardware research.
All contributions are warmly welcome:
- Bug reports and game compatibility testing.
- Pull requests improving CPU timing, peripheral registers, or decoder accuracy.
- Hardware traces and documentation from real CD-i units.

---

## License

Distributed under the **MIT License**. See [`LICENSE`](LICENSE) for details.
