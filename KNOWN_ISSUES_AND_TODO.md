# OpenCDi - Known Issues & Resolution Plan (TODO)

[Versione Italiana](PROBLEMI_STORICI_E_TODO.md)

This document tracks all historically reported issues on the OpenCDi emulator, the technical analysis of their architectural causes, the planned solution, and current progress. Every issue resolved and validated in actual gameplay is marked as completed `[x]` with technical fix details.

## Current Status (Updated 16/10/2026 - Version v0.1.3)
Legend: `[ ]` open · `[~]` fix applied in code, **pending in-game confirmation** · `[x]` confirmed by user.

| Title | Issue | Status | Fix / Note |
|---|---|---|---|
| Video transitions (Hotel Mario, Striker) | Intermediate glitchy frames / artifacts during screen fades lasting a few tenths of a second | `[x]` | **Root architectural fix (v0.1.1):** in `mcd212.rs`, `mix_lines` bypassed analog weight blending when a plane was transparent (`out[x] = pa; continue;`), exposing uninitialized VRAM at 100% brightness during fade-outs. Aligned with MAME and hardware specifications: transparent planes adopt black level `s_4bpp_color[0]` and pass through the formula with `weight_factor`. With weight 0, output is solid black (verified with `trace_video` on frames 2426–2468 of Hotel Mario). |
| Pad Controls | Upward D-Pad input unresponsive ("cannot move up") | `[x]` | **Fixed (v0.1.1):** in `main.rs`, decoupled and prioritized D-pad directions: D-pad buttons first (`Button::DPad*`), D-pad axis only if neutral (`Axis::DPad*`), analog stick last (`LeftStickY`) only if neutral. Fixed conflict that summed -1 and +1, zeroing out the UP direction. |
| Striker | Player sprite flickering and variable game speed | `[ ]` | **Under active analysis:** isolating the double-buffering cycle (VSR/DCP) and scanline/field interrupt timings (DCA line 43 and ICA line 0). |
| All Games | Disc boot failure in BIOS (stuck on "OPEN" / disc not recognized) | `[x]` | **Fixed:** in `bus.rs`, Level 2 INTACK (Slave MCU) returned `0x3A` instead of Autovector 2 `0x1A`. With `0x1A`, OS-9 `cdapdriv` executes `0x42D3D2` and transitions disc state to "PLAY CD-I". |
| 7th Guest | Black backdrop post-video fixed; corridor freeze after ghost scene persists | `[ ]` | **Black backdrop resolved:** removed improper `DCR_DE_BIT` check on Path 1 in `mcd212.rs`. **Corridor freeze OPEN:** active investigation on room transition, I/O calls to CDIC, and subsequent MPEG stream loading. |
| Rise of the Robots | Combat speed excessively fast / cutscenes accelerated | `[ ]` | Under investigation: CD-DA streaming rate at 75 sectors/s and OS-9 `F$Sleep` timing. |
| Lost Eden | Audio/dialogue voices, menus, FMV stuttering | `[ ]` | Under investigation: CDIC ADPCM soundmap processing and Plane B external video overlay keying. |

> Binary synchronization note: all 3 executable binaries (`opencdi/target/release/opencdi.exe`, `opencdi/opencdi.exe`, and `../opencdi.exe`) are compiled in Release mode and synchronized with identical SHA256:
> `99260CE0FE805FC5BFFECABC16433811BC5793F48BDCA2075B617AAF1B0169C3`.

---

## 1. Hotel Mario
- [x] **Missing startup console logo / animation**
  - **Resolved:** By adhering to BIOS shell timing requirements (minimum wait of 30 fields before clicking PLAY CD-I) and clean rendering without filter bail-outs when `VSR=0`, the Philips console crest and startup animation display completely.
- [x] **Corrupted transitions with psychedelic colors / visible drawing**
  - **Resolved (v0.1.1):** In `mix_lines`, Plane B transparency no longer bypasses multiplication by `weight_factor`. During VRAM loads where `weight == 0`, the screen stays solid black until rendering setup completes, eliminating all artifacts.
- [x] **Missing background during intro & gameplay without backdrops**
  - **Resolved:** Eliminating the improper `* 298` scaling factor in `mix_lines` and removing the `vsr == 0` early return restored proper transparency and mixing between Plane A and Plane B.

---

## 2. Striker
- [ ] **Player sprite flickering and uneven game pace during match**
  - **In Progress:** Analyzing buffer swap cadence and INT1 interrupt synchronization.
