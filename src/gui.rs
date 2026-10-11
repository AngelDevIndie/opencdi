use std::ffi::{OsStr, OsString};
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::path::PathBuf;
use std::sync::Mutex;
use log::info;
use minifb::Key;

pub const ID_FILE_INSERT: usize = 1000;
pub const ID_FILE_OPEN: usize = 1001;
pub const ID_FILE_RESET: usize = 1002;
pub const ID_FILE_EJECT: usize = 1004;
pub const ID_FILE_EXIT: usize = 1003;

pub const ID_PROFILE_STANDARD: usize = 1010;
pub const ID_PROFILE_WASD: usize = 1011;
pub const ID_PROFILE_NUMPAD: usize = 1012;
pub const ID_PROFILE_CUSTOM: usize = 1015;
pub const ID_CONTROLS_EDIT_INI: usize = 1016;
pub const ID_CONTROLS_RELOAD_INI: usize = 1017;
pub const ID_CONTROLS_INFO: usize = 1013;
pub const ID_CONTROLS_RELEASE_MOUSE: usize = 1014;
pub const ID_INPUT_DEVICE_GAMEPAD: usize = 1060;
pub const ID_INPUT_DEVICE_MOUSE: usize = 1061;

pub const ID_AUDIO_MUTE: usize = 1020;
pub const ID_AUDIO_VOL_UP: usize = 1021;
pub const ID_AUDIO_VOL_DOWN: usize = 1022;

pub const ID_VIDEO_FULLSCREEN: usize = 1030;
pub const ID_VIDEO_SCREENSHOT: usize = 1031;
pub const ID_VIDEO_PAL: usize = 1032;
pub const ID_VIDEO_NTSC: usize = 1033;
pub const ID_EMU_WAITSTATES: usize = 1050;

pub const ID_HELP_CONTROLS: usize = 1040;
pub const ID_HELP_ABOUT: usize = 1041;

// Win32 Constants
const MF_STRING: u32 = 0x0000_0000;
const MF_POPUP: u32 = 0x0000_0010;
const MF_SEPARATOR: u32 = 0x0000_0800;
const MF_BYCOMMAND: u32 = 0x0000_0000;
const MF_CHECKED: u32 = 0x0000_0008;
const MF_UNCHECKED: u32 = 0x0000_0000;

const WM_COMMAND: u32 = 0x0111;
const GWLP_WNDPROC: i32 = -4;

const MB_OK: u32 = 0x0000_0000;
const MB_YESNO: u32 = 0x0000_0004;
const MB_YESNOCANCEL: u32 = 0x0000_0003;
const MB_ICONINFORMATION: u32 = 0x0000_0040;
const MB_ICONQUESTION: u32 = 0x0000_0020;
const IDYES: i32 = 6;
const IDNO: i32 = 7;

#[repr(C)]
#[allow(non_snake_case)]
pub struct OPENFILENAMEW {
    pub lStructSize: u32,
    pub hwndOwner: *mut std::ffi::c_void,
    pub hInstance: *mut std::ffi::c_void,
    pub lpstrFilter: *const u16,
    pub lpstrCustomFilter: *mut u16,
    pub nMaxCustFilter: u32,
    pub nFilterIndex: u32,
    pub lpstrFile: *mut u16,
    pub nMaxFile: u32,
    pub lpstrFileTitle: *mut u16,
    pub nMaxFileTitle: u32,
    pub lpstrInitialDir: *const u16,
    pub lpstrTitle: *const u16,
    pub Flags: u32,
    pub nFileOffset: u16,
    pub nFileExtension: u16,
    pub lpstrDefExt: *const u16,
    pub lCustData: usize,
    pub lpfnHook: *mut std::ffi::c_void,
    pub lpTemplateName: *const u16,
    pub pvReserved: *mut std::ffi::c_void,
    pub dwReserved: u32,
    pub FlagsEx: u32,
}

#[link(name = "user32")]
unsafe extern "system" {
    fn CreateMenu() -> *mut std::ffi::c_void;
    fn CreatePopupMenu() -> *mut std::ffi::c_void;
    fn AppendMenuW(hmenu: *mut std::ffi::c_void, uflags: u32, uid: usize, lpitem: *const u16) -> i32;
    fn SetMenu(hwnd: *mut std::ffi::c_void, hmenu: *mut std::ffi::c_void) -> i32;
    fn DrawMenuBar(hwnd: *mut std::ffi::c_void) -> i32;
    fn CheckMenuItem(hmenu: *mut std::ffi::c_void, uid: u32, ucheck: u32) -> u32;
    fn MessageBoxW(hwnd: *mut std::ffi::c_void, lptext: *const u16, lpcaption: *const u16, utype: u32) -> i32;
    fn SetWindowLongPtrW(hwnd: *mut std::ffi::c_void, nindex: i32, dwnewlong: isize) -> isize;
    fn CallWindowProcW(prev: isize, hwnd: *mut std::ffi::c_void, msg: u32, wparam: usize, lparam: isize) -> isize;
}

