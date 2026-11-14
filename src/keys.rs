use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;

#[derive(Debug, Clone)]
pub struct Hotkey {
    #[allow(dead_code)]
    pub display_name: String,
    pub keys: Vec<i32>,
}

impl Hotkey {
    pub fn parse(input: &str) -> Self {
        let trimmed = input.trim();
        if trimmed.is_empty()
            || trimmed.eq_ignore_ascii_case("none")
            || trimmed.eq_ignore_ascii_case("нет")
        {
            return Self {
                display_name: "Не назначено".to_string(),
                keys: Vec::new(),
            };
        }

        let mut keys = Vec::new();
        if let Some(vk) = parse_single_key(trimmed) {
            keys.push(vk);
        } else {
            let parts: Vec<&str> = trimmed.split('+').map(|s| s.trim()).collect();
            for part in parts {
                if part.is_empty() {
                    continue;
                }
                if let Some(vk) = parse_single_key(part) {
                    keys.push(vk);
                } else {
                    eprintln!("Предупреждение: Неизвестная клавиша '{}', пропущена", part);
                }
            }
        }

        Self {
            display_name: if keys.is_empty() {
                "Не назначено".to_string()
            } else {
                trimmed.to_uppercase()
            },
            keys,
        }
    }

    pub fn is_bound(&self) -> bool {
        !self.keys.is_empty()
    }

    pub fn is_pressed(&self) -> bool {
        if self.keys.is_empty() {
            return false;
        }
        unsafe {
            for &vk in &self.keys {
                let state = GetAsyncKeyState(vk);
                if (state as u16 & 0x8000) == 0 {
                    return false;
                }
            }
        }
        true
    }
}

pub fn same_hotkey(a: &str, b: &str) -> bool {
    let ha = Hotkey::parse(a);
    let hb = Hotkey::parse(b);
    !ha.keys.is_empty() && ha.keys == hb.keys
}

