use gilrs::{Axis, Button, Gilrs};
use minifb::{Key, MouseButton, MouseMode, Window};
use crate::gui::{InputDeviceMode, KeyBindings};
use crate::slave::SlaveHle;

#[cfg(target_os = "windows")]
#[repr(C)]
struct RECT {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

#[cfg(target_os = "windows")]
#[repr(C)]
struct POINT {
    x: i32,
    y: i32,
}

#[cfg(target_os = "windows")]
#[link(name = "user32")]
unsafe extern "system" {
    fn GetClientRect(hwnd: *mut std::ffi::c_void, lpRect: *mut RECT) -> i32;
    fn ClientToScreen(hwnd: *mut std::ffi::c_void, lpPoint: *mut POINT) -> i32;
    fn ClipCursor(lpRect: *const RECT) -> i32;
    fn ShowCursor(bShow: i32) -> i32;
}

/// Logical CD-i host input frame (sampled once per host frame)
#[derive(Debug, Clone, Copy, Default)]
pub struct HostInputFrame {
    /// Directional X: -1 (Left), 0 (Neutral), +1 (Right)
    pub dir_x: i32,
    /// Directional Y: -1 (Up), 0 (Neutral), +1 (Down)
    pub dir_y: i32,
    /// Primary Button (Button 1: Action / Confirm)
    pub btn1: bool,
    /// Secondary Button (Button 2: Jump / Cancel)
    pub btn2: bool,
    /// Tertiary Button (Button 3: Option / Pause)
    pub btn3: bool,
    /// Absolute cursor X in CD-i resolution (0..767)
    pub mouse_x: i32,
    /// Absolute cursor Y in CD-i resolution (0..559)
    pub mouse_y: i32,
    /// Physical mouse left click
    pub mouse_btn1: bool,
    /// Physical mouse right click
    pub mouse_btn2: bool,
    /// User generated any active input this frame
    pub user_active: bool,
}

pub struct InputManager {
    pub mode: InputDeviceMode,
    pub bindings: KeyBindings,
    pub gilrs: Option<Gilrs>,
    pub mouse_captured: bool,
    pub host_cursor_hidden: bool,
    pub mouse_clipped: bool,

    // Internal state tracking
    prev_middle: bool,
    prev_key_btn: u8,
    prev_mx: f32,
    prev_my: f32,
    pub pad_x: i32,
    pub pad_y: i32,
    pub stream_counter: u32,
    pub was_moving: bool,
}

impl InputManager {
    pub fn new(mode: InputDeviceMode, bindings: KeyBindings) -> Self {
        let gilrs = match Gilrs::new() {
            Ok(g) => {
                log::info!("Sottosistema Gamepad Gilrs inizializzato con successo.");
                Some(g)
            }
            Err(e) => {
                log::warn!("Inizializzazione Gamepad Gilrs non riuscita (utilizzare tastiera): {:?}", e);
                None
            }
        };

        Self {
            mode,
            bindings,
            gilrs,
            mouse_captured: false,
            host_cursor_hidden: false,
            mouse_clipped: false,
            prev_middle: false,
            prev_key_btn: 0,
            prev_mx: 384.0,
            prev_my: 280.0,
            pad_x: 384,
            pad_y: 280,
            stream_counter: 0,
            was_moving: false,
        }
    }