#[link(name = "comdlg32")]
unsafe extern "system" {
    fn GetOpenFileNameW(lpofn: *mut OPENFILENAMEW) -> i32;
}

fn to_wide(s: &str) -> Vec<u16> {
    OsStr::new(s).encode_wide().chain(std::iter::once(0)).collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputProfile {
    Standard, // Frecce + Z / X
    Wasd,     // W, A, S, D + J / K
    Numpad,   // Numpad 8, 4, 6, 2 + 1 / 2
    Custom,   // Personalizzato da opencdi_controls.ini
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputDeviceMode {
    Gamepad, // Controller CD-i (Gamepad USB / Joystick / Tastiera centrate)
    Mouse,   // Mouse Philips CD-i (Cursore Fisico Host)
}

pub fn key_to_string(key: Key) -> &'static str {
    match key {
        Key::Up => "Up",
        Key::Down => "Down",
        Key::Left => "Left",
        Key::Right => "Right",
        Key::A => "A",
        Key::B => "B",
        Key::C => "C",
        Key::D => "D",
        Key::E => "E",
        Key::F => "F",
        Key::G => "G",
        Key::H => "H",
        Key::I => "I",
        Key::J => "J",
        Key::K => "K",
        Key::L => "L",
        Key::M => "M",
        Key::N => "N",
        Key::O => "O",
        Key::P => "P",
        Key::Q => "Q",
        Key::R => "R",
        Key::S => "S",
        Key::T => "T",
        Key::U => "U",
        Key::V => "V",
        Key::W => "W",
        Key::X => "X",
        Key::Y => "Y",
        Key::Z => "Z",
        Key::Space => "Space",
        Key::Enter => "Enter",
        Key::LeftShift => "LeftShift",
        Key::RightShift => "RightShift",
        Key::LeftCtrl => "LeftCtrl",
        Key::RightCtrl => "RightCtrl",
        Key::LeftAlt => "LeftAlt",
        Key::RightAlt => "RightAlt",
        Key::Tab => "Tab",
        Key::Backspace => "Backspace",
        Key::NumPad0 => "NumPad0",
        Key::NumPad1 => "NumPad1",
        Key::NumPad2 => "NumPad2",
        Key::NumPad3 => "NumPad3",
        Key::NumPad4 => "NumPad4",
        Key::NumPad5 => "NumPad5",
        Key::NumPad6 => "NumPad6",
        Key::NumPad7 => "NumPad7",
        Key::NumPad8 => "NumPad8",
        Key::NumPad9 => "NumPad9",
        _ => "Z",
    }
}

pub fn string_to_key(s: &str) -> Option<Key> {
    let s_upper = s.trim().to_uppercase();
    match s_upper.as_str() {
        "UP" => Some(Key::Up),
        "DOWN" => Some(Key::Down),
        "LEFT" => Some(Key::Left),
        "RIGHT" => Some(Key::Right),
        "A" => Some(Key::A),
        "B" => Some(Key::B),
        "C" => Some(Key::C),
        "D" => Some(Key::D),
        "E" => Some(Key::E),
        "F" => Some(Key::F),
        "G" => Some(Key::G),
        "H" => Some(Key::H),
        "I" => Some(Key::I),
        "J" => Some(Key::J),
        "K" => Some(Key::K),
        "L" => Some(Key::L),
        "M" => Some(Key::M),
        "N" => Some(Key::N),
        "O" => Some(Key::O),
        "P" => Some(Key::P),
        "Q" => Some(Key::Q),
        "R" => Some(Key::R),
        "S" => Some(Key::S),
        "T" => Some(Key::T),
        "U" => Some(Key::U),
        "V" => Some(Key::V),
        "W" => Some(Key::W),
        "X" => Some(Key::X),
        "Y" => Some(Key::Y),
        "Z" => Some(Key::Z),
        "SPACE" => Some(Key::Space),
        "ENTER" | "RETURN" => Some(Key::Enter),
        "LEFTSHIFT" | "SHIFT" => Some(Key::LeftShift),
        "RIGHTSHIFT" => Some(Key::RightShift),
        "LEFTCTRL" | "CTRL" => Some(Key::LeftCtrl),
        "RIGHTCTRL" => Some(Key::RightCtrl),
        "LEFTALT" | "ALT" => Some(Key::LeftAlt),
        "RIGHTALT" => Some(Key::RightAlt),
        "TAB" => Some(Key::Tab),
        "BACKSPACE" => Some(Key::Backspace),
        "NUMPAD0" | "KP0" | "0" => Some(Key::NumPad0),
        "NUMPAD1" | "KP1" | "1" => Some(Key::NumPad1),
        "NUMPAD2" | "KP2" | "2" => Some(Key::NumPad2),
        "NUMPAD3" | "KP3" | "3" => Some(Key::NumPad3),
        "NUMPAD4" | "KP4" | "4" => Some(Key::NumPad4),
        "NUMPAD5" | "KP5" | "5" => Some(Key::NumPad5),
        "NUMPAD6" | "KP6" | "6" => Some(Key::NumPad6),
        "NUMPAD7" | "KP7" | "7" => Some(Key::NumPad7),
        "NUMPAD8" | "KP8" | "8" => Some(Key::NumPad8),
        "NUMPAD9" | "KP9" | "9" => Some(Key::NumPad9),
        _ => None,
    }
}

#[derive(Debug, Clone)]
pub struct KeyBindings {
    pub profile: InputProfile,
    pub up: Key,
    pub down: Key,
    pub left: Key,
    pub right: Key,
    pub btn1: Key,
    pub btn2: Key,
}

impl KeyBindings {
    pub fn new() -> Self {
        let mut bindings = Self {
            profile: InputProfile::Standard,
            up: Key::Up,
            down: Key::Down,
            left: Key::Left,
            right: Key::Right,
            btn1: Key::Z,
            btn2: Key::X,
        };
        bindings.load_from_ini();
        bindings
    }

    pub fn set_profile(&mut self, profile: InputProfile) {
        self.profile = profile;
        match profile {
            InputProfile::Standard => {
                self.up = Key::Up;
                self.down = Key::Down;
                self.left = Key::Left;
                self.right = Key::Right;
                self.btn1 = Key::Z;
                self.btn2 = Key::X;
            }
            InputProfile::Wasd => {
                self.up = Key::W;
                self.down = Key::S;
                self.left = Key::A;
                self.right = Key::D;
                self.btn1 = Key::J;
                self.btn2 = Key::K;
            }
            InputProfile::Numpad => {
                self.up = Key::NumPad8;
                self.down = Key::NumPad2;
                self.left = Key::NumPad4;
                self.right = Key::NumPad6;
                self.btn1 = Key::NumPad1;
                self.btn2 = Key::NumPad0;
            }
            InputProfile::Custom => {
                // Keep custom key configuration intact
            }
        }
        self.save_to_ini();
    }

    pub fn save_to_ini(&self) {
        let prof_str = match self.profile {
            InputProfile::Standard => "Standard",
            InputProfile::Wasd => "WASD",
            InputProfile::Numpad => "Numpad",
            InputProfile::Custom => "Custom",
        };
        let content = format!(
            "# Configurazione Comandi OpenCDi\n\
             # Profilo: Standard | WASD | Numpad | Custom\n\
             profile={}\n\n\
             # Tasti personalizzati (attivi con profile=Custom o come override):\n\
             up={}\n\
             down={}\n\
             left={}\n\
             right={}\n\
             btn1={}\n\
             btn2={}\n",
            prof_str,
            key_to_string(self.up),
            key_to_string(self.down),
            key_to_string(self.left),
            key_to_string(self.right),
            key_to_string(self.btn1),
            key_to_string(self.btn2),
        );
        let _ = std::fs::write("opencdi_controls.ini", content);
    }

    pub fn load_from_ini(&mut self) {
        if let Ok(content) = std::fs::read_to_string("opencdi_controls.ini") {
            let mut detected_custom = false;
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with(';') {
                    continue;
                }
                let parts: Vec<&str> = trimmed.split('=').map(|s| s.trim()).collect();
                if parts.len() == 2 {
                    match parts[0].to_lowercase().as_str() {
                        "profile" => match parts[1].to_lowercase().as_str() {
                            "wasd" => self.set_profile(InputProfile::Wasd),
                            "numpad" => self.set_profile(InputProfile::Numpad),
                            "custom" => {
                                self.profile = InputProfile::Custom;
                                detected_custom = true;
                            }
                            _ => self.set_profile(InputProfile::Standard),
                        },
                        "up" => if let Some(k) = string_to_key(parts[1]) { self.up = k; detected_custom = true; }
                        "down" => if let Some(k) = string_to_key(parts[1]) { self.down = k; detected_custom = true; }
                        "left" => if let Some(k) = string_to_key(parts[1]) { self.left = k; detected_custom = true; }
                        "right" => if let Some(k) = string_to_key(parts[1]) { self.right = k; detected_custom = true; }
                        "btn1" => if let Some(k) = string_to_key(parts[1]) { self.btn1 = k; detected_custom = true; }
                        "btn2" => if let Some(k) = string_to_key(parts[1]) { self.btn2 = k; detected_custom = true; }
                        _ => {}
                    }
                }
            }
            if detected_custom && self.profile == InputProfile::Custom {
                info!("Comandi personalizzati caricati da opencdi_controls.ini: up={:?}, down={:?}, left={:?}, right={:?}, btn1={:?}, btn2={:?}",
                    self.up, self.down, self.left, self.right, self.btn1, self.btn2);
            }
        } else {
            self.save_to_ini();
        }
    }
}