- [x] **Pitch / playing field not displayed**
  - **Resolved:** The hardware-accurate `mix_lines` formula renders both Plane A and Plane B correctly (green grass texture and all menu assets).

---

## 3. Lost Eden (DVC)
- [ ] **Dialogue audio and voice distortion**
  - **Symptom:** Overall sound has defects and character dialogues (Elo, Adam, Dina) sound distorted or corrupted.
  - **Technical Cause:**
    1. In-game dialogues utilize CD-i Soundmap ADPCM sectors (Level A/B at 37.8 kHz / 18.9 kHz) processed by CDIC/Audio DSP.
    2. Video cutscenes utilize MPEG-1 Audio Layer 2 (MP2). Discrepancies in buffer consumption or ADPCM filter coefficients distort human voice reproduction.
  - **Action Plan:** Inspect ADPCM soundmap packet decoding and DVC MP2 audio resampling.

- [ ] **Game menus and interface not displaying correctly**
  - **Symptom:** Interactive menus (inventory, selection screens) are invisible or glitchy.
  - **Technical Cause:** Lost Eden menus are drawn on MCD-212 Plane B on top of external DVC video using Color Keying / Mask Color (`0x00FC_FCFC`). If external video line blending or transparency is disabled, menus are obscured or faded out.
  - **Action Plan:** Verify External Video Enable bits in `image_coding_method` and Plane B color key mask.

- [ ] **Micro-stuttering during FMV playback**
  - **Symptom:** Slight stutter/frame drops during MPEG-1 playback.
  - **Technical Cause:** Presentation cadence of MPEG frames (nominal 25 fps) vs PAL video refresh (50 Hz). A jitter-free Presentation Timestamp (PTS) mechanism at 2 fields per frame is required.
  - **Action Plan:** Refine PTS/SCR synchronization in `dvc.rs` demuxer.

---

## 4. Rise of the Robots
- [ ] **Excessively fast combat gameplay (Game runs too fast)**
  - **Symptom:** Robot combatants move and fight at unrealistic speeds (2x-3x).
  - **Technical Cause:** Rise of the Robots gameplay loop performs OS-9 `F$Sleep` calls or waits on VBLANK ticks. If tasks wake up early or CPU runs unthrottled cycles without synchronous wait, the game runs at uncapped speed.
  - **Action Plan:** Verify OS-9 tick duration in `scc_timer.rs` and 50 Hz synchronous `F$Sleep` blocking.

- [ ] **Cutscenes ending prematurely with frozen final frame**
  - **Symptom:** Opening and cutscenes play at hyper-speed and freeze on the final frame, waiting until the CD-DA audio track finishes.
  - **Technical Cause:** Investigating CD-DA playback and timing. `CYCLES_PER_SECTOR` is already at 75 sectors/s; game issues `I$SetStt` (path 4, code 0x56, sub 0x11, D3=0xFF) with D4=0.
  - **Next Steps:** (1) Identify where D4 is populated / what CDAP sub 0x11 expects; (2) Verify OS-9 SCC timer reload frequency and CPU clock; (3) Compare with MAME VBLANKs per game loop.

---

## 5. The 7th Guest (DVC)
- [x] **Black backdrop with isolated foreground elements after video transitions**
  - **Symptom:** At the end of FMV transitions, the room displays almost entirely black with only interactive hotspot items visible, lacking the 3D mansion backdrop.
  - **Technical Cause:** In `src/mcd212.rs`, an erroneous `(self.dcr[PATH] & DCR_DE_BIT) == 0` check forced black pixels and full transparency. In Philips MCD-212 hardware specs, only DCR1 (Path 0) possesses the global Display Enable bit; on DCR2 (Path 1 / Plane B) bit 15 is not a plane enable and is not set by the game. Consequently, Plane B (the 3D room backdrop) was zeroed, leaving only Plane A overlays on black.
  - **Resolution:** Removed the erroneous check in `process_vsr`. 3D backdrops and interactive objects render fused properly.

- [ ] **Corridor freeze after upstairs ghost cutscene**
  - **Symptom:** Walking up the stairs and turning left, after the ghost corridor FMV finishes, the game hangs when attempting to move forward.
  - **Status:** `[ ]` Under active investigation. DMAREQ2 non-dropping handshake and SharedIn4Owner daisy-chain arbitration have been implemented in the core architecture, but the in-game transition to the next room still hangs. Currently investigating subsequent CDIC sector requests, room script execution, and MPEG stream chaining.

*(For an overview of all 15 games in `file utili/giochi/`, see [`COMPATIBILITY.md`](COMPATIBILITY.md))*