pub fn parse_single_key(name: &str) -> Option<i32> {
    let raw = name.trim();
    let lower = raw.to_lowercase();

    // 1. Check for "key_186", "vk_186" or pure numeric "186"
    if let Some(num_str) = lower
        .strip_prefix("key_")
        .or_else(|| lower.strip_prefix("vk_"))
    {
        if let Ok(code) = num_str.parse::<i32>() {
            return Some(code);
        }
    }
    if let Ok(code) = lower.parse::<i32>() {
        return Some(code);
    }

    // 2. Named keys & symbols
    match lower.as_str() {
        // Mouse Buttons
        "mouse1" | "lbutton" | "leftclick" | "lmb" | "лкм" => Some(VK_LBUTTON as i32),
        "mouse2" | "rbutton" | "rightclick" | "rmb" | "пкм" => Some(VK_RBUTTON as i32),
        "mouse3" | "mbutton" | "middleclick" | "mmb" | "скм" | "колесико" => {
            Some(VK_MBUTTON as i32)
        }
        "mouse4" | "xbutton1" | "m4" => Some(VK_XBUTTON1 as i32),
        "mouse5" | "xbutton2" | "m5" => Some(VK_XBUTTON2 as i32),

        // Modifiers & System Keys
        "caps" | "capslock" | "capital" => Some(VK_CAPITAL as i32),
        "shift" => Some(VK_SHIFT as i32),
        "lshift" => Some(VK_LSHIFT as i32),
        "rshift" => Some(VK_RSHIFT as i32),
        "ctrl" | "control" => Some(VK_CONTROL as i32),
        "lctrl" => Some(VK_LCONTROL as i32),
        "rctrl" => Some(VK_RCONTROL as i32),
        "alt" | "menu" => Some(VK_MENU as i32),
        "lalt" => Some(VK_LMENU as i32),
        "ralt" => Some(VK_RMENU as i32),
        "space" | "пробел" | " " => Some(VK_SPACE as i32),
        "tab" => Some(VK_TAB as i32),
        "esc" | "escape" => Some(VK_ESCAPE as i32),
        "enter" | "return" => Some(VK_RETURN as i32),
        "backspace" | "back" => Some(VK_BACK as i32),

        // OEM & Punctuation (both symbols and English/Russian characters)
        "tilde" | "`" | "~" | "ё" => Some(VK_OEM_3 as i32),
        ";" | "semicolon" | ":" | "ж" => Some(VK_OEM_1 as i32),
        "'" | "quote" | "\"" | "э" => Some(VK_OEM_7 as i32),
        "[" | "bracketleft" | "{" | "х" => Some(VK_OEM_4 as i32),
        "]" | "bracketright" | "}" | "ъ" => Some(VK_OEM_6 as i32),
        "," | "comma" | "<" | "б" => Some(VK_OEM_COMMA as i32),
        "." | "period" | "dot" | ">" | "ю" => Some(VK_OEM_PERIOD as i32),
        "/" | "slash" | "?" => Some(VK_OEM_2 as i32),
        "\\" | "backslash" | "|" => Some(VK_OEM_5 as i32),
        "-" | "minus" | "_" => Some(VK_OEM_MINUS as i32),
        "=" | "plus" | "equal" | "+" => Some(VK_OEM_PLUS as i32),

        // Navigation & Editing
        "insert" | "ins" => Some(VK_INSERT as i32),
        "delete" | "del" => Some(VK_DELETE as i32),
        "home" => Some(VK_HOME as i32),
        "end" => Some(VK_END as i32),
        "pageup" | "pgup" => Some(VK_PRIOR as i32),
        "pagedown" | "pgdn" => Some(VK_NEXT as i32),
        "up" | "стрелкавверх" => Some(VK_UP as i32),
        "down" | "стрелкавниз" => Some(VK_DOWN as i32),
        "left" | "стрелкавлево" => Some(VK_LEFT as i32),
        "right" | "стрелкавправо" => Some(VK_RIGHT as i32),
        "scrolllock" | "scroll" => Some(VK_SCROLL as i32),
        "pause" => Some(VK_PAUSE as i32),

        // Function Keys F1 - F24
        "f1" => Some(VK_F1 as i32),
        "f2" => Some(VK_F2 as i32),
        "f3" => Some(VK_F3 as i32),
        "f4" => Some(VK_F4 as i32),
        "f5" => Some(VK_F5 as i32),
        "f6" => Some(VK_F6 as i32),
        "f7" => Some(VK_F7 as i32),
        "f8" => Some(VK_F8 as i32),
        "f9" => Some(VK_F9 as i32),
        "f10" => Some(VK_F10 as i32),
        "f11" => Some(VK_F11 as i32),
        "f12" => Some(VK_F12 as i32),
        "f13" => Some(VK_F13 as i32),
        "f14" => Some(VK_F14 as i32),
        "f15" => Some(VK_F15 as i32),
        "f16" => Some(VK_F16 as i32),
        "f17" => Some(VK_F17 as i32),
        "f18" => Some(VK_F18 as i32),
        "f19" => Some(VK_F19 as i32),
        "f20" => Some(VK_F20 as i32),
        "f21" => Some(VK_F21 as i32),
        "f22" => Some(VK_F22 as i32),
        "f23" => Some(VK_F23 as i32),
        "f24" => Some(VK_F24 as i32),

        // Numpad Keys
        "num0" | "numpad0" => Some(VK_NUMPAD0 as i32),
        "num1" | "numpad1" => Some(VK_NUMPAD1 as i32),
        "num2" | "numpad2" => Some(VK_NUMPAD2 as i32),
        "num3" | "numpad3" => Some(VK_NUMPAD3 as i32),
        "num4" | "numpad4" => Some(VK_NUMPAD4 as i32),
        "num5" | "numpad5" => Some(VK_NUMPAD5 as i32),
        "num6" | "numpad6" => Some(VK_NUMPAD6 as i32),
        "num7" | "numpad7" => Some(VK_NUMPAD7 as i32),
        "num8" | "numpad8" => Some(VK_NUMPAD8 as i32),
        "num9" | "numpad9" => Some(VK_NUMPAD9 as i32),
        "num*" | "multiply" => Some(VK_MULTIPLY as i32),
        "num+" | "add" => Some(VK_ADD as i32),
        "num-" | "subtract" => Some(VK_SUBTRACT as i32),
        "num." | "decimal" => Some(VK_DECIMAL as i32),
        "num/" | "divide" => Some(VK_DIVIDE as i32),

        // Single Character / Russian Layout Mapping
        other => {
            if other.len() == 1 {
                let ch = other.chars().next().unwrap();
                if ch.is_ascii_alphabetic() {
                    Some(ch.to_ascii_uppercase() as i32)
                } else if ch.is_ascii_digit() {
                    Some(ch as i32)
                } else {
                    None
                }
            } else {
                // Russian layout characters
                match other {
                    "й" => Some(b'Q' as i32),
                    "ц" => Some(b'W' as i32),
                    "у" => Some(b'E' as i32),
                    "к" => Some(b'R' as i32),
                    "е" => Some(b'T' as i32),
                    "н" => Some(b'Y' as i32),
                    "г" => Some(b'U' as i32),
                    "ш" => Some(b'I' as i32),
                    "щ" => Some(b'O' as i32),
                    "з" => Some(b'P' as i32),
                    "х" => Some(VK_OEM_4 as i32),
                    "ъ" => Some(VK_OEM_6 as i32),
                    "ф" => Some(b'A' as i32),
                    "ы" => Some(b'S' as i32),
                    "в" => Some(b'D' as i32),
                    "а" => Some(b'F' as i32),
                    "п" => Some(b'G' as i32),
                    "р" => Some(b'H' as i32),
                    "о" => Some(b'J' as i32),
                    "л" => Some(b'K' as i32),
                    "д" => Some(b'L' as i32),
                    "ж" => Some(VK_OEM_1 as i32),
                    "э" => Some(VK_OEM_7 as i32),
                    "я" => Some(b'Z' as i32),
                    "ч" => Some(b'X' as i32),
                    "с" => Some(b'C' as i32),
                    "м" => Some(b'V' as i32),
                    "и" => Some(b'B' as i32),
                    "т" => Some(b'N' as i32),
                    "ь" => Some(b'M' as i32),
                    "б" => Some(VK_OEM_COMMA as i32),
                    "ю" => Some(VK_OEM_PERIOD as i32),
                    _ => None,
                }
            }
        }
    }
}