pub enum GuiAction {
    InsertDisc(PathBuf),
    OpenDisc(PathBuf),
    EjectDisc,
    Reset,
    Exit,
    SetProfile(InputProfile),
    SetInputDevice(InputDeviceMode),
    ReloadControls,
    ToggleReleaseMouse,
    ToggleMute,
    VolumeUp,
    VolumeDown,
    ToggleFullscreen,
    TakeScreenshot,
    SetVideoMode(crate::mcd212::VideoMode),
    ToggleWaitStates,
    ShowHelp,
    ShowAbout,
}

static PENDING_ACTIONS: Mutex<Vec<GuiAction>> = Mutex::new(Vec::new());
static mut PREV_WNDPROC: isize = 0;
static mut MENU_HANDLE: *mut std::ffi::c_void = std::ptr::null_mut();
static mut OWNER_HWND: *mut std::ffi::c_void = std::ptr::null_mut();

pub fn pop_action() -> Option<GuiAction> {
    let mut actions = PENDING_ACTIONS.lock().unwrap();
    if actions.is_empty() {
        None
    } else {
        Some(actions.remove(0))
    }
}

pub fn push_action(action: GuiAction) {
    PENDING_ACTIONS.lock().unwrap().push(action);
}

pub fn update_input_device_menu(mode: InputDeviceMode) {
    unsafe {
        if !MENU_HANDLE.is_null() {
            CheckMenuItem(
                MENU_HANDLE,
                ID_INPUT_DEVICE_GAMEPAD as u32,
                MF_BYCOMMAND | if mode == InputDeviceMode::Gamepad { MF_CHECKED } else { MF_UNCHECKED },
            );
            CheckMenuItem(
                MENU_HANDLE,
                ID_INPUT_DEVICE_MOUSE as u32,
                MF_BYCOMMAND | if mode == InputDeviceMode::Mouse { MF_CHECKED } else { MF_UNCHECKED },
            );
            if !OWNER_HWND.is_null() {
                DrawMenuBar(OWNER_HWND);
            }
        }
    }
}

