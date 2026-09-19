use serde::{Deserialize, Serialize};

/// A global keyboard shortcut. `vk` is a Windows virtual-key code; on Linux
/// the same numeric value is kept in config so a Windows install can be copied
/// over, but Linux never registers it (the desktop environment owns PrintScreen).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hotkey {
    #[serde(default)]
    pub ctrl: bool,
    #[serde(default)]
    pub alt: bool,
    #[serde(default)]
    pub shift: bool,
    #[serde(default)]
    pub win: bool,
    pub vk: u32,
}

impl Hotkey {
    #[allow(dead_code)]
    pub const PRINT_SCREEN: Self = Self {
        ctrl: false,
        alt: false,
        shift: false,
        win: false,
        vk: 0x2C,
    };

    pub fn ctrl_shift_s() -> Self {
        Self {
            ctrl: true,
            alt: false,
            shift: true,
            win: false,
            vk: 0x53,
        }
    }

    pub fn ctrl_alt_s() -> Self {
        Self {
            ctrl: true,
            alt: true,
            shift: false,
            win: false,
            vk: 0x53,
        }
    }

    pub fn display(&self) -> String {
        let mut parts = Vec::new();
        if self.ctrl {
            parts.push("Ctrl");
        }
        if self.alt {
            parts.push("Alt");
        }
        if self.shift {
            parts.push("Shift");
        }
        if self.win {
            parts.push("Win");
        }
        parts.push(vk_name(self.vk));
        parts.join(" + ")
    }

    /// Win32 `RegisterHotKey` modifier bitmask (without `MOD_NOREPEAT`).
    #[cfg(windows)]
    pub fn native_modifiers(&self) -> u32 {
        let mut m = 0u32;
        if self.alt {
            m |= 0x0001; // MOD_ALT
        }
        if self.ctrl {
            m |= 0x0002; // MOD_CONTROL
        }
        if self.shift {
            m |= 0x0004; // MOD_SHIFT
        }
        if self.win {
            m |= 0x0008; // MOD_WIN
        }
        m
    }
}

fn vk_name(vk: u32) -> &'static str {
    match vk {
        0x08 => "Backspace",
        0x09 => "Tab",
        0x0D => "Enter",
        0x1B => "Esc",
        0x20 => "Space",
        0x21 => "Page Up",
        0x22 => "Page Down",
        0x23 => "End",
        0x24 => "Home",
        0x25 => "Left",
        0x26 => "Up",
        0x27 => "Right",
        0x28 => "Down",
        0x2C => "Print Screen",
        0x2D => "Insert",
        0x2E => "Delete",
        0x30 => "0",
        0x31 => "1",
        0x32 => "2",
        0x33 => "3",
        0x34 => "4",
        0x35 => "5",
        0x36 => "6",
        0x37 => "7",
        0x38 => "8",
        0x39 => "9",
        0x41 => "A",
        0x42 => "B",
        0x43 => "C",
        0x44 => "D",
        0x45 => "E",
        0x46 => "F",
        0x47 => "G",
        0x48 => "H",
        0x49 => "I",
        0x4A => "J",
        0x4B => "K",
        0x4C => "L",
        0x4D => "M",
        0x4E => "N",
        0x4F => "O",
        0x50 => "P",
        0x51 => "Q",
        0x52 => "R",
        0x53 => "S",
        0x54 => "T",
        0x55 => "U",
        0x56 => "V",
        0x57 => "W",
        0x58 => "X",
        0x59 => "Y",
        0x5A => "Z",
        0x70 => "F1",
        0x71 => "F2",
        0x72 => "F3",
        0x73 => "F4",
        0x74 => "F5",
        0x75 => "F6",
        0x76 => "F7",
        0x77 => "F8",
        0x78 => "F9",
        0x79 => "F10",
        0x7A => "F11",
        0x7B => "F12",
        _ => "Key",
    }
}

/// Maps an egui key + modifiers onto a hotkey. Returns `None` for keys that
/// should not be bound (modifiers alone, Escape while cancelling, etc.).
pub fn from_egui(key: eframe::egui::Key, modifiers: eframe::egui::Modifiers) -> Option<Hotkey> {
    use eframe::egui::Key;
    let vk = match key {
        Key::Escape => return None,
        Key::Tab => 0x09,
        Key::Enter => 0x0D,
        Key::Space => 0x20,
        Key::Backspace => 0x08,
        Key::Insert => 0x2D,
        Key::Delete => 0x2E,
        Key::Home => 0x24,
        Key::End => 0x23,
        Key::PageUp => 0x21,
        Key::PageDown => 0x22,
        Key::ArrowLeft => 0x25,
        Key::ArrowUp => 0x26,
        Key::ArrowRight => 0x27,
        Key::ArrowDown => 0x28,
        Key::A => 0x41,
        Key::B => 0x42,
        Key::C => 0x43,
        Key::D => 0x44,
        Key::E => 0x45,
        Key::F => 0x46,
        Key::G => 0x47,
        Key::H => 0x48,
        Key::I => 0x49,
        Key::J => 0x4A,
        Key::K => 0x4B,
        Key::L => 0x4C,
        Key::M => 0x4D,
        Key::N => 0x4E,
        Key::O => 0x4F,
        Key::P => 0x50,
        Key::Q => 0x51,
        Key::R => 0x52,
        Key::S => 0x53,
        Key::T => 0x54,
        Key::U => 0x55,
        Key::V => 0x56,
        Key::W => 0x57,
        Key::X => 0x58,
        Key::Y => 0x59,
        Key::Z => 0x5A,
        Key::Num0 => 0x30,
        Key::Num1 => 0x31,
        Key::Num2 => 0x32,
        Key::Num3 => 0x33,
        Key::Num4 => 0x34,
        Key::Num5 => 0x35,
        Key::Num6 => 0x36,
        Key::Num7 => 0x37,
        Key::Num8 => 0x38,
        Key::Num9 => 0x39,
        Key::F1 => 0x70,
        Key::F2 => 0x71,
        Key::F3 => 0x72,
        Key::F4 => 0x73,
        Key::F5 => 0x74,
        Key::F6 => 0x75,
        Key::F7 => 0x76,
        Key::F8 => 0x77,
        Key::F9 => 0x78,
        Key::F10 => 0x79,
        Key::F11 => 0x7A,
        Key::F12 => 0x7B,
        _ => return None,
    };
    Some(Hotkey {
        ctrl: modifiers.ctrl || modifiers.command,
        alt: modifiers.alt,
        shift: modifiers.shift,
        win: modifiers.mac_cmd,
        vk,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn print_screen_displays_cleanly() {
        assert_eq!(Hotkey::PRINT_SCREEN.display(), "Print Screen");
    }

    #[test]
    fn combo_displays_in_lightshot_order() {
        assert_eq!(Hotkey::ctrl_shift_s().display(), "Ctrl + Shift + S");
        assert_eq!(Hotkey::ctrl_alt_s().display(), "Ctrl + Alt + S");
    }
}