pub fn vk_to_name(vk: i32) -> String {
    match vk as u16 {
        VK_LBUTTON => "LButton".to_string(),
        VK_RBUTTON => "RButton".to_string(),
        VK_MBUTTON => "MButton".to_string(),
        VK_XBUTTON1 => "Mouse4".to_string(),
        VK_XBUTTON2 => "Mouse5".to_string(),

        VK_CAPITAL => "Caps".to_string(),
        VK_SHIFT | VK_LSHIFT => "LShift".to_string(),
        VK_RSHIFT => "RShift".to_string(),
        VK_CONTROL | VK_LCONTROL => "LCtrl".to_string(),
        VK_RCONTROL => "RCtrl".to_string(),
        VK_MENU | VK_LMENU => "LAlt".to_string(),
        VK_RMENU => "RAlt".to_string(),
        VK_SPACE => "Space".to_string(),
        VK_TAB => "Tab".to_string(),
        VK_RETURN => "Enter".to_string(),
        VK_BACK => "Backspace".to_string(),

        // Punctuation & OEM keys
        VK_OEM_3 => "Tilde".to_string(),
        VK_OEM_1 => ";".to_string(),
        VK_OEM_7 => "'".to_string(),
        VK_OEM_4 => "[".to_string(),
        VK_OEM_6 => "]".to_string(),
        VK_OEM_COMMA => ",".to_string(),
        VK_OEM_PERIOD => ".".to_string(),
        VK_OEM_2 => "/".to_string(),
        VK_OEM_5 => "\\".to_string(),
        VK_OEM_MINUS => "-".to_string(),
        VK_OEM_PLUS => "=".to_string(),

        VK_INSERT => "Insert".to_string(),
        VK_DELETE => "Delete".to_string(),
        VK_HOME => "Home".to_string(),
        VK_END => "End".to_string(),
        VK_PRIOR => "PageUp".to_string(),
        VK_NEXT => "PageDown".to_string(),
        VK_UP => "Up".to_string(),
        VK_DOWN => "Down".to_string(),
        VK_LEFT => "Left".to_string(),
        VK_RIGHT => "Right".to_string(),
        VK_SCROLL => "ScrollLock".to_string(),
        VK_PAUSE => "Pause".to_string(),

        VK_F1 => "F1".to_string(),
        VK_F2 => "F2".to_string(),
        VK_F3 => "F3".to_string(),
        VK_F4 => "F4".to_string(),
        VK_F5 => "F5".to_string(),
        VK_F6 => "F6".to_string(),
        VK_F7 => "F7".to_string(),
        VK_F8 => "F8".to_string(),
        VK_F9 => "F9".to_string(),
        VK_F10 => "F10".to_string(),
        VK_F11 => "F11".to_string(),
        VK_F12 => "F12".to_string(),
        VK_F13 => "F13".to_string(),
        VK_F14 => "F14".to_string(),
        VK_F15 => "F15".to_string(),
        VK_F16 => "F16".to_string(),
        VK_F17 => "F17".to_string(),
        VK_F18 => "F18".to_string(),
        VK_F19 => "F19".to_string(),
        VK_F20 => "F20".to_string(),
        VK_F21 => "F21".to_string(),
        VK_F22 => "F22".to_string(),
        VK_F23 => "F23".to_string(),
        VK_F24 => "F24".to_string(),

        VK_NUMPAD0 => "Num0".to_string(),
        VK_NUMPAD1 => "Num1".to_string(),
        VK_NUMPAD2 => "Num2".to_string(),
        VK_NUMPAD3 => "Num3".to_string(),
        VK_NUMPAD4 => "Num4".to_string(),
        VK_NUMPAD5 => "Num5".to_string(),
        VK_NUMPAD6 => "Num6".to_string(),
        VK_NUMPAD7 => "Num7".to_string(),
        VK_NUMPAD8 => "Num8".to_string(),
        VK_NUMPAD9 => "Num9".to_string(),
        VK_MULTIPLY => "Num*".to_string(),
        VK_ADD => "Num+".to_string(),
        VK_SUBTRACT => "Num-".to_string(),
        VK_DECIMAL => "Num.".to_string(),
        VK_DIVIDE => "Num/".to_string(),

        0x41..=0x5A => ((vk as u8) as char).to_string(),
        0x30..=0x39 => ((vk as u8) as char).to_string(),

        _ => format!("Key_{}", vk),
    }
}