pub fn update_profile_menu(profile: InputProfile) {
    unsafe {
        if !MENU_HANDLE.is_null() {
            CheckMenuItem(
                MENU_HANDLE,
                ID_PROFILE_STANDARD as u32,
                MF_BYCOMMAND | if profile == InputProfile::Standard { MF_CHECKED } else { MF_UNCHECKED },
            );
            CheckMenuItem(
                MENU_HANDLE,
                ID_PROFILE_WASD as u32,
                MF_BYCOMMAND | if profile == InputProfile::Wasd { MF_CHECKED } else { MF_UNCHECKED },
            );
            CheckMenuItem(
                MENU_HANDLE,
                ID_PROFILE_NUMPAD as u32,
                MF_BYCOMMAND | if profile == InputProfile::Numpad { MF_CHECKED } else { MF_UNCHECKED },
            );
            CheckMenuItem(
                MENU_HANDLE,
                ID_PROFILE_CUSTOM as u32,
                MF_BYCOMMAND | if profile == InputProfile::Custom { MF_CHECKED } else { MF_UNCHECKED },
            );
            if !OWNER_HWND.is_null() {
                DrawMenuBar(OWNER_HWND);
            }
        }
    }
}

pub fn update_video_menu(mode: crate::mcd212::VideoMode) {
    unsafe {
        if !MENU_HANDLE.is_null() {
            CheckMenuItem(
                MENU_HANDLE,
                ID_VIDEO_PAL as u32,
                MF_BYCOMMAND | if mode == crate::mcd212::VideoMode::Pal { MF_CHECKED } else { MF_UNCHECKED },
            );
            CheckMenuItem(
                MENU_HANDLE,
                ID_VIDEO_NTSC as u32,
                MF_BYCOMMAND | if mode == crate::mcd212::VideoMode::Ntsc { MF_CHECKED } else { MF_UNCHECKED },
            );
            if !OWNER_HWND.is_null() {
                DrawMenuBar(OWNER_HWND);
            }
        }
    }
}

