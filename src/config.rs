use crate::keys::same_hotkey;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::ptr::null_mut;

fn default_false() -> bool {
    false
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Language {
    #[serde(rename = "ru")]
    Ru,
    #[serde(rename = "en")]
    En,
}

impl Default for Language {
    fn default() -> Self {
        Language::Ru
    }
}

impl Language {
    pub fn t<'a>(self, ru: &'a str, en: &'a str) -> &'a str {
        match self {
            Language::Ru => ru,
            Language::En => en,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProfile {
    pub name: String,
    pub crosshair_only: bool,
    pub always_show_crosshair: bool,
    pub hide_crosshair_on_zoom: bool,
    pub zoom_factor: f32,
    pub scope_size: i32,
    pub shape: String,
    pub border_style: String,
    pub crosshair_code: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperatingMode {
    ZoomWithCrosshair,
    AlwaysCrosshairPlusZoom,
    CrosshairOnly,
    ZoomOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HotkeySlot {
    Zoom,
    Crosshair,
    ZoomOut,
    ZoomIn,
    SizeDec,
    SizeInc,
    ToggleNoZoom,
    CycleReticle,
    CycleShape,
    CycleColor,
}

impl HotkeySlot {
    pub const ALL: [HotkeySlot; 10] = [
        HotkeySlot::Zoom,
        HotkeySlot::Crosshair,
        HotkeySlot::ZoomOut,
        HotkeySlot::ZoomIn,
        HotkeySlot::SizeDec,
        HotkeySlot::SizeInc,
        HotkeySlot::ToggleNoZoom,
        HotkeySlot::CycleReticle,
        HotkeySlot::CycleShape,
        HotkeySlot::CycleColor,
    ];
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Режим зума: "hold" (удерживать), "toggle" (вкл/выкл по нажатию), "always" (постоянно включен)
    pub mode: String,

    /// Кнопка активации зума (например: "V", "C", "X", "Caps", "Mouse4", "Mouse5", "RButton", "MButton", "Alt+V")
    pub hotkey: String,

    /// Кнопка включения / выключения прицела (например: "F7", "X", "C", "Mouse5", "Caps", "Tilde")
    pub crosshair_hotkey: String,

    /// Настраиваемые горячие клавиши быстрой подстройки прямо в игре
    pub hk_zoom_out: String,
    pub hk_zoom_in: String,
    pub hk_size_dec: String,
    pub hk_size_inc: String,
    pub hk_toggle_no_zoom: String,
    pub hk_cycle_reticle: String,
    pub hk_cycle_shape: String,
    pub hk_cycle_color: String,

    /// Активен ли прицел на экране
    pub crosshair_enabled: bool,

    /// Режим кнопки прицела: "toggle" (нажал вкл / нажал выкл) или "hold" (удерживать)
    pub crosshair_mode: String,

    /// Коэффициент приближения (1.0 = без приближения, 1.1 - 10.0 = зум)
    pub zoom_factor: f32,

    /// Режим «Только прицел без приближения» (прозрачный оверлей перекрестия без линзы зума)
    pub crosshair_only: bool,

    /// Оставлять прозрачный прицел на экране постоянно, даже когда зум выключен (для стрельбы от бедра)
    pub always_show_crosshair: bool,

    /// Скрывать / выключать прицел при приближении (чистый зум без оверлейной сетки поверх игрового прицела)
    pub hide_crosshair_on_zoom: bool,

    /// Размер области прицела в пикселях (диаметр / ширина: 150 - 1600)
    pub scope_size: i32,

    /// Форма прицела: "circle" (круглый), "square" (квадрат), "wide" (широкий 16:9), "fullscreen" (весь экран)
    pub shape: String,

    /// Стиль перекрестия: "dot", "cross", "mildot", "chevron", "t_cross", "x_cross", "circle_dot", "box", "custom", "none"
    pub crosshair: String,

    /// Цвет перекрестия в HEX: "#00FF00", "#FF0000", "#00FFFF", "#FFFF00", "#FFFFFF", "#FF00FF"
    pub crosshair_color: String,

    /// Толщина линий перекрестия (1 - 15 пикселей)
    pub crosshair_size: i32,

    /// Зазор в центре перекрестия (-10 - 50 пикселей)
    pub crosshair_gap: i32,

    /// Длина линий перекрестия (2 - 100 пикселей)
    pub crosshair_length: i32,

    // --- Продвинутый редактор прицелов (слои) ---
    pub xhair_use_secondary_color: bool,
    pub xhair_secondary_color: String,
    pub xhair_outline: bool,
    pub xhair_outline_thickness: i32,
    pub xhair_outline_color: String,

    pub xhair_show_lines: bool,
    pub xhair_top_line: bool,
    pub xhair_bottom_line: bool,
    pub xhair_left_line: bool,
    pub xhair_right_line: bool,
    pub xhair_diagonal: bool,
    pub xhair_unlink_axis: bool,
    pub xhair_v_length: i32,

    pub xhair_center_dot: bool,
    pub xhair_dot_size: i32,
    pub xhair_dot_shape: String,

    pub xhair_show_circle: bool,
    pub xhair_circle_radius: i32,
    pub xhair_circle_thickness: i32,

    pub xhair_corner_brackets: bool,
    pub xhair_bracket_size: i32,
    pub xhair_bracket_dist: i32,

    pub xhair_ticks: bool,
    pub xhair_tick_count: i32,
    pub xhair_tick_step: i32,

    pub xhair_scale_with_zoom: bool,

    #[serde(default = "default_false")]
    pub xhair_layers_synced: bool,

    /// Стиль рамки линзы: "tactical" (темная двойная), "neon" (неоновая в цвет прицела), "thin" (тонкая), "none" (без рамки)
    pub border_style: String,

    /// Качество масштабирования: "smooth" (плавное/бикубическое) или "sharp" (четкие пиксели)
    pub scaling: String,

    /// Лимит FPS обновления оверлея (60, 120, 144, 240, 360)
    pub fps_limit: u32,

    /// Звуковой щелчок при включении/выключении (true / false)
    pub beep: bool,

    /// Следовать за курсором мыши вместо центра экрана
    pub follow_mouse: bool,

    /// Смещение прицела по горизонтали (X) в пикселях (-300..300)
    pub offset_x: i32,

    /// Смещение прицела по вертикали (Y) в пикселях (-300..300)
    pub offset_y: i32,

    /// Горячие клавиши быстрой настройки прямо в игре без открытия окна
    pub in_game_hotkeys: bool,

    /// Отображать тактический HUD кратности и всплывающие подсказки (OSD) прямо на линзе
    pub show_hud: bool,

    /// Сворачивать программу в системный трей при закрытии окна [X]
    pub close_to_tray: bool,

    /// Запускать программу сразу свернутой в системном трее
    pub start_minimized: bool,

    /// Язык интерфейса (Language::Ru / Language::En)
    #[serde(default)]
    pub language: Language,

    /// Пользовательские профили
    #[serde(default)]
    pub user_profiles: Vec<UserProfile>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            mode: "hold".to_string(),
            hotkey: "V".to_string(),
            crosshair_hotkey: "F7".to_string(),
            hk_zoom_out: "[".to_string(),
            hk_zoom_in: "]".to_string(),
            hk_size_dec: "-".to_string(),
            hk_size_inc: "=".to_string(),
            hk_toggle_no_zoom: "F8".to_string(),
            hk_cycle_reticle: "F9".to_string(),
            hk_cycle_shape: "F10".to_string(),
            hk_cycle_color: "F11".to_string(),
            crosshair_enabled: true,
            crosshair_mode: "toggle".to_string(),
            zoom_factor: 2.0,
            crosshair_only: false,
            always_show_crosshair: false,
            hide_crosshair_on_zoom: false,
            scope_size: 550,
            shape: "circle".to_string(),
            crosshair: "cross".to_string(),
            crosshair_color: "#00FF00".to_string(),
            crosshair_size: 2,
            crosshair_gap: 4,
            crosshair_length: 12,
            xhair_use_secondary_color: false,
            xhair_secondary_color: "#FF0055".to_string(),
            xhair_outline: true,
            xhair_outline_thickness: 1,
            xhair_outline_color: "#000000".to_string(),
            xhair_show_lines: true,
            xhair_top_line: true,
            xhair_bottom_line: true,
            xhair_left_line: true,
            xhair_right_line: true,
            xhair_diagonal: false,
            xhair_unlink_axis: false,
            xhair_v_length: 12,
            xhair_center_dot: true,
            xhair_dot_size: 2,
            xhair_dot_shape: "circle".to_string(),
            xhair_show_circle: false,
            xhair_circle_radius: 24,
            xhair_circle_thickness: 2,
            xhair_corner_brackets: false,
            xhair_bracket_size: 14,
            xhair_bracket_dist: 32,
            xhair_ticks: false,
            xhair_tick_count: 3,
            xhair_tick_step: 14,
            xhair_scale_with_zoom: false,
            xhair_layers_synced: true,
            border_style: "tactical".to_string(),
            scaling: "smooth".to_string(),
            fps_limit: 144,
            beep: false,
            follow_mouse: false,
            offset_x: 0,
            offset_y: 0,
            in_game_hotkeys: true,
            show_hud: true,
            close_to_tray: true,
            start_minimized: false,
            language: Language::default(),
            user_profiles: Vec::new(),
        }
    }
}

pub const DEFAULT_CONFIG_FILE: &str = "config.toml";

impl Config {
    /// Determines the best path for `config.toml` (supports running from shortcuts, Startup, or portable folder)
    pub fn resolve_config_path() -> PathBuf {
        // 1. Always prioritize config.toml next to the executable
        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(exe_dir) = exe_path.parent() {
                let exe_cfg = exe_dir.join(DEFAULT_CONFIG_FILE);
                if exe_cfg.exists() {
                    return exe_cfg;
                }
                // If directory is writable, return exe_cfg as primary
                return exe_cfg;
            }
        }

        // 2. Fallback to current working directory
        let cwd_path = Path::new(DEFAULT_CONFIG_FILE);
        cwd_path.to_path_buf()
    }

    pub fn load_or_create() -> Self {
        let path = Self::resolve_config_path();
        if !path.exists() {
            let default_cfg = Config::default();
            let _ = default_cfg.save();
            return default_cfg;
        }

        match fs::read_to_string(&path) {
            Ok(content) => match toml::from_str::<Config>(&content) {
                Ok(mut cfg) => {
                    cfg.sanitize();
                    cfg
                }
                Err(_) => Config::default(),
            },
            Err(_) => Config::default(),
        }
    }

    pub fn sanitize(&mut self) {
        self.zoom_factor = self.zoom_factor.clamp(1.0, 10.0);
        self.scope_size = self.scope_size.clamp(150, 1600);
        self.crosshair_size = self.crosshair_size.clamp(1, 15);
        self.crosshair_gap = self.crosshair_gap.clamp(-10, 50);
        self.crosshair_length = self.crosshair_length.clamp(2, 100);
        self.xhair_v_length = self.xhair_v_length.clamp(2, 100);
        self.xhair_dot_size = self.xhair_dot_size.clamp(1, 12);
        self.xhair_circle_radius = self.xhair_circle_radius.clamp(4, 120);
        self.xhair_circle_thickness = self.xhair_circle_thickness.clamp(1, 8);
        self.xhair_outline_thickness = self.xhair_outline_thickness.clamp(1, 4);
        self.xhair_bracket_size = self.xhair_bracket_size.clamp(6, 50);
        self.xhair_bracket_dist = self.xhair_bracket_dist.clamp(10, 120);
        self.xhair_tick_count = self.xhair_tick_count.clamp(1, 6);
        self.xhair_tick_step = self.xhair_tick_step.clamp(6, 35);
        self.offset_x = self.offset_x.clamp(-300, 300);
        self.offset_y = self.offset_y.clamp(-300, 300);
        self.fps_limit = self.fps_limit.clamp(30, 360);

        if !self.xhair_layers_synced {
            self.sync_layers_from_style();
            self.xhair_layers_synced = true;
        }

        // Automatically clear any quick hotkey that conflicts with primary Zoom or Crosshair hotkeys
        let quick_slots = [
            HotkeySlot::ZoomOut,
            HotkeySlot::ZoomIn,
            HotkeySlot::SizeDec,
            HotkeySlot::SizeInc,
            HotkeySlot::ToggleNoZoom,
            HotkeySlot::CycleReticle,
            HotkeySlot::CycleShape,
            HotkeySlot::CycleColor,
        ];
        for slot in quick_slots {
            let hk = self.get_hotkey(slot).to_string();
            if !hk.trim().is_empty()
                && (same_hotkey(&hk, &self.hotkey) || same_hotkey(&hk, &self.crosshair_hotkey))
            {
                *self.set_hotkey_mut(slot) = String::new();
            }
        }
    }

    pub fn get_hotkey(&self, slot: HotkeySlot) -> &str {
        match slot {
            HotkeySlot::Zoom => &self.hotkey,
            HotkeySlot::Crosshair => &self.crosshair_hotkey,
            HotkeySlot::ZoomOut => &self.hk_zoom_out,
            HotkeySlot::ZoomIn => &self.hk_zoom_in,
            HotkeySlot::SizeDec => &self.hk_size_dec,
            HotkeySlot::SizeInc => &self.hk_size_inc,
            HotkeySlot::ToggleNoZoom => &self.hk_toggle_no_zoom,
            HotkeySlot::CycleReticle => &self.hk_cycle_reticle,
            HotkeySlot::CycleShape => &self.hk_cycle_shape,
            HotkeySlot::CycleColor => &self.hk_cycle_color,
        }
    }

    pub fn set_hotkey_mut(&mut self, slot: HotkeySlot) -> &mut String {
        match slot {
            HotkeySlot::Zoom => &mut self.hotkey,
            HotkeySlot::Crosshair => &mut self.crosshair_hotkey,
            HotkeySlot::ZoomOut => &mut self.hk_zoom_out,
            HotkeySlot::ZoomIn => &mut self.hk_zoom_in,
            HotkeySlot::SizeDec => &mut self.hk_size_dec,
            HotkeySlot::SizeInc => &mut self.hk_size_inc,
            HotkeySlot::ToggleNoZoom => &mut self.hk_toggle_no_zoom,
            HotkeySlot::CycleReticle => &mut self.hk_cycle_reticle,
            HotkeySlot::CycleShape => &mut self.hk_cycle_shape,
            HotkeySlot::CycleColor => &mut self.hk_cycle_color,
        }
    }

    pub fn slot_label(slot: HotkeySlot, lang: Language) -> &'static str {
        match slot {
            HotkeySlot::Zoom => lang.t("Активация приближения", "Zoom activation"),
            HotkeySlot::Crosshair => lang.t("Постоянный прицел", "Always-on crosshair"),
            HotkeySlot::ZoomOut => lang.t("Уменьшить кратность (-0.25x)", "Decrease zoom (-0.25x)"),
            HotkeySlot::ZoomIn => lang.t("Увеличить кратность (+0.25x)", "Increase zoom (+0.25x)"),
            HotkeySlot::SizeDec => lang.t("Уменьшить область (-60 пкс)", "Decrease scope size (-60 px)"),
            HotkeySlot::SizeInc => lang.t("Увеличить область (+60 пкс)", "Increase scope size (+60 px)"),
            HotkeySlot::ToggleNoZoom => lang.t("Режим без приближения", "Crosshair-only mode (1.0x)"),
            HotkeySlot::CycleReticle => lang.t("Следующая форма сетки", "Next reticle style"),
            HotkeySlot::CycleShape => lang.t("Следующая форма области", "Next scope shape"),
            HotkeySlot::CycleColor => lang.t("Следующий цвет прицела", "Next crosshair color"),
        }
    }

    pub fn clear_hotkey(&mut self, slot: HotkeySlot) {
        *self.set_hotkey_mut(slot) = String::new();
    }

    pub fn clear_quick_hotkeys(&mut self) {
        self.hk_zoom_out.clear();
        self.hk_zoom_in.clear();
        self.hk_size_dec.clear();
        self.hk_size_inc.clear();
        self.hk_toggle_no_zoom.clear();
        self.hk_cycle_reticle.clear();
        self.hk_cycle_shape.clear();
        self.hk_cycle_color.clear();
    }

    pub fn reset_hotkeys_to_default(&mut self) {
        self.hotkey = "V".to_string();
        self.crosshair_hotkey = "F7".to_string();
        self.hk_zoom_out = "[".to_string();
        self.hk_zoom_in = "]".to_string();
        self.hk_size_dec = "-".to_string();
        self.hk_size_inc = "=".to_string();
        self.hk_toggle_no_zoom = "F8".to_string();
        self.hk_cycle_reticle = "F9".to_string();
        self.hk_cycle_shape = "F10".to_string();
        self.hk_cycle_color = "F11".to_string();
    }

    /// Assigns a new hotkey to `target` and frees that key from any other function that held it.
    pub fn assign_hotkey(&mut self, target: HotkeySlot, new_key: &str) -> Vec<&'static str> {
        let trimmed = new_key.trim().to_string();
        let lang = self.language;
        let mut freed = Vec::new();
        if !trimmed.is_empty() {
            for slot in HotkeySlot::ALL {
                if slot != target && same_hotkey(self.get_hotkey(slot), &trimmed) {
                    *self.set_hotkey_mut(slot) = String::new();
                    freed.push(Self::slot_label(slot, lang));
                }
            }
        }
        *self.set_hotkey_mut(target) = trimmed;
        freed
    }

    pub fn is_no_zoom_mode(&self) -> bool {
        self.crosshair_only || self.zoom_factor <= 1.02
    }

    pub fn operating_mode(&self) -> OperatingMode {
        if self.is_no_zoom_mode() {
            OperatingMode::CrosshairOnly
        } else if self.hide_crosshair_on_zoom {
            OperatingMode::ZoomOnly
        } else if self.always_show_crosshair {
            OperatingMode::AlwaysCrosshairPlusZoom
        } else {
            OperatingMode::ZoomWithCrosshair
        }
    }

    pub fn set_operating_mode(&mut self, mode: OperatingMode) {
        match mode {
            OperatingMode::ZoomWithCrosshair => {
                self.crosshair_only = false;
                self.always_show_crosshair = false;
                self.hide_crosshair_on_zoom = false;
                if self.zoom_factor <= 1.02 {
                    self.zoom_factor = 2.0;
                }
            }
            OperatingMode::AlwaysCrosshairPlusZoom => {
                self.crosshair_only = false;
                self.always_show_crosshair = true;
                self.crosshair_enabled = true;
                self.hide_crosshair_on_zoom = false;
                if self.zoom_factor <= 1.02 {
                    self.zoom_factor = 2.0;
                }
                if self.crosshair.eq_ignore_ascii_case("none") {
                    self.apply_crosshair_style("cross");
                }
            }
            OperatingMode::CrosshairOnly => {
                self.crosshair_only = true;
                self.always_show_crosshair = true;
                self.crosshair_enabled = true;
                self.hide_crosshair_on_zoom = false;
                if self.crosshair.eq_ignore_ascii_case("none") {
                    self.apply_crosshair_style("cross");
                }
            }
            OperatingMode::ZoomOnly => {
                self.crosshair_only = false;
                self.always_show_crosshair = false;
                self.hide_crosshair_on_zoom = true;
                if self.zoom_factor <= 1.02 {
                    self.zoom_factor = 2.0;
                }
            }
        }
    }

    pub fn operating_mode_short_label(&self) -> &'static str {
        match self.operating_mode() {
            OperatingMode::ZoomWithCrosshair => {
                self.language.t("Прицел при приближении", "Crosshair on zoom")
            }
            OperatingMode::AlwaysCrosshairPlusZoom => {
                self.language.t("Постоянный прицел и зум", "Always crosshair + zoom")
            }
            OperatingMode::CrosshairOnly => self.language.t("Только прицел", "Crosshair only"),
            OperatingMode::ZoomOnly => self.language.t("Чистый зум", "Zoom only"),
        }
    }

    pub fn active_profile_name(&self) -> Option<&'static str> {
        if self.crosshair_only
            && self.mode.eq_ignore_ascii_case("always")
            && self.crosshair_enabled
            && self.crosshair_size == 2
            && self.crosshair_gap == 4
            && self.crosshair_length == 12
        {
            Some(self.language.t("Только прицел", "Crosshair only"))
        } else if !self.crosshair_only
            && self.always_show_crosshair
            && self.mode.eq_ignore_ascii_case("hold")
            && (self.zoom_factor - 2.5).abs() < 0.05
            && self.scope_size == 600
            && self.shape.eq_ignore_ascii_case("circle")
            && self.border_style.eq_ignore_ascii_case("tactical")
        {
            Some(self.language.t("Постоянный прицел + зум", "Always crosshair + zoom"))
        } else if !self.crosshair_only
            && (self.zoom_factor - 1.8).abs() < 0.05
            && self.scope_size == 480
            && self.shape.eq_ignore_ascii_case("circle")
            && self.crosshair.eq_ignore_ascii_case("circle_dot")
        {
            Some(self.language.t("Штурмовик", "Assault"))
        } else if !self.crosshair_only
            && (self.zoom_factor - 4.0).abs() < 0.05
            && self.scope_size == 800
            && self.shape.eq_ignore_ascii_case("circle")
            && self.crosshair.eq_ignore_ascii_case("mildot")
        {
            Some(self.language.t("Снайпер", "Sniper"))
        } else if !self.crosshair_only
            && (self.zoom_factor - 2.5).abs() < 0.05
            && self.scope_size == 1050
            && self.shape.eq_ignore_ascii_case("wide")
            && self.crosshair.eq_ignore_ascii_case("chevron")
        {
            Some(self.language.t("Панорама 16:9", "Panorama 16:9"))
        } else if !self.crosshair_only
            && (self.zoom_factor - 5.0).abs() < 0.05
            && self.scope_size == 1350
            && self.shape.eq_ignore_ascii_case("square")
            && self.crosshair.eq_ignore_ascii_case("cross")
        {
            Some(self.language.t("Крупный план", "Close-up"))
        } else {
            None
        }
    }

    pub fn save(&self) -> Result<(), std::io::Error> {
        let serialized = toml::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

        let primary_path = Self::resolve_config_path();
        let cwd_path = Path::new(DEFAULT_CONFIG_FILE);
        if cwd_path.exists() && cwd_path != primary_path.as_path() {
            let _ = fs::write(cwd_path, &serialized);
        }

        if let Err(e) = fs::write(&primary_path, &serialized) {
            if let Ok(appdata) = std::env::var("APPDATA") {
                let fallback_dir = Path::new(&appdata).join("SniperZoom");
                let _ = fs::create_dir_all(&fallback_dir);
                return fs::write(fallback_dir.join(DEFAULT_CONFIG_FILE), serialized);
            }
            return Err(e);
        }
        Ok(())
    }

    pub fn parse_hex_rgb(hex: &str, default: (u8, u8, u8)) -> (u8, u8, u8) {
        let clean = hex.trim().trim_start_matches('#');
        if clean.len() == 6 {
            if let Ok(val) = u32::from_str_radix(clean, 16) {
                let r = ((val >> 16) & 0xFF) as u8;
                let g = ((val >> 8) & 0xFF) as u8;
                let b = (val & 0xFF) as u8;
                return (r, g, b);
            }
        }
        default
    }

    pub fn parse_color_rgb(&self) -> (u8, u8, u8) {
        Self::parse_hex_rgb(&self.crosshair_color, (0, 255, 0))
    }

    pub fn parse_secondary_color_rgb(&self) -> (u8, u8, u8) {
        if self.xhair_use_secondary_color {
            Self::parse_hex_rgb(&self.xhair_secondary_color, (255, 0, 85))
        } else {
            self.parse_color_rgb()
        }
    }

    pub fn parse_outline_color_rgb(&self) -> (u8, u8, u8) {
        Self::parse_hex_rgb(&self.xhair_outline_color, (0, 0, 0))
    }

    pub fn has_any_xhair_layer(&self) -> bool {
        self.xhair_center_dot
            || self.xhair_show_circle
            || self.xhair_corner_brackets
            || self.xhair_ticks
            || self.crosshair.eq_ignore_ascii_case("chevron")
            || self.crosshair.eq_ignore_ascii_case("mildot")
            || (self.xhair_show_lines
                && (self.xhair_top_line
                    || self.xhair_bottom_line
                    || self.xhair_left_line
                    || self.xhair_right_line
                    || self.xhair_diagonal))
    }

    pub fn sync_layers_from_style(&mut self) {
        match self.crosshair.to_lowercase().as_str() {
            "dot" => {
                self.xhair_show_lines = false;
                self.xhair_top_line = true;
                self.xhair_bottom_line = true;
                self.xhair_left_line = true;
                self.xhair_right_line = true;
                self.xhair_diagonal = false;
                self.xhair_center_dot = true;
                self.xhair_show_circle = false;
                self.xhair_corner_brackets = false;
                self.xhair_ticks = false;
            }
            "cross" => {
                self.xhair_show_lines = true;
                self.xhair_top_line = true;
                self.xhair_bottom_line = true;
                self.xhair_left_line = true;
                self.xhair_right_line = true;
                self.xhair_diagonal = false;
                self.xhair_center_dot = true;
                self.xhair_show_circle = false;
                self.xhair_corner_brackets = false;
                self.xhair_ticks = false;
            }
            "t_cross" => {
                self.xhair_show_lines = true;
                self.xhair_top_line = false;
                self.xhair_bottom_line = true;
                self.xhair_left_line = true;
                self.xhair_right_line = true;
                self.xhair_diagonal = false;
                self.xhair_center_dot = true;
                self.xhair_show_circle = false;
                self.xhair_corner_brackets = false;
                self.xhair_ticks = false;
            }
            "x_cross" => {
                self.xhair_show_lines = true;
                self.xhair_top_line = false;
                self.xhair_bottom_line = false;
                self.xhair_left_line = false;
                self.xhair_right_line = false;
                self.xhair_diagonal = true;
                self.xhair_center_dot = true;
                self.xhair_show_circle = false;
                self.xhair_corner_brackets = false;
                self.xhair_ticks = false;
            }
            "circle_dot" => {
                self.xhair_show_lines = false;
                self.xhair_top_line = true;
                self.xhair_bottom_line = true;
                self.xhair_left_line = true;
                self.xhair_right_line = true;
                self.xhair_diagonal = false;
                self.xhair_center_dot = true;
                self.xhair_show_circle = true;
                self.xhair_corner_brackets = false;
                self.xhair_ticks = false;
            }
            "box" => {
                self.xhair_show_lines = false;
                self.xhair_top_line = true;
                self.xhair_bottom_line = true;
                self.xhair_left_line = true;
                self.xhair_right_line = true;
                self.xhair_diagonal = false;
                self.xhair_center_dot = true;
                self.xhair_show_circle = false;
                self.xhair_corner_brackets = true;
                self.xhair_ticks = false;
            }
            "mildot" => {
                self.xhair_show_lines = true;
                self.xhair_top_line = true;
                self.xhair_bottom_line = true;
                self.xhair_left_line = true;
                self.xhair_right_line = true;
                self.xhair_diagonal = false;
                self.xhair_center_dot = true;
                self.xhair_show_circle = false;
                self.xhair_corner_brackets = false;
                self.xhair_ticks = false;
            }
            "chevron" => {
                self.xhair_show_lines = false;
                self.xhair_top_line = true;
                self.xhair_bottom_line = true;
                self.xhair_left_line = true;
                self.xhair_right_line = true;
                self.xhair_diagonal = false;
                self.xhair_center_dot = false;
                self.xhair_show_circle = false;
                self.xhair_corner_brackets = false;
                self.xhair_ticks = false;
            }
            "none" => {
                self.xhair_show_lines = false;
                self.xhair_diagonal = false;
                self.xhair_center_dot = false;
                self.xhair_show_circle = false;
                self.xhair_corner_brackets = false;
                self.xhair_ticks = false;
            }
            _ => {}
        }
        self.xhair_layers_synced = true;
    }

    pub fn apply_crosshair_style(&mut self, style: &str) {
        self.crosshair = style.to_string();
        self.sync_layers_from_style();
    }

    /// Applies one of the built-in Pro Crosshair Presets in the Advanced Crosshair Editor
    pub fn apply_crosshair_preset(&mut self, preset: &str) {
        self.xhair_outline = true;
        self.xhair_outline_thickness = 1;
        self.xhair_outline_color = "#000000".to_string();
        self.xhair_unlink_axis = false;
        self.xhair_scale_with_zoom = false;

        match preset {
            "cs2_pro" => {
                self.apply_crosshair_style("cross");
                self.crosshair_color = "#00FF00".to_string();
                self.xhair_use_secondary_color = false;
                self.crosshair_size = 2;
                self.crosshair_gap = 3;
                self.crosshair_length = 8;
                self.xhair_center_dot = false;
            }
            "valorant_cyan" => {
                self.apply_crosshair_style("cross");
                self.crosshair_color = "#00FFFF".to_string();
                self.xhair_use_secondary_color = false;
                self.crosshair_size = 2;
                self.crosshair_gap = 4;
                self.crosshair_length = 7;
                self.xhair_center_dot = true;
                self.xhair_dot_size = 1;
                self.xhair_dot_shape = "square".to_string();
            }
            "headshot_t" => {
                self.apply_crosshair_style("t_cross");
                self.crosshair_color = "#FFFF00".to_string();
                self.xhair_use_secondary_color = true;
                self.xhair_secondary_color = "#FF0055".to_string();
                self.crosshair_size = 2;
                self.crosshair_gap = 4;
                self.crosshair_length = 11;
                self.xhair_center_dot = true;
                self.xhair_dot_size = 2;
                self.xhair_dot_shape = "circle".to_string();
            }
            "eotech_holo" => {
                self.apply_crosshair_style("circle_dot");
                self.crosshair_color = "#FF2A2A".to_string();
                self.xhair_use_secondary_color = false;
                self.crosshair_size = 2;
                self.crosshair_length = 24;
                self.xhair_circle_radius = 24;
                self.xhair_circle_thickness = 2;
                self.xhair_center_dot = true;
                self.xhair_dot_size = 2;
                self.xhair_dot_shape = "circle".to_string();
            }
            "sniper_mildot" => {
                self.apply_crosshair_style("mildot");
                self.crosshair_color = "#00FFFF".to_string();
                self.xhair_use_secondary_color = false;
                self.crosshair_size = 2;
                self.crosshair_gap = 5;
                self.xhair_center_dot = true;
                self.xhair_dot_size = 2;
                self.xhair_dot_shape = "circle".to_string();
            }
            "pso_chevron" => {
                self.apply_crosshair_style("chevron");
                self.crosshair_color = "#FF0000".to_string();
                self.xhair_use_secondary_color = false;
                self.crosshair_size = 2;
                self.crosshair_length = 14;
            }
            "predator_box" => {
                self.apply_crosshair_style("box");
                self.crosshair_color = "#00FF66".to_string();
                self.xhair_use_secondary_color = true;
                self.xhair_secondary_color = "#FF0044".to_string();
                self.crosshair_size = 2;
                self.xhair_bracket_size = 14;
                self.xhair_bracket_dist = 28;
                self.xhair_center_dot = true;
                self.xhair_dot_size = 2;
                self.xhair_dot_shape = "square".to_string();
            }
            "hitmarker_x" => {
                self.apply_crosshair_style("x_cross");
                self.crosshair_color = "#FFFFFF".to_string();
                self.xhair_use_secondary_color = true;
                self.xhair_secondary_color = "#FF0055".to_string();
                self.crosshair_size = 2;
                self.crosshair_gap = 5;
                self.crosshair_length = 11;
                self.xhair_center_dot = true;
                self.xhair_dot_size = 2;
                self.xhair_dot_shape = "circle".to_string();
            }
            "minimal_dot" => {
                self.apply_crosshair_style("dot");
                self.crosshair_color = "#00FF00".to_string();
                self.xhair_use_secondary_color = false;
                self.crosshair_size = 2;
                self.xhair_center_dot = true;
                self.xhair_dot_size = 2;
                self.xhair_dot_shape = "circle".to_string();
            }
            _ => {}
        }
    }

    pub fn export_crosshair_code(&self) -> String {
        let b = |v: bool| if v { "1" } else { "0" };
        format!(
            "SZ2;{};{};{};{};{};{};{};{};{};{};{};{};{};{};{};{};{};{};{};{};{};{};{};{};{};{};{};{};{};{};{}",
            self.crosshair,
            self.crosshair_color,
            b(self.xhair_use_secondary_color),
            self.xhair_secondary_color,
            b(self.xhair_outline),
            self.xhair_outline_thickness,
            self.xhair_outline_color,
            b(self.xhair_show_lines),
            b(self.xhair_top_line),
            b(self.xhair_bottom_line),
            b(self.xhair_left_line),
            b(self.xhair_right_line),
            b(self.xhair_diagonal),
            self.crosshair_size,
            self.crosshair_gap,
            self.crosshair_length,
            b(self.xhair_unlink_axis),
            self.xhair_v_length,
            b(self.xhair_center_dot),
            self.xhair_dot_size,
            self.xhair_dot_shape,
            b(self.xhair_show_circle),
            self.xhair_circle_radius,
            self.xhair_circle_thickness,
            b(self.xhair_corner_brackets),
            self.xhair_bracket_size,
            self.xhair_bracket_dist,
            b(self.xhair_ticks),
            self.xhair_tick_count,
            self.xhair_tick_step,
            b(self.xhair_scale_with_zoom),
        )
    }

    pub fn import_crosshair_code(&mut self, code: &str) -> bool {
        let parts: Vec<&str> = code.trim().split(';').collect();
        if parts.len() != 32 || parts[0] != "SZ2" {
            return false;
        }
        let parse_b = |s: &str| match s {
            "1" | "true" => Some(true),
            "0" | "false" => Some(false),
            _ => None,
        };

        let Some(use_sec) = parse_b(parts[3]) else { return false };
        let Some(outline) = parse_b(parts[5]) else { return false };
        let Ok(out_th) = parts[6].parse::<i32>() else { return false };
        let Some(show_lines) = parse_b(parts[8]) else { return false };
        let Some(top_line) = parse_b(parts[9]) else { return false };
        let Some(bottom_line) = parse_b(parts[10]) else { return false };
        let Some(left_line) = parse_b(parts[11]) else { return false };
        let Some(right_line) = parse_b(parts[12]) else { return false };
        let Some(diagonal) = parse_b(parts[13]) else { return false };
        let Ok(size) = parts[14].parse::<i32>() else { return false };
        let Ok(gap) = parts[15].parse::<i32>() else { return false };
        let Ok(len) = parts[16].parse::<i32>() else { return false };
        let Some(unlink) = parse_b(parts[17]) else { return false };
        let Ok(v_len) = parts[18].parse::<i32>() else { return false };
        let Some(dot) = parse_b(parts[19]) else { return false };
        let Ok(dot_sz) = parts[20].parse::<i32>() else { return false };
        let Some(show_circle) = parse_b(parts[22]) else { return false };
        let Ok(c_rad) = parts[23].parse::<i32>() else { return false };
        let Ok(c_th) = parts[24].parse::<i32>() else { return false };
        let Some(brackets) = parse_b(parts[25]) else { return false };
        let Ok(b_sz) = parts[26].parse::<i32>() else { return false };
        let Ok(b_dist) = parts[27].parse::<i32>() else { return false };
        let Some(ticks) = parse_b(parts[28]) else { return false };
        let Ok(t_cnt) = parts[29].parse::<i32>() else { return false };
        let Ok(t_step) = parts[30].parse::<i32>() else { return false };
        let Some(scale_zoom) = parse_b(parts[31]) else { return false };

        self.crosshair = parts[1].to_string();
        self.crosshair_color = parts[2].to_string();
        self.xhair_use_secondary_color = use_sec;
        self.xhair_secondary_color = parts[4].to_string();
        self.xhair_outline = outline;
        self.xhair_outline_thickness = out_th;
        self.xhair_outline_color = parts[7].to_string();
        self.xhair_show_lines = show_lines;
        self.xhair_top_line = top_line;
        self.xhair_bottom_line = bottom_line;
        self.xhair_left_line = left_line;
        self.xhair_right_line = right_line;
        self.xhair_diagonal = diagonal;
        self.crosshair_size = size;
        self.crosshair_gap = gap;
        self.crosshair_length = len;
        self.xhair_unlink_axis = unlink;
        self.xhair_v_length = v_len;
        self.xhair_center_dot = dot;
        self.xhair_dot_size = dot_sz;
        self.xhair_dot_shape = if parts[21] == "square" {
            "square".to_string()
        } else {
            "circle".to_string()
        };
        self.xhair_show_circle = show_circle;
        self.xhair_circle_radius = c_rad;
        self.xhair_circle_thickness = c_th;
        self.xhair_corner_brackets = brackets;
        self.xhair_bracket_size = b_sz;
        self.xhair_bracket_dist = b_dist;
        self.xhair_ticks = ticks;
        self.xhair_tick_count = t_cnt;
        self.xhair_tick_step = t_step;
        self.xhair_scale_with_zoom = scale_zoom;
        self.xhair_layers_synced = true;
        self.sanitize();
        true
    }

    pub fn save_user_profile(&mut self, name: &str) -> String {
        let trimmed = name.trim();
        let final_name = if trimmed.is_empty() {
            format!(
                "{} {}",
                self.language.t("Профиль", "Profile"),
                self.user_profiles.len() + 1
            )
        } else {
            trimmed.to_string()
        };

        let profile = UserProfile {
            name: final_name.clone(),
            crosshair_only: self.crosshair_only,
            always_show_crosshair: self.always_show_crosshair,
            hide_crosshair_on_zoom: self.hide_crosshair_on_zoom,
            zoom_factor: self.zoom_factor,
            scope_size: self.scope_size,
            shape: self.shape.clone(),
            border_style: self.border_style.clone(),
            crosshair_code: self.export_crosshair_code(),
        };

        if let Some(existing) = self
            .user_profiles
            .iter_mut()
            .find(|p| p.name.eq_ignore_ascii_case(&final_name))
        {
            *existing = profile;
        } else {
            self.user_profiles.push(profile);
        }

        final_name
    }

    pub fn apply_user_profile(&mut self, index: usize) -> Option<String> {
        let profile = self.user_profiles.get(index)?.clone();
        self.crosshair_only = profile.crosshair_only;
        self.always_show_crosshair = profile.always_show_crosshair;
        self.hide_crosshair_on_zoom = profile.hide_crosshair_on_zoom;
        self.zoom_factor = profile.zoom_factor;
        self.scope_size = profile.scope_size;
        self.shape = profile.shape;
        self.border_style = profile.border_style;
        self.import_crosshair_code(&profile.crosshair_code);
        Some(profile.name)
    }

    pub fn delete_user_profile(&mut self, index: usize) -> Option<String> {
        if index < self.user_profiles.len() {
            Some(self.user_profiles.remove(index).name)
        } else {
            None
        }
    }

    /// Applies one of the built-in tactical presets
    pub fn apply_profile(&mut self, profile: &str) {
        match profile {
            "crosshair_only" => {
                // Постоянный прицел без приближения (1.0x)
                self.crosshair_only = true;
                self.crosshair_enabled = true;
                self.mode = "always".to_string();
                if self.crosshair == "none" {
                    self.apply_crosshair_style("cross");
                } else {
                    self.sync_layers_from_style();
                }
                self.crosshair_color = "#00FF00".to_string();
                self.crosshair_size = 2;
                self.crosshair_gap = 4;
                self.crosshair_length = 12;
            }
            "hybrid" => {
                // Прицел всегда на экране + Зум по кнопке
                self.crosshair_only = false;
                self.always_show_crosshair = true;
                self.crosshair_enabled = true;
                self.mode = "hold".to_string();
                self.zoom_factor = 2.5;
                self.scope_size = 600;
                self.shape = "circle".to_string();
                if self.crosshair == "none" {
                    self.apply_crosshair_style("dot");
                } else {
                    self.sync_layers_from_style();
                }
                self.crosshair_color = "#00FF00".to_string();
                self.border_style = "tactical".to_string();
            }
            "assault" => {
                // Быстрый коллиматор штурмовика
                self.crosshair_only = false;
                self.zoom_factor = 1.8;
                self.scope_size = 480;
                self.shape = "circle".to_string();
                self.apply_crosshair_style("circle_dot");
                self.crosshair_color = "#00FF00".to_string();
                self.crosshair_size = 2;
                self.xhair_circle_radius = 22;
                self.border_style = "tactical".to_string();
            }
            "sniper" => {
                // Крупная снайперская оптика Mil-Dot
                self.crosshair_only = false;
                self.zoom_factor = 4.0;
                self.scope_size = 800;
                self.shape = "circle".to_string();
                self.apply_crosshair_style("mildot");
                self.crosshair_color = "#00FFFF".to_string();
                self.crosshair_size = 2;
                self.border_style = "tactical".to_string();
            }
            "panorama" => {
                // Широкоформатная панорама 16:9
                self.crosshair_only = false;
                self.zoom_factor = 2.5;
                self.scope_size = 1050;
                self.shape = "wide".to_string();
                self.apply_crosshair_style("chevron");
                self.crosshair_color = "#FF0000".to_string();
                self.crosshair_size = 2;
                self.border_style = "neon".to_string();
            }
            "mega" => {
                // Огромная зона Scope 1350px
                self.crosshair_only = false;
                self.zoom_factor = 5.0;
                self.scope_size = 1350;
                self.shape = "square".to_string();
                self.apply_crosshair_style("cross");
                self.crosshair_color = "#00FF00".to_string();
                self.crosshair_size = 2;
                self.crosshair_gap = 6;
                self.crosshair_length = 22;
                self.border_style = "thin".to_string();
            }
            _ => {}
        }
    }
}