fn format_single_vk_label_lang(vk: i32, lang: crate::config::Language) -> String {
    match vk as u16 {
        VK_OEM_1 => ";".to_string(),
        VK_OEM_7 => "'".to_string(),
        VK_OEM_4 => "[".to_string(),
        VK_OEM_6 => "]".to_string(),
        VK_OEM_3 => "~".to_string(),
        VK_OEM_COMMA => ",".to_string(),
        VK_OEM_PERIOD => ".".to_string(),
        VK_LBUTTON => lang.t("ЛКМ", "LMB").to_string(),
        VK_RBUTTON => lang.t("ПКМ", "RMB").to_string(),
        VK_MBUTTON => lang.t("СКМ", "MMB").to_string(),
        _ => vk_to_name(vk),
    }
}

pub fn format_hotkey_display_lang(raw: &str, lang: crate::config::Language) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return lang.t("Не назначено", "Not set").to_string();
    }
    if let Some(vk) = parse_single_key(trimmed) {
        return format_single_vk_label_lang(vk, lang);
    }
    let parsed = Hotkey::parse(trimmed);
    if parsed.keys.is_empty() {
        return lang.t("Не назначено", "Not set").to_string();
    }
    parsed
        .keys
        .iter()
        .map(|&vk| format_single_vk_label_lang(vk, lang))
        .collect::<Vec<_>>()
        .join(" + ")
}

#[allow(dead_code)]
pub fn format_hotkey_display(raw: &str) -> String {
    format_hotkey_display_lang(raw, crate::config::Language::Ru)
}