pub fn update_waitstates_menu(enabled: bool) {
    unsafe {
        if !MENU_HANDLE.is_null() {
            CheckMenuItem(
                MENU_HANDLE,
                ID_EMU_WAITSTATES as u32,
                MF_BYCOMMAND | if enabled { MF_CHECKED } else { MF_UNCHECKED },
            );
            if !OWNER_HWND.is_null() {
                DrawMenuBar(OWNER_HWND);
            }
        }
    }
}

unsafe extern "system" fn custom_wndproc(
    hwnd: *mut std::ffi::c_void,
    msg: u32,
    wparam: usize,
    lparam: isize,
) -> isize {
    if msg == WM_COMMAND {
        let menu_id = (wparam & 0xFFFF) as usize;
        match menu_id {
            ID_FILE_INSERT => {
                if let Some(path) = open_file_dialog(hwnd) {
                    push_action(GuiAction::InsertDisc(path));
                }
                return 0;
            }
            ID_FILE_OPEN => {
                if let Some(path) = open_file_dialog(hwnd) {
                    push_action(GuiAction::OpenDisc(path));
                }
                return 0;
            }
            ID_FILE_EJECT => {
                push_action(GuiAction::EjectDisc);
                return 0;
            }
            ID_FILE_RESET => {
                push_action(GuiAction::Reset);
                return 0;
            }
            ID_FILE_EXIT => {
                push_action(GuiAction::Exit);
                return 0;
            }
            ID_PROFILE_STANDARD => {
                push_action(GuiAction::SetProfile(InputProfile::Standard));
                return 0;
            }
            ID_PROFILE_WASD => {
                push_action(GuiAction::SetProfile(InputProfile::Wasd));
                return 0;
            }
            ID_PROFILE_NUMPAD => {
                push_action(GuiAction::SetProfile(InputProfile::Numpad));
                return 0;
            }
            ID_PROFILE_CUSTOM => {
                push_action(GuiAction::SetProfile(InputProfile::Custom));
                return 0;
            }
            ID_INPUT_DEVICE_GAMEPAD => {
                push_action(GuiAction::SetInputDevice(InputDeviceMode::Gamepad));
                return 0;
            }
            ID_INPUT_DEVICE_MOUSE => {
                push_action(GuiAction::SetInputDevice(InputDeviceMode::Mouse));
                return 0;
            }
            ID_CONTROLS_EDIT_INI => {
                let _ = std::process::Command::new("notepad.exe").arg("opencdi_controls.ini").spawn();
                push_action(GuiAction::SetProfile(InputProfile::Custom));
                return 0;
            }
            ID_CONTROLS_RELOAD_INI => {
                push_action(GuiAction::ReloadControls);
                return 0;
            }
            ID_CONTROLS_INFO => {
                show_controls_dialog(hwnd);
                return 0;
            }
            ID_CONTROLS_RELEASE_MOUSE => {
                push_action(GuiAction::ToggleReleaseMouse);
                return 0;
            }
            ID_AUDIO_MUTE => {
                push_action(GuiAction::ToggleMute);
                return 0;
            }
            ID_AUDIO_VOL_UP => {
                push_action(GuiAction::VolumeUp);
                return 0;
            }
            ID_AUDIO_VOL_DOWN => {
                push_action(GuiAction::VolumeDown);
                return 0;
            }
            ID_VIDEO_FULLSCREEN => {
                push_action(GuiAction::ToggleFullscreen);
                return 0;
            }
            ID_VIDEO_SCREENSHOT => {
                push_action(GuiAction::TakeScreenshot);
                return 0;
            }
            ID_VIDEO_PAL => {
                push_action(GuiAction::SetVideoMode(crate::mcd212::VideoMode::Pal));
                return 0;
            }
            ID_VIDEO_NTSC => {
                push_action(GuiAction::SetVideoMode(crate::mcd212::VideoMode::Ntsc));
                return 0;
            }
            ID_EMU_WAITSTATES => {
                push_action(GuiAction::ToggleWaitStates);
                return 0;
            }
            ID_HELP_CONTROLS => {
                show_controls_dialog(hwnd);
                return 0;
            }
            ID_HELP_ABOUT => {
                show_about_dialog(hwnd);
                return 0;
            }
            _ => {}
        }
    }

    unsafe {
        if PREV_WNDPROC != 0 {
            CallWindowProcW(PREV_WNDPROC, hwnd, msg, wparam, lparam)
        } else {
            0
        }
    }
}