    /// Poll all host input sources (Keyboard, Gamepad, Mouse)
    pub fn poll(&mut self, window: &Window, is_active: bool) -> HostInputFrame {
        let mut frame = HostInputFrame {
            mouse_x: self.pad_x,
            mouse_y: self.pad_y,
            ..Default::default()
        };

        // 1. Mouse Capture Toggle (Middle mouse button / wheel click)
        let middle_down = window.get_mouse_down(MouseButton::Middle);
        if is_active && middle_down && !self.prev_middle {
            self.mouse_captured = !self.mouse_captured;
            log::info!(
                "Controllo mouse: {}",
                if self.mouse_captured {
                    "CATTURATO (focus console, puntatore host nascosto)"
                } else {
                    "LIBERO (puntatore desktop visibile, rotellina o clic per ricatturare)"
                }
            );
        }
        self.prev_middle = middle_down;

        // Left click inside window re-captures mouse if released
        if is_active && !self.mouse_captured && window.get_mouse_down(MouseButton::Left) {
            self.mouse_captured = true;
            log::info!("Controllo mouse: CATTURATO (focus console, puntatore host nascosto)");
        }

        // Apply host cursor visibility & confinement
        #[cfg(target_os = "windows")]
        {
            let should_confine = is_active && (self.mouse_captured || self.mode == InputDeviceMode::Mouse);
            self.set_cursor_visibility(!should_confine);
            self.set_mouse_confinement(should_confine, window.get_window_handle());
        }

        // 2. Sample Keyboard Inputs
        if is_active {
            // Directional D-Pad keys: Configured bindings + Universal fallbacks (Arrows & WASD)
            if window.is_key_down(self.bindings.left) || window.is_key_down(Key::Left) || window.is_key_down(Key::A) {
                frame.dir_x -= 1;
                frame.user_active = true;
            }
            if window.is_key_down(self.bindings.right) || window.is_key_down(Key::Right) || window.is_key_down(Key::D) {
                frame.dir_x += 1;
                frame.user_active = true;
            }
            if window.is_key_down(self.bindings.up) || window.is_key_down(Key::Up) || window.is_key_down(Key::W) {
                frame.dir_y -= 1;
                frame.user_active = true;
            }
            if window.is_key_down(self.bindings.down) || window.is_key_down(Key::Down) || window.is_key_down(Key::S) {
                frame.dir_y += 1;
                frame.user_active = true;
            }

            // Button 1 (Action / Confirm): Binding, Space, Enter, Z, J
            if window.is_key_down(self.bindings.btn1)
                || window.is_key_down(Key::Space)
                || window.is_key_down(Key::Enter)
                || window.is_key_down(Key::Z)
                || window.is_key_down(Key::J)
            {
                frame.btn1 = true;
                frame.user_active = true;
            }

            // Button 2 (Jump / Cancel): Binding, Left/Right Shift, X, K
            if window.is_key_down(self.bindings.btn2)
                || window.is_key_down(Key::LeftShift)
                || window.is_key_down(Key::RightShift)
                || window.is_key_down(Key::X)
                || window.is_key_down(Key::K)
            {
                frame.btn2 = true;
                frame.user_active = true;
            }

            // Button 3 (Option / Pause): C, P, Backspace
            if window.is_key_down(Key::C) || window.is_key_down(Key::P) || window.is_key_down(Key::Backspace) {
                frame.btn3 = true;
                frame.user_active = true;
            }
        }

        // 3. Sample Gamepad Inputs via Gilrs
        if let Some(ref mut g) = self.gilrs {
            while let Some(_) = g.next_event() {} // Drain events to refresh internal state
            for (_id, gamepad) in g.gamepads() {
                if gamepad.is_connected() {
                    let mut pad_dir_x = 0;
                    let mut pad_dir_y = 0;

                    // D-Pad buttons (highest priority)
                    if gamepad.is_pressed(Button::DPadUp) { pad_dir_y = -1; }
                    else if gamepad.is_pressed(Button::DPadDown) { pad_dir_y = 1; }

                    if gamepad.is_pressed(Button::DPadLeft) { pad_dir_x = -1; }
                    else if gamepad.is_pressed(Button::DPadRight) { pad_dir_x = 1; }

                    // D-Pad axes (for gamepads where D-Pad is reported as Hat/Axis)
                    if pad_dir_x == 0 {
                        let dpx = gamepad.value(Axis::DPadX);
                        if dpx < -0.4 { pad_dir_x = -1; }
                        else if dpx > 0.4 { pad_dir_x = 1; }
                    }
                    if pad_dir_y == 0 {
                        let dpy = gamepad.value(Axis::DPadY);
                        if dpy > 0.4 { pad_dir_y = -1; }
                        else if dpy < -0.4 { pad_dir_y = 1; }
                    }

                    // Left Analog Stick (deadzone 0.35, checked if D-Pad is neutral)
                    if pad_dir_x == 0 {
                        let lx = gamepad.value(Axis::LeftStickX);
                        if lx < -0.35 { pad_dir_x = -1; }
                        else if lx > 0.35 { pad_dir_x = 1; }
                    }
                    if pad_dir_y == 0 {
                        let ly = gamepad.value(Axis::LeftStickY);
                        if ly > 0.35 { pad_dir_y = -1; }
                        else if ly < -0.35 { pad_dir_y = 1; }
                    }

                    if pad_dir_x != 0 {
                        frame.dir_x = pad_dir_x;
                        frame.user_active = true;
                    }
                    if pad_dir_y != 0 {
                        frame.dir_y = pad_dir_y;
                        frame.user_active = true;
                    }

                    // Button 1: South (A on Xbox, Cross on PlayStation)
                    if gamepad.is_pressed(Button::South) {
                        frame.btn1 = true;
                        frame.user_active = true;
                    }
                    // Button 2: East (B / Circle) or West (X / Square)
                    if gamepad.is_pressed(Button::East) || gamepad.is_pressed(Button::West) {
                        frame.btn2 = true;
                        frame.user_active = true;
                    }
                    // Button 3: North (Y / Triangle) or Start
                    if gamepad.is_pressed(Button::North) || gamepad.is_pressed(Button::Start) {
                        frame.btn3 = true;
                        frame.user_active = true;
                    }
                }
            }
        }

        // Clamp combined directions to -1, 0, 1
        frame.dir_x = frame.dir_x.clamp(-1, 1);
        frame.dir_y = frame.dir_y.clamp(-1, 1);

        // 4. Sample Host Mouse Coordinates
        if is_active && (self.mouse_captured || self.mode == InputDeviceMode::Mouse) {
            if let Some((mx, my)) = window.get_mouse_pos(MouseMode::Discard) {
                frame.mouse_btn1 = window.get_mouse_down(MouseButton::Left);
                frame.mouse_btn2 = window.get_mouse_down(MouseButton::Right);

                frame.mouse_x = (mx.round() as i32).clamp(0, 767);
                frame.mouse_y = (my.round() as i32).clamp(0, 559);
                self.prev_mx = mx;
                self.prev_my = my;

                if frame.mouse_btn1 || frame.mouse_btn2 {
                    frame.user_active = true;
                }
            }
        }

        frame
    }