// ============================================================================
// Windows Registry Autostart Helpers (HKCU\Software\Microsoft\Windows\CurrentVersion\Run)
// ============================================================================

const HKEY_CURRENT_USER: isize = -2147483647i32 as isize; // 0x80000001
const KEY_READ: u32 = 0x20019;
const KEY_WRITE: u32 = 0x20006;
const REG_SZ: u32 = 1;

#[link(name = "advapi32")]
unsafe extern "system" {
    fn RegOpenKeyExW(
        hKey: isize,
        lpSubKey: *const u16,
        ulOptions: u32,
        samDesired: u32,
        phkResult: *mut isize,
    ) -> i32;
    fn RegQueryValueExW(
        hKey: isize,
        lpValueName: *const u16,
        lpReserved: *const u32,
        lpType: *mut u32,
        lpData: *mut u8,
        lpcbData: *mut u32,
    ) -> i32;
    fn RegSetValueExW(
        hKey: isize,
        lpValueName: *const u16,
        Reserved: u32,
        dwType: u32,
        lpData: *const u8,
        cbData: u32,
    ) -> i32;
    fn RegDeleteValueW(hKey: isize, lpValueName: *const u16) -> i32;
    fn RegCloseKey(hKey: isize) -> i32;
}

fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

pub fn is_autostart_enabled() -> bool {
    unsafe {
        let subkey = to_wide("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
        let val_name = to_wide("SniperZoom");
        let mut hkey: isize = 0;
        if RegOpenKeyExW(HKEY_CURRENT_USER, subkey.as_ptr(), 0, KEY_READ, &mut hkey) == 0 {
            let mut data_len: u32 = 0;
            let res = RegQueryValueExW(
                hkey,
                val_name.as_ptr(),
                null_mut(),
                null_mut(),
                null_mut(),
                &mut data_len,
            );
            RegCloseKey(hkey);
            return res == 0 && data_len > 0;
        }
        false
    }
}

pub fn set_autostart(enable: bool) -> bool {
    unsafe {
        let subkey = to_wide("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
        let val_name = to_wide("SniperZoom");
        let mut hkey: isize = 0;
        if RegOpenKeyExW(HKEY_CURRENT_USER, subkey.as_ptr(), 0, KEY_WRITE, &mut hkey) != 0 {
            return false;
        }

        let ok = if enable {
            if let Ok(exe) = std::env::current_exe() {
                let cmd = format!("\"{}\" --minimized", exe.display());
                let cmd_wide = to_wide(&cmd);
                let byte_len = (cmd_wide.len() * 2) as u32;
                RegSetValueExW(
                    hkey,
                    val_name.as_ptr(),
                    0,
                    REG_SZ,
                    cmd_wide.as_ptr() as *const u8,
                    byte_len,
                ) == 0
            } else {
                false
            }
        } else {
            RegDeleteValueW(hkey, val_name.as_ptr()) == 0
        };

        RegCloseKey(hkey);
        ok
    }
}