pub fn attach_gui_menu(
    hwnd: *mut std::ffi::c_void,
    current_profile: InputProfile,
    current_device_mode: InputDeviceMode,
    current_video_mode: crate::mcd212::VideoMode,
) {
    if hwnd.is_null() {
        return;
    }

    unsafe {
        OWNER_HWND = hwnd;

        let main_menu = CreateMenu();
        if main_menu.is_null() {
            return;
        }
        MENU_HANDLE = main_menu;

        // 1. Menu File
        let file_menu = CreatePopupMenu();
        AppendMenuW(file_menu, MF_STRING, ID_FILE_INSERT, to_wide("Inserisci Disco nel Lettore (a caldo)...\tCtrl+I").as_ptr());
        AppendMenuW(file_menu, MF_STRING, ID_FILE_OPEN, to_wide("Apri Disco e Riavvia Console...\tCtrl+O").as_ptr());
        AppendMenuW(file_menu, MF_STRING, ID_FILE_EJECT, to_wide("Espelli Disco (Eject)").as_ptr());
        AppendMenuW(file_menu, MF_SEPARATOR, 0, std::ptr::null());
        AppendMenuW(file_menu, MF_STRING, ID_FILE_RESET, to_wide("Riavvia Console (Reset)\tF5").as_ptr());
        AppendMenuW(file_menu, MF_SEPARATOR, 0, std::ptr::null());
        AppendMenuW(file_menu, MF_STRING, ID_FILE_EXIT, to_wide("Esci\tAlt+F4").as_ptr());
        AppendMenuW(main_menu, MF_POPUP, file_menu as usize, to_wide("File").as_ptr());

        // 2. Menu Comandi
        let controls_menu = CreatePopupMenu();

        let device_menu = CreatePopupMenu();
        AppendMenuW(device_menu, MF_STRING, ID_INPUT_DEVICE_GAMEPAD, to_wide("Controller CD-i (Gamepad USB / Tastiera)").as_ptr());
        AppendMenuW(device_menu, MF_STRING, ID_INPUT_DEVICE_MOUSE, to_wide("Mouse Philips CD-i (Cursore Host)").as_ptr());
        AppendMenuW(controls_menu, MF_POPUP, device_menu as usize, to_wide("Dispositivo di Controllo").as_ptr());
        AppendMenuW(controls_menu, MF_SEPARATOR, 0, std::ptr::null());

        AppendMenuW(controls_menu, MF_STRING, ID_PROFILE_STANDARD, to_wide("Profilo Tasti: Standard (Frecce + Z / X)").as_ptr());
        AppendMenuW(controls_menu, MF_STRING, ID_PROFILE_WASD, to_wide("Profilo Tasti: WASD + J / K (Stile PC)").as_ptr());
        AppendMenuW(controls_menu, MF_STRING, ID_PROFILE_NUMPAD, to_wide("Profilo Tasti: Tastierino Numerico (Numpad)").as_ptr());
        AppendMenuW(controls_menu, MF_STRING, ID_PROFILE_CUSTOM, to_wide("Profilo Tasti: Personalizzato (opencdi_controls.ini)").as_ptr());
        AppendMenuW(controls_menu, MF_SEPARATOR, 0, std::ptr::null());
        AppendMenuW(controls_menu, MF_STRING, ID_CONTROLS_EDIT_INI, to_wide("Modifica Comandi in Notepad...").as_ptr());
        AppendMenuW(controls_menu, MF_STRING, ID_CONTROLS_RELOAD_INI, to_wide("Ricarica Comandi da File INI").as_ptr());
        AppendMenuW(controls_menu, MF_SEPARATOR, 0, std::ptr::null());
        AppendMenuW(controls_menu, MF_STRING, ID_CONTROLS_RELEASE_MOUSE, to_wide("Libera / Cattura Mouse\tTasto Centrale").as_ptr());
        AppendMenuW(controls_menu, MF_STRING, ID_CONTROLS_INFO, to_wide("Visualizza Configurazione Tasti...").as_ptr());
        AppendMenuW(main_menu, MF_POPUP, controls_menu as usize, to_wide("Comandi").as_ptr());

        // 3. Menu Audio
        let audio_menu = CreatePopupMenu();
        AppendMenuW(audio_menu, MF_STRING, ID_AUDIO_MUTE, to_wide("Muto / Riattiva Audio\tM").as_ptr());
        AppendMenuW(audio_menu, MF_STRING, ID_AUDIO_VOL_UP, to_wide("Volume Su (+)\t+").as_ptr());
        AppendMenuW(audio_menu, MF_STRING, ID_AUDIO_VOL_DOWN, to_wide("Volume Giù (-)\t-").as_ptr());
        AppendMenuW(main_menu, MF_POPUP, audio_menu as usize, to_wide("Audio").as_ptr());

        // 4. Menu Video
        let video_menu = CreatePopupMenu();
        AppendMenuW(video_menu, MF_STRING, ID_VIDEO_FULLSCREEN, to_wide("Schermo Intero\tF11").as_ptr());
        AppendMenuW(video_menu, MF_STRING, ID_VIDEO_SCREENSHOT, to_wide("Salva Screenshot\tF12").as_ptr());
        AppendMenuW(video_menu, MF_SEPARATOR, 0, std::ptr::null());
        AppendMenuW(video_menu, MF_STRING, ID_VIDEO_PAL, to_wide("PAL (50 Hz) [Standard CD-i 220]").as_ptr());
        AppendMenuW(video_menu, MF_STRING, ID_VIDEO_NTSC, to_wide("NTSC (60 Hz)").as_ptr());
        AppendMenuW(video_menu, MF_SEPARATOR, 0, std::ptr::null());
        AppendMenuW(video_menu, MF_STRING | MF_CHECKED, ID_EMU_WAITSTATES, to_wide("Velocità Hardware Reale CD-i (Wait-States)").as_ptr());
        AppendMenuW(main_menu, MF_POPUP, video_menu as usize, to_wide("Video").as_ptr());

        // 5. Menu Aiuto
        let help_menu = CreatePopupMenu();
        AppendMenuW(help_menu, MF_STRING, ID_HELP_CONTROLS, to_wide("Guida Comandi CD-i...").as_ptr());
        AppendMenuW(help_menu, MF_STRING, ID_HELP_ABOUT, to_wide("Informazioni su OpenCDi...").as_ptr());
        AppendMenuW(main_menu, MF_POPUP, help_menu as usize, to_wide("Aiuto").as_ptr());

        SetMenu(hwnd, main_menu);
        DrawMenuBar(hwnd);

        update_input_device_menu(current_device_mode);
        update_profile_menu(current_profile);
        update_video_menu(current_video_mode);

        // Subclass window procedure
        let prev = SetWindowLongPtrW(hwnd, GWLP_WNDPROC, custom_wndproc as *const () as usize as isize);
        PREV_WNDPROC = prev;
        info!("Win32 Native Menu attached and window subclassed successfully.");
    }
}