    /// Dispatch input to Slave MCU according to current mode
    pub fn dispatch(&mut self, slave: &mut SlaveHle, input: &HostInputFrame) {
        // Sync cursor position if game warped cursor
        if slave.device_mouse_updated {
            self.pad_x = slave.device_mouse_x;
            self.pad_y = slave.device_mouse_y;
            slave.device_mouse_updated = false;
        }

        let btn_mask = (if input.btn1 { 1 } else { 0 })
            | (if input.btn2 { 2 } else { 0 })
            | (if input.btn3 { 4 } else { 0 });

        match self.mode {
            InputDeviceMode::Gamepad => {
                let pad_speed = 8;
                let is_moving = input.dir_x != 0 || input.dir_y != 0;

                if input.dir_x != 0 {
                    self.pad_x = (self.pad_x + input.dir_x.clamp(-1, 1) * pad_speed).clamp(0, 767);
                }
                if input.dir_y != 0 {
                    self.pad_y = (self.pad_y + input.dir_y.clamp(-1, 1) * pad_speed).clamp(0, 559);
                }

                if is_moving || btn_mask != 0 || btn_mask != self.prev_key_btn || self.was_moving {
                    self.stream_counter = self.stream_counter.wrapping_add(1);
                    let dither = (self.stream_counter & 1) as i32;

                    let send_x = if input.dir_x > 0 && self.pad_x >= 767 {
                        767 - dither
                    } else if input.dir_x < 0 && self.pad_x <= 0 {
                        dither
                    } else {
                        self.pad_x
                    };

                    let send_y = if input.dir_y > 0 && self.pad_y >= 559 {
                        559 - dither
                    } else if input.dir_y < 0 && self.pad_y <= 0 {
                        dither
                    } else {
                        self.pad_y
                    };

                    slave.update_mouse(send_x, send_y, btn_mask);
                }

                self.prev_key_btn = btn_mask;
                self.was_moving = is_moving;
            }
            InputDeviceMode::Mouse => {
                // Philips CD-i Mouse (22ER9011): direct coordinate positioning
                let mouse_btn = (if input.mouse_btn1 { 1 } else { 0 })
                    | (if input.mouse_btn2 { 2 } else { 0 })
                    | btn_mask;
                self.pad_x = input.mouse_x;
                self.pad_y = input.mouse_y;
                slave.update_mouse(self.pad_x, self.pad_y, mouse_btn);
                self.prev_key_btn = mouse_btn;
            }
        }
    }

    #[cfg(target_os = "windows")]
    pub fn set_cursor_visibility(&mut self, visible: bool) {
        unsafe {
            if !visible && !self.host_cursor_hidden {
                while ShowCursor(0) >= 0 {}
                self.host_cursor_hidden = true;
            } else if visible && self.host_cursor_hidden {
                while ShowCursor(1) < 0 {}
                self.host_cursor_hidden = false;
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    pub fn set_cursor_visibility(&mut self, _visible: bool) {}

    #[cfg(target_os = "windows")]
    pub fn set_mouse_confinement(&mut self, confined: bool, hwnd_raw: *mut std::ffi::c_void) {
        unsafe {
            if confined && !hwnd_raw.is_null() {
                let mut client_rect: RECT = RECT { left: 0, top: 0, right: 0, bottom: 0 };
                GetClientRect(hwnd_raw, &mut client_rect);
                let mut top_left = POINT {
                    x: client_rect.left,
                    y: client_rect.top,
                };
                let mut bot_right = POINT {
                    x: client_rect.right,
                    y: client_rect.bottom,
                };
                ClientToScreen(hwnd_raw, &mut top_left);
                ClientToScreen(hwnd_raw, &mut bot_right);
                let clip_rect = RECT {
                    left: top_left.x,
                    top: top_left.y,
                    right: bot_right.x,
                    bottom: bot_right.y,
                };
                ClipCursor(&clip_rect);
                self.mouse_clipped = true;
            } else if !confined && self.mouse_clipped {
                ClipCursor(std::ptr::null());
                self.mouse_clipped = false;
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    pub fn set_mouse_confinement(&mut self, _confined: bool, _hwnd_raw: *mut std::ffi::c_void) {}

    /// Restore host mouse on exit
    pub fn cleanup(&mut self, _hwnd_raw: *mut std::ffi::c_void) {
        self.set_cursor_visibility(true);
        self.set_mouse_confinement(false, _hwnd_raw);
    }
}