pub fn open_file_dialog(owner: *mut std::ffi::c_void) -> Option<PathBuf> {
    let mut file_buf = [0u16; 1024];

    // Filter string: "Giochi Philips CD-i (*.chd, *.iso, *.bin, *.cue)\0*.chd;*.iso;*.bin;*.cue\0Tutti i file (*.*)\0*.*\0\0"
    let mut filter: Vec<u16> = Vec::new();
    filter.extend(OsStr::new("Giochi CD-i (*.chd;*.iso;*.bin;*.cue)").encode_wide());
    filter.push(0);
    filter.extend(OsStr::new("*.chd;*.iso;*.bin;*.cue").encode_wide());
    filter.push(0);
    filter.extend(OsStr::new("Tutti i file (*.*)").encode_wide());
    filter.push(0);
    filter.extend(OsStr::new("*.*").encode_wide());
    filter.push(0);
    filter.push(0);

    let title = to_wide("Seleziona Gioco CD-i (CHD / ISO / BIN)");

    let mut ofn: OPENFILENAMEW = unsafe { std::mem::zeroed() };
    ofn.lStructSize = std::mem::size_of::<OPENFILENAMEW>() as u32;
    ofn.hwndOwner = owner;
    ofn.lpstrFilter = filter.as_ptr();
    ofn.lpstrFile = file_buf.as_mut_ptr();
    ofn.nMaxFile = file_buf.len() as u32;
    ofn.lpstrTitle = title.as_ptr();
    ofn.Flags = 0x00000800 | 0x00001000 | 0x00000004; // OFN_PATHMUSTEXIST | OFN_FILEMUSTEXIST | OFN_HIDEREADONLY

    unsafe {
        if GetOpenFileNameW(&mut ofn) != 0 {
            let len = file_buf.iter().position(|&c| c == 0).unwrap_or(file_buf.len());
            let os_string = OsString::from_wide(&file_buf[..len]);
            Some(PathBuf::from(os_string))
        } else {
            None
        }
    }
}

pub fn show_controls_dialog(owner: *mut std::ffi::c_void) {
    let text = to_wide(
        "OpenCDi - Configurazione Comandi Philips CD-i\n\
        =================================================\n\n\
        Sono disponibili profili rapidi selezionabili dal menu Comandi:\n\n\
        1. PROFILO STANDARD (Predefinito):\n\
           • Movimento / D-Pad: Frecce Direzionali\n\
           • Tasto 1 (Azione / Conferma): [Z]\n\
           • Tasto 2 (Salto / Azione 2): [X]\n\n\
        2. PROFILO WASD (Stile PC):\n\
           • Movimento: [W] [A] [S] [D]\n\
           • Tasto 1: [J]  |  Tasto 2: [K]\n\n\
        3. PROFILO TASTIERINO (Numpad):\n\
           • Movimento: [8] [4] [6] [2]\n\
           • Tasto 1: [1]  |  Tasto 2: [0]\n\n\
        4. PROFILO PERSONALIZZATO:\n\
           • Modificabile tramite 'Comandi -> Modifica Comandi in Notepad...'\n\
           • Salva in opencdi_controls.ini qualsiasi tasto (es. Space, Enter, Shift...)\n\n\
        MOUSE E TOUCH:\n\
           • Muovi il mouse per muovere la freccia o il cursore.\n\
           • Clic Sinistro = Tasto 1  |  Clic Destro = Tasto 2\n\
           • Tasto Centrale: Sblocca / blocca il cursore dalla finestra.\n\n\
        GAMEPAD USB / XBOX / PLAYSTATION:\n\
           • Croce Direzionale o Levetta Analogica Sinistra = Movimento\n\
           • Tasto A / Croce = Tasto 1\n\
           • Tasto B / Cerchio / X = Tasto 2\n\
           • Tasto Start = Azione / Pausa\n\n\
        SCORCIATOIE RAPIDE:\n\
           • F12: Salva Screenshot (screenshots_hotel_mario/)\n\
           • Ctrl+O: Apri file gioco (CHD / ISO / BIN)\n\
           • F5: Riavvia console (Reset)\n\
           • F11: Schermo Intero\n\
           • M: Muto / Riattiva audio\n\
           • + / - : Regola volume"
    );
    let title = to_wide("OpenCDi - Guida e Mappatura Comandi");
    unsafe {
        MessageBoxW(owner, text.as_ptr(), title.as_ptr(), MB_OK | MB_ICONINFORMATION);
    }
}

pub fn show_about_dialog(owner: *mut std::ffi::c_void) {
    let text = to_wide(
        "OpenCDi - Emulatore Philips CD-i\n\
        Versione: 0.2.0 (High Performance Edition)\n\
        Sviluppato con Rust & MiniFB\n\n\
        Caratteristiche implementate:\n\
        • Microprocessore SCC68070 & OS-9 CD-RTOS\n\
        • Controller Video MCD212 con piani A & B e supporto bitmap\n\
        • Decoder Audio CD-i ADPCM Level B Stereo con riempimento mappe audio SoundMap\n\
        • Interfaccia grafica nativa Windows con menu di controllo e selezione file\n\
        • Supporto Gamepad USB (DirectInput / XInput) e cattura mouse precisa"
    );
    let title = to_wide("Informazioni su OpenCDi");
    unsafe {
        MessageBoxW(owner, text.as_ptr(), title.as_ptr(), MB_OK | MB_ICONINFORMATION);
    }
}
