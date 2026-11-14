use crate::config::{
    is_autostart_enabled, set_autostart, Config, HotkeySlot, Language, OperatingMode,
};
use crate::keys::{format_hotkey_display_lang, vk_to_name};
use crate::tray::hide_main_window_to_tray;
use eframe::egui;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::time::Instant;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;

// =============================================================================
// DESIGN TOKENS (Flat neutral dark palette, single blue accent, >= 4.5:1 contrast)
// =============================================================================
const BG_APP: egui::Color32 = egui::Color32::from_rgb(19, 21, 25); // #131519
const BG_PANEL: egui::Color32 = egui::Color32::from_rgb(22, 25, 30); // #16191E
const BG_CARD: egui::Color32 = egui::Color32::from_rgb(27, 30, 36); // #1B1E24
const BG_CONTROL: egui::Color32 = egui::Color32::from_rgb(35, 39, 47); // #23272F
const BG_CONTROL_HOVER: egui::Color32 = egui::Color32::from_rgb(46, 52, 63); // #2E343F
const BORDER_SUBTLE: egui::Color32 = egui::Color32::from_rgb(46, 52, 64); // #2E3440
const BORDER_CONTROL: egui::Color32 = egui::Color32::from_rgb(68, 76, 92); // #444C5C

const ACCENT: egui::Color32 = egui::Color32::from_rgb(33, 94, 224); // #215EE0 (~5.5:1 with white)
const ACCENT_HOVER: egui::Color32 = egui::Color32::from_rgb(49, 110, 238);
const ACCENT_BRIGHT: egui::Color32 = egui::Color32::from_rgb(76, 138, 255);
const ACCENT_TINT: egui::Color32 = egui::Color32::from_rgb(29, 44, 74); // #1D2C4A

const TEXT_PRIMARY: egui::Color32 = egui::Color32::from_rgb(243, 244, 246); // #F3F4F6
const TEXT_SECONDARY: egui::Color32 = egui::Color32::from_rgb(166, 176, 192); // #A6B0C0

const DANGER_BG: egui::Color32 = egui::Color32::from_rgb(56, 24, 28);
const DANGER_BG_HOVER: egui::Color32 = egui::Color32::from_rgb(76, 31, 36);
const DANGER_BORDER: egui::Color32 = egui::Color32::from_rgb(185, 68, 68);
const DANGER_TEXT: egui::Color32 = egui::Color32::from_rgb(248, 148, 148);

const STATUS_ACTIVE_DOT: egui::Color32 = egui::Color32::from_rgb(52, 211, 153);
const STATUS_PAUSED_DOT: egui::Color32 = egui::Color32::from_rgb(245, 158, 11);

const TOAST_BG: egui::Color32 = egui::Color32::from_rgb(22, 42, 34);
const TOAST_BORDER: egui::Color32 = egui::Color32::from_rgb(46, 150, 102);
const TOAST_TEXT: egui::Color32 = egui::Color32::from_rgb(134, 239, 188);

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum GuiTab {
    CrosshairEditor,
    OpticsAndZoom,
    Hotkeys,
    Profiles,
    System,
}

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum PreviewBg {
    Night,
    City,
    Desert,
    Forest,
    Snow,
}

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum UiIcon {
    Crosshair,
    Zoom,
    Keyboard,
    Profiles,
    System,
    Play,
    Pause,
    Eye,
    Tray,
    Copy,
    Import,
    Reset,
    Close,
}

#[allow(dead_code)]
pub struct SniperZoomApp {
    pub config: Arc<RwLock<Config>>,
    pub is_active_indicator: Arc<AtomicBool>,
    pub is_paused: Arc<AtomicBool>,
    pub force_test_active: Arc<AtomicBool>,
    pub window_visible: Arc<AtomicBool>,
    pub show_balloon_flag: Arc<AtomicBool>,
    pub should_exit: Arc<AtomicBool>,
    pub osd_message: Arc<RwLock<Option<(String, Instant)>>>,
    pub listening_slot: Option<HotkeySlot>,
    pub listening_since: Option<Instant>,
    pub status_msg: Option<(String, Instant)>,
    pub temp_hex: String,
    pub temp_sec_hex: String,
    pub temp_out_hex: String,
    pub share_code_buf: String,
    pub new_profile_name: String,
    pub active_tab: GuiTab,
    pub preview_bg: PreviewBg,
    pub preview_zoom: f32,
    pub preview_lens_mode: bool,
    pub autostart_cached: bool,
    pub start_hidden_once: bool,
    pub frames_rendered: u32,
}

impl SniperZoomApp {
    pub fn new(
        config: Arc<RwLock<Config>>,
        is_active_indicator: Arc<AtomicBool>,
        is_paused: Arc<AtomicBool>,
        force_test_active: Arc<AtomicBool>,
        window_visible: Arc<AtomicBool>,
        show_balloon_flag: Arc<AtomicBool>,
        should_exit: Arc<AtomicBool>,
        osd_message: Arc<RwLock<Option<(String, Instant)>>>,
        start_hidden_once: bool,
    ) -> Self {
        let cfg_guard = config.read().unwrap();
        let initial_hex = cfg_guard.crosshair_color.clone();
        let initial_sec_hex = cfg_guard.xhair_secondary_color.clone();
        let initial_out_hex = cfg_guard.xhair_outline_color.clone();
        let initial_code = cfg_guard.export_crosshair_code();
        drop(cfg_guard);

        Self {
            config,
            is_active_indicator,
            is_paused,
            force_test_active,
            window_visible,
            show_balloon_flag,
            should_exit,
            osd_message,
            listening_slot: None,
            listening_since: None,
            status_msg: None,
            temp_hex: initial_hex,
            temp_sec_hex: initial_sec_hex,
            temp_out_hex: initial_out_hex,
            share_code_buf: initial_code,
            new_profile_name: String::new(),
            active_tab: GuiTab::CrosshairEditor,
            preview_bg: PreviewBg::City,
            preview_zoom: 2.0,
            preview_lens_mode: false,
            autostart_cached: is_autostart_enabled(),
            start_hidden_once,
            frames_rendered: 0,
        }
    }

    fn set_status(&mut self, msg: impl Into<String>) {
        self.status_msg = Some((msg.into(), Instant::now()));
    }

    fn sync_hex_buffers(&mut self, cfg: &Config) {
        self.temp_hex = cfg.crosshair_color.clone();
        self.temp_sec_hex = cfg.xhair_secondary_color.clone();
        self.temp_out_hex = cfg.xhair_outline_color.clone();
        self.share_code_buf = cfg.export_crosshair_code();
    }
}

fn configure_theme(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.dark_mode = true;
    visuals.panel_fill = BG_APP;
    visuals.window_fill = BG_CARD;
    visuals.extreme_bg_color = BG_PANEL;
    visuals.faint_bg_color = BG_CONTROL;
    visuals.weak_text_color = Some(TEXT_SECONDARY);
    visuals.slider_trailing_fill = true;
    visuals.disabled_alpha = 0.72;

    visuals.widgets.noninteractive.bg_fill = BG_CARD;
    visuals.widgets.noninteractive.weak_bg_fill = BG_CARD;
    visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, BORDER_SUBTLE);
    visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, TEXT_PRIMARY);
    visuals.widgets.noninteractive.corner_radius = egui::CornerRadius::same(5);

    visuals.widgets.inactive.bg_fill = BG_CONTROL;
    visuals.widgets.inactive.weak_bg_fill = BG_CONTROL;
    visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, BORDER_CONTROL);
    visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, TEXT_PRIMARY);
    visuals.widgets.inactive.corner_radius = egui::CornerRadius::same(5);

    visuals.widgets.hovered.bg_fill = BG_CONTROL_HOVER;
    visuals.widgets.hovered.weak_bg_fill = BG_CONTROL_HOVER;
    visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, ACCENT_BRIGHT);
    visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.5, egui::Color32::WHITE);
    visuals.widgets.hovered.corner_radius = egui::CornerRadius::same(5);

    visuals.widgets.active.bg_fill = ACCENT;
    visuals.widgets.active.weak_bg_fill = ACCENT;
    visuals.widgets.active.bg_stroke = egui::Stroke::new(1.0, ACCENT_BRIGHT);
    visuals.widgets.active.fg_stroke = egui::Stroke::new(1.5, egui::Color32::WHITE);
    visuals.widgets.active.corner_radius = egui::CornerRadius::same(5);

    visuals.selection.bg_fill = ACCENT;
    visuals.selection.stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);

    ctx.set_visuals(visuals);
}

impl eframe::App for SniperZoomApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        configure_theme(&ctx);

        // Hide on startup if launched with --minimized or start_minimized is enabled
        self.frames_rendered = self.frames_rendered.saturating_add(1);
        if self.start_hidden_once && self.frames_rendered == 2 {
            self.start_hidden_once = false;
            hide_main_window_to_tray(&self.window_visible, &self.show_balloon_flag);
        }

        let mut cfg = self.config.read().unwrap().clone();
        if !ctx.egui_wants_keyboard_input() {
            if self.temp_hex != cfg.crosshair_color {
                self.temp_hex = cfg.crosshair_color.clone();
            }
            if self.temp_sec_hex != cfg.xhair_secondary_color {
                self.temp_sec_hex = cfg.xhair_secondary_color.clone();
            }
            if self.temp_out_hex != cfg.xhair_outline_color {
                self.temp_out_hex = cfg.xhair_outline_color.clone();
            }
        }

        // Intercept window [X] close button -> hide to System Tray instead of quitting
        if ctx.input(|i| i.viewport().close_requested()) {
            if !self.should_exit.load(Ordering::Relaxed) && cfg.close_to_tray {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                hide_main_window_to_tray(&self.window_visible, &self.show_balloon_flag);
            } else {
                self.should_exit.store(true, Ordering::Relaxed);
            }
        }

        ctx.request_repaint_after(std::time::Duration::from_millis(120));

        let mut changed = false;
        let lang = cfg.language;

        // 1. Hotkey Listening Mode for any HotkeySlot
        if let Some(slot) = self.listening_slot {
            ctx.request_repaint();

            let debounce_passed = self
                .listening_since
                .map(|since| since.elapsed() >= std::time::Duration::from_millis(150))
                .unwrap_or(true);

            if debounce_passed {
                unsafe {
                    if (GetAsyncKeyState(VK_ESCAPE as i32) as u16 & 0x8000) != 0 {
                        self.listening_slot = None;
                        self.listening_since = None;
                        self.set_status(lang.t("Назначение клавиши отменено", "Key binding cancelled"));
                    } else if (GetAsyncKeyState(VK_BACK as i32) as u16 & 0x8000) != 0
                        || (GetAsyncKeyState(VK_DELETE as i32) as u16 & 0x8000) != 0
                    {
                        cfg.clear_hotkey(slot);
                        self.listening_slot = None;
                        self.listening_since = None;
                        changed = true;
                        self.set_status(format!(
                            "{} «{}»",
                            lang.t("Клавиша отвязана:", "Key unbound:"),
                            Config::slot_label(slot, lang)
                        ));
                    } else {
                        for vk in 1..255 {
                            if vk == VK_SHIFT as i32
                                || vk == VK_CONTROL as i32
                                || vk == VK_MENU as i32
                            {
                                continue;
                            }
                            if vk == VK_LBUTTON as i32 {
                                if let Some(since) = self.listening_since {
                                    if since.elapsed() < std::time::Duration::from_millis(350) {
                                        continue;
                                    }
                                }
                            }

                            if (GetAsyncKeyState(vk) as u16 & 0x8000) != 0 {
                                let name = vk_to_name(vk);
                                let freed = cfg.assign_hotkey(slot, &name);
                                let pretty = format_hotkey_display_lang(&name, lang);

                                self.listening_slot = None;
                                self.listening_since = None;
                                changed = true;

                                if freed.is_empty() {
                                    self.set_status(format!(
                                        "{} [{}] -> «{}»",
                                        lang.t("Назначено", "Bound"),
                                        pretty,
                                        Config::slot_label(slot, lang)
                                    ));
                                } else {
                                    self.set_status(format!(
                                        "{} [{}] -> «{}» ({}: {})",
                                        lang.t("Назначено", "Bound"),
                                        pretty,
                                        Config::slot_label(slot, lang),
                                        lang.t("отвязано от", "unbound from"),
                                        freed.join(", ")
                                    ));
                                }
                                break;
                            }
                        }
                    }
                }
            }
        }

        // --- TOP BAR: NAVIGATION TABS + STATUS + CONSISTENT HEADER ACTIONS ---
        egui::Panel::top("top_header")
            .frame(
                egui::Frame::new()
                    .fill(BG_PANEL)
                    .stroke(egui::Stroke::new(1.0, BORDER_SUBTLE))
                    .inner_margin(egui::Margin::symmetric(12, 8)),
            )
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    // Left: Navigation tabs (styled distinctly as flat tabs with bottom indicator)
                    let tabs = [
                        (GuiTab::CrosshairEditor, UiIcon::Crosshair, lang.t("Прицел", "Crosshair")),
                        (GuiTab::OpticsAndZoom, UiIcon::Zoom, lang.t("Увеличение", "Zoom")),
                        (GuiTab::Hotkeys, UiIcon::Keyboard, lang.t("Управление", "Controls")),
                        (GuiTab::Profiles, UiIcon::Profiles, lang.t("Профили", "Profiles")),
                        (GuiTab::System, UiIcon::System, lang.t("Система", "System")),
                    ];
                    for (tab, icon, label) in tabs {
                        let active = self.active_tab == tab;
                        if nav_tab_button(ui, active, icon, label).clicked() {
                            self.active_tab = tab;
                            if tab == GuiTab::OpticsAndZoom {
                                self.preview_lens_mode = true;
                            } else if tab == GuiTab::CrosshairEditor {
                                self.preview_lens_mode = false;
                            }
                        }
                    }

                    // Right: Status indicator (non-interactive) + uniform action buttons
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if action_button(ui, lang.t("В трей", "To tray"), Some(UiIcon::Tray))
                            .on_hover_text(lang.t("Скрыть окно в системный трей", "Hide window to system tray"))
                            .clicked()
                        {
                            hide_main_window_to_tray(&self.window_visible, &self.show_balloon_flag);
                        }

                        let is_forced = self.force_test_active.load(Ordering::Relaxed);
                        let test_resp = if is_forced {
                            primary_button_with_icon(ui, lang.t("Стоп", "Stop"), Some(UiIcon::Eye))
                        } else {
                            action_button(ui, lang.t("Проверка", "Preview"), Some(UiIcon::Eye))
                        };
                        if test_resp
                            .on_hover_text(lang.t("Показать оверлей поверх экрана для проверки", "Show overlay on screen for testing"))
                            .clicked()
                        {
                            self.force_test_active.store(!is_forced, Ordering::Relaxed);
                        }

                        let paused = self.is_paused.load(Ordering::Relaxed);
                        let pause_resp = if paused {
                            primary_button_with_icon(ui, lang.t("Продолжить", "Resume"), Some(UiIcon::Play))
                        } else {
                            action_button(ui, lang.t("Приостановить", "Pause"), Some(UiIcon::Pause))
                        };
                        if pause_resp.clicked() {
                            self.is_paused.store(!paused, Ordering::Relaxed);
                        }

                        let mut cur_lang = cfg.language;
                        if segmented_control(
                            ui,
                            &mut cur_lang,
                            &[(Language::Ru, "RU"), (Language::En, "EN")],
                        ) {
                            cfg.language = cur_lang;
                            changed = true;
                        }

                        ui.add_space(4.0);

                        // Non-interactive status indicator
                        let (status_dot, status_text) = if paused {
                            (STATUS_PAUSED_DOT, lang.t("На паузе", "Paused"))
                        } else {
                            (STATUS_ACTIVE_DOT, lang.t("Работает", "Active"))
                        };
                        status_indicator(ui, status_dot, status_text);
                    });
                });
            });

        // --- LEFT PINNED PREVIEW COLUMN ---
        egui::Panel::left("preview_sidebar")
            .resizable(false)
            .exact_size(322.0)
            .show_separator_line(false)
            .frame(
                egui::Frame::new()
                    .fill(BG_APP)
                    .inner_margin(egui::Margin {
                        left: 12,
                        right: 6,
                        top: 10,
                        bottom: 10,
                    }),
            )
            .show(ui, |ui| {
                section_card(ui, lang.t("Предпросмотр", "Preview"), |ui| {
                    // View mode + Background controls
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(lang.t("Вид", "View"))
                                .size(12.5)
                                .color(TEXT_SECONDARY),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            segmented_control(
                                ui,
                                &mut self.preview_lens_mode,
                                &[
                                    (false, lang.t("Сетка", "Reticle")),
                                    (true, lang.t("Линза", "Lens")),
                                ],
                            );
                        });
                    });

                    ui.add_space(4.0);

                    segmented_control(
                        ui,
                        &mut self.preview_bg,
                        &[
                            (PreviewBg::Night, lang.t("Ночь", "Night")),
                            (PreviewBg::City, lang.t("Город", "City")),
                            (PreviewBg::Desert, lang.t("Песок", "Desert")),
                            (PreviewBg::Forest, lang.t("Лес", "Forest")),
                            (PreviewBg::Snow, lang.t("Снег", "Snow")),
                        ],
                    );

                    ui.add_space(6.0);

                    // Preview Canvas (shows chosen background in both crosshair and lens views)
                    let canvas_w = ui.available_width();
                    let (rect, _) = ui.allocate_exact_size(
                        egui::vec2(canvas_w, 224.0),
                        egui::Sense::hover(),
                    );
                    if self.preview_lens_mode {
                        draw_lens_preview_canvas(ui, rect, &cfg, self.preview_bg);
                    } else {
                        draw_advanced_preview_canvas(
                            ui,
                            rect,
                            &cfg,
                            self.preview_bg,
                            self.preview_zoom,
                        );
                    }

                    ui.add_space(6.0);

                    if self.preview_lens_mode {
                        let summary = if cfg.is_no_zoom_mode() {
                            lang.t("Режим без приближения (1.0x)", "No-zoom mode (1.0x)")
                                .to_string()
                        } else {
                            format!(
                                "{}: {:.1}x  ·  {}: {} {}",
                                lang.t("Кратность", "Zoom"),
                                cfg.zoom_factor,
                                lang.t("Область", "Size"),
                                cfg.scope_size,
                                lang.t("пкс", "px"),
                            )
                        };
                        ui.label(
                            egui::RichText::new(summary)
                                .size(12.0)
                                .color(TEXT_SECONDARY),
                        );
                    } else {
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new(lang.t("Масштаб", "Scale"))
                                    .size(12.5)
                                    .color(TEXT_SECONDARY),
                            );
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    let mut zoom_choice = if (self.preview_zoom - 3.0).abs() < 0.1 {
                                        3
                                    } else if (self.preview_zoom - 2.0).abs() < 0.1 {
                                        2
                                    } else {
                                        1
                                    };
                                    if segmented_control(
                                        ui,
                                        &mut zoom_choice,
                                        &[(1, "100%"), (2, "200%"), (3, "300%")],
                                    ) {
                                        self.preview_zoom = zoom_choice as f32;
                                    }
                                },
                            );
                        });
                    }
                });

                ui.add_space(8.0);

                // Share code block stays pinned under preview
                section_card(ui, lang.t("Код прицела", "Crosshair code"), |ui| {
                    changed |= self.ui_share_code_block(ui, &mut cfg);
                });
            });

        // --- RIGHT SCROLLABLE SETTINGS COLUMN ---
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(BG_APP)
                    .inner_margin(egui::Margin {
                        left: 6,
                        right: 12,
                        top: 10,
                        bottom: 10,
                    }),
            )
            .show(ui, |ui| {
                // Status toast bar
                if let Some((ref msg, time)) = self.status_msg {
                    if time.elapsed().as_secs() < 5 {
                        egui::Frame::new()
                            .fill(TOAST_BG)
                            .stroke(egui::Stroke::new(1.0, TOAST_BORDER))
                            .corner_radius(6.0)
                            .inner_margin(egui::Margin::symmetric(10, 6))
                            .show(ui, |ui| {
                                ui.set_width(ui.available_width());
                                ui.label(egui::RichText::new(msg).size(12.5).color(TOAST_TEXT));
                            });
                        ui.add_space(6.0);
                    }
                }

                // Active listening banner (if user is capturing a key)
                if let Some(slot) = self.listening_slot {
                    egui::Frame::new()
                        .fill(DANGER_BG)
                        .stroke(egui::Stroke::new(1.0, DANGER_BORDER))
                        .corner_radius(6.0)
                        .inner_margin(egui::Margin::symmetric(10, 7))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{} «{}» ({})",
                                        lang.t("Нажмите клавишу или кнопку мыши для", "Press a key or mouse button for"),
                                        Config::slot_label(slot, lang),
                                        lang.t("Del — отвязать, Esc — отмена", "Del — unbind, Esc — cancel"),
                                    ))
                                    .size(12.5)
                                    .color(DANGER_TEXT),
                                );
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if action_button(ui, lang.t("Отмена", "Cancel"), None).clicked() {
                                            self.listening_slot = None;
                                            self.listening_since = None;
                                        }
                                    },
                                );
                            });
                        });
                    ui.add_space(6.0);
                }

                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| match self.active_tab {
                        GuiTab::CrosshairEditor => {
                            changed |= self.ui_crosshair_editor_tab(ui, &mut cfg);
                        }
                        GuiTab::OpticsAndZoom => {
                            changed |= self.ui_optics_tab(ui, &mut cfg);
                        }
                        GuiTab::Hotkeys => {
                            changed |= self.ui_hotkeys_tab(ui, &mut cfg);
                        }
                        GuiTab::Profiles => {
                            changed |= self.ui_profiles_tab(ui, &mut cfg);
                        }
                        GuiTab::System => {
                            changed |= self.ui_system_tab(ui, &mut cfg);
                        }
                    });
            });

        if changed {
            cfg.sanitize();
            let mut writer = self.config.write().unwrap();
            *writer = cfg.clone();
            let _ = cfg.save();
        }
    }
}

impl SniperZoomApp {
    // =========================================================================
    // SHARE CODE BLOCK (Pinned in left sidebar)
    // =========================================================================
    fn ui_share_code_block(&mut self, ui: &mut egui::Ui, cfg: &mut Config) -> bool {
        let mut changed = false;
        let lang = cfg.language;

        ui.add(
            egui::TextEdit::singleline(&mut self.share_code_buf)
                .desired_width(ui.available_width())
                .font(egui::TextStyle::Monospace),
        );

        ui.add_space(6.0);

        ui.horizontal(|ui| {
            if action_button(ui, lang.t("Копировать", "Copy"), Some(UiIcon::Copy)).clicked() {
                let code = cfg.export_crosshair_code();
                self.share_code_buf = code.clone();
                ui.ctx().copy_text(code);
                self.set_status(lang.t("Код прицела скопирован в буфер обмена", "Crosshair code copied to clipboard"));
            }
            if action_button(ui, lang.t("Импорт", "Import"), Some(UiIcon::Import)).clicked() {
                let code = self.share_code_buf.clone();
                if cfg.import_crosshair_code(&code) {
                    self.sync_hex_buffers(cfg);
                    changed = true;
                    self.set_status(lang.t("Прицел загружен из кода", "Crosshair imported from code"));
                } else {
                    self.set_status(lang.t("Неверный формат кода прицела (ожидается SZ2;...)", "Invalid crosshair code format (expected SZ2;...)"));
                }
            }

            // Visually separated destructive reset action on the opposite side
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if destructive_button(ui, lang.t("Сбросить", "Reset"), Some(UiIcon::Reset))
                    .on_hover_text(lang.t("Сбросить настройки прицела к стандартному виду", "Reset crosshair settings to default"))
                    .clicked()
                {
                    cfg.apply_crosshair_preset("cs2_pro");
                    self.sync_hex_buffers(cfg);
                    changed = true;
                    self.set_status(lang.t("Прицел сброшен к стандартному виду", "Crosshair reset to default"));
                }
            });
        });

        changed
    }

    // =========================================================================
    // TAB 1: CROSSHAIR EDITOR
    // =========================================================================
    fn ui_crosshair_editor_tab(&mut self, ui: &mut egui::Ui, cfg: &mut Config) -> bool {
        let mut changed = false;
        let lang = cfg.language;
        let px_suffix = lang.t(" пкс", " px");

        // 1. Visibility & Scaling toggles
        section_card(ui, lang.t("Отображение прицела", "Crosshair visibility"), |ui| {
            let mut always_on = if cfg.mode.eq_ignore_ascii_case("always") && cfg.is_no_zoom_mode() {
                cfg.crosshair_enabled
            } else {
                cfg.always_show_crosshair
            };
            if toggle_switch(
                ui,
                &mut always_on,
                lang.t("Показывать прицел постоянно поверх экрана", "Always show crosshair on screen"),
            )
            .changed()
            {
                cfg.always_show_crosshair = always_on;
                cfg.crosshair_enabled = always_on;
                if always_on && cfg.crosshair.eq_ignore_ascii_case("none") {
                    cfg.apply_crosshair_style("cross");
                }
                changed = true;
            }

            ui.add_space(4.0);
            if toggle_switch(
                ui,
                &mut cfg.hide_crosshair_on_zoom,
                lang.t("Скрывать сетку при приближении", "Hide reticle when zoomed in"),
            )
            .changed()
            {
                changed = true;
            }

            ui.add_space(4.0);
            if toggle_switch(
                ui,
                &mut cfg.xhair_scale_with_zoom,
                lang.t("Масштабировать вместе с зумом", "Scale reticle with zoom"),
            )
            .on_hover_text(lang.t("Пропорционально увеличивает элементы сетки при приближении", "Proportionally scales reticle elements when zoomed"))
            .changed()
            {
                changed = true;
            }
        });

        ui.add_space(8.0);

        // 2. Reticle Shape Selector with 48x48 visual thumbnails (no orphan "Выкл" button)
        section_card(ui, lang.t("Форма сетки", "Reticle shape"), |ui| {
            let styles = [
                ("dot", lang.t("Точка", "Dot")),
                ("cross", lang.t("Крест", "Cross")),
                ("t_cross", lang.t("Т-образный", "T-Cross")),
                ("x_cross", lang.t("Х-образный", "X-Cross")),
                ("circle_dot", lang.t("Коллиматор", "Red Dot")),
                ("box", lang.t("Рамка", "Box")),
                ("mildot", lang.t("С делениями", "Mil-Dot")),
                ("chevron", lang.t("Угольник ПСО", "Chevron")),
            ];

            let cols = 4usize;
            let spacing = 8.0;
            let tile_w = ((ui.available_width() - spacing * (cols as f32 - 1.0)) / cols as f32)
                .floor()
                .max(96.0);

            for row in styles.chunks(cols) {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = spacing;
                    for &(id, label) in row {
                        let selected = cfg.crosshair.eq_ignore_ascii_case(id);
                        let mut thumb_cfg = Config::default();
                        thumb_cfg.crosshair_color = "#F3F4F6".to_string();
                        thumb_cfg.xhair_outline = false;
                        thumb_cfg.apply_crosshair_style(id);

                        if thumbnail_tile(ui, tile_w, selected, label, &thumb_cfg).clicked() {
                            cfg.apply_crosshair_style(id);
                            self.share_code_buf = cfg.export_crosshair_code();
                            changed = true;
                        }
                    }
                });
                ui.add_space(spacing);
            }
        });

        ui.add_space(8.0);

        // 3. Colors & Outline
        section_card(ui, lang.t("Цвета и обводка", "Colors & outline"), |ui| {
            ui.label(
                egui::RichText::new(lang.t("Основной цвет", "Primary color"))
                    .size(12.5)
                    .color(TEXT_SECONDARY),
            );
            ui.add_space(2.0);
            if color_swatch_row(ui, &mut cfg.crosshair_color, &mut self.temp_hex, lang) {
                changed = true;
            }

            ui.add_space(6.0);
            if toggle_switch(
                ui,
                &mut cfg.xhair_use_secondary_color,
                lang.t("Отдельный цвет центральной точки", "Separate center dot color"),
            )
            .changed()
            {
                changed = true;
            }
            if cfg.xhair_use_secondary_color {
                ui.add_space(4.0);
                if color_swatch_row(ui, &mut cfg.xhair_secondary_color, &mut self.temp_sec_hex, lang) {
                    changed = true;
                }
            }

            ui.add_space(6.0);
            ui.separator();
            ui.add_space(4.0);

            if toggle_switch(ui, &mut cfg.xhair_outline, lang.t("Контрастная обводка", "Contrast outline")).changed() {
                changed = true;
            }
            if cfg.xhair_outline {
                ui.add_space(4.0);
                labeled_slider(
                    ui,
                    lang.t("Толщина обводки", "Outline thickness"),
                    egui::Slider::new(&mut cfg.xhair_outline_thickness, 1..=4).suffix(px_suffix),
                    &mut changed,
                );
                ui.add_space(4.0);
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        egui::RichText::new(lang.t("Цвет обводки", "Outline color"))
                            .size(12.5)
                            .color(TEXT_SECONDARY),
                    );
                    let mut out_choice = if cfg.xhair_outline_color.eq_ignore_ascii_case("#000000") {
                        0
                    } else if cfg.xhair_outline_color.eq_ignore_ascii_case("#181820") {
                        1
                    } else if cfg.xhair_outline_color.eq_ignore_ascii_case("#FFFFFF") {
                        2
                    } else {
                        -1
                    };
                    if segmented_control(
                        ui,
                        &mut out_choice,
                        &[
                            (0, lang.t("Чёрный", "Black")),
                            (1, lang.t("Тёмный", "Dark")),
                            (2, lang.t("Белый", "White")),
                        ],
                    ) {
                        let hex = match out_choice {
                            1 => "#181820",
                            2 => "#FFFFFF",
                            _ => "#000000",
                        };
                        cfg.xhair_outline_color = hex.to_string();
                        self.temp_out_hex = hex.to_string();
                        changed = true;
                    }
                    let resp = ui.add(
                        egui::TextEdit::singleline(&mut self.temp_out_hex).desired_width(68.0),
                    );
                    if resp.changed() {
                        cfg.xhair_outline_color = self.temp_out_hex.clone();
                        changed = true;
                    }
                });
            }
        });

        ui.add_space(8.0);

        // 4. Crosshair Lines
        section_card(ui, lang.t("Линии перекрестия", "Crosshair lines"), |ui| {
            if toggle_switch(ui, &mut cfg.xhair_show_lines, lang.t("Показывать лучи", "Show lines")).changed() {
                changed = true;
            }

            ui.add_space(6.0);
            ui.add_enabled_ui(cfg.xhair_show_lines, |ui| {
                ui.horizontal_wrapped(|ui| {
                    changed |= check_option(ui, &mut cfg.xhair_top_line, lang.t("Верх", "Top")).changed();
                    changed |= check_option(ui, &mut cfg.xhair_bottom_line, lang.t("Низ", "Bottom")).changed();
                    changed |= check_option(ui, &mut cfg.xhair_left_line, lang.t("Лево", "Left")).changed();
                    changed |= check_option(ui, &mut cfg.xhair_right_line, lang.t("Право", "Right")).changed();
                    changed |= check_option(ui, &mut cfg.xhair_diagonal, "45°").changed();
                });

                ui.add_space(6.0);
                labeled_slider(
                    ui,
                    lang.t("Толщина", "Thickness"),
                    egui::Slider::new(&mut cfg.crosshair_size, 1..=15).suffix(px_suffix),
                    &mut changed,
                );
                labeled_slider(
                    ui,
                    lang.t("Зазор", "Gap"),
                    egui::Slider::new(&mut cfg.crosshair_gap, -10..=50).suffix(px_suffix),
                    &mut changed,
                );
                labeled_slider(
                    ui,
                    if cfg.xhair_unlink_axis {
                        lang.t("Длина по X", "Length X")
                    } else {
                        lang.t("Длина", "Length")
                    },
                    egui::Slider::new(&mut cfg.crosshair_length, 2..=100).suffix(px_suffix),
                    &mut changed,
                );

                ui.add_space(4.0);
                if toggle_switch(
                    ui,
                    &mut cfg.xhair_unlink_axis,
                    lang.t("Раздельная длина по вертикали", "Separate vertical length"),
                )
                .changed()
                {
                    changed = true;
                }
                if cfg.xhair_unlink_axis {
                    ui.add_space(4.0);
                    labeled_slider(
                        ui,
                        lang.t("Длина по Y", "Length Y"),
                        egui::Slider::new(&mut cfg.xhair_v_length, 2..=100).suffix(px_suffix),
                        &mut changed,
                    );
                }
            });
        });

        ui.add_space(8.0);

        // 5. Center Dot & Ring (Two cleanly separated sections)
        ui.columns(2, |cols| {
            cols[0].vertical(|ui| {
                section_card(ui, lang.t("Центральная точка", "Center dot"), |ui| {
                    if toggle_switch(ui, &mut cfg.xhair_center_dot, lang.t("Показывать точку", "Show dot")).changed() {
                        changed = true;
                    }

                    ui.add_space(6.0);
                    ui.add_enabled_ui(cfg.xhair_center_dot, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new(lang.t("Форма", "Shape"))
                                    .size(12.5)
                                    .color(TEXT_PRIMARY),
                            );
                            let mut is_sq = cfg.xhair_dot_shape.eq_ignore_ascii_case("square");
                            if segmented_control(
                                ui,
                                &mut is_sq,
                                &[
                                    (false, lang.t("Круг", "Circle")),
                                    (true, lang.t("Квадрат", "Square")),
                                ],
                            ) {
                                cfg.xhair_dot_shape = if is_sq {
                                    "square".to_string()
                                } else {
                                    "circle".to_string()
                                };
                                changed = true;
                            }
                        });
                        ui.add_space(4.0);
                        labeled_slider(
                            ui,
                            lang.t("Размер", "Size"),
                            egui::Slider::new(&mut cfg.xhair_dot_size, 1..=12).suffix(px_suffix),
                            &mut changed,
                        );
                    });
                });
            });

            cols[1].vertical(|ui| {
                section_card(ui, lang.t("Кольцо", "Ring"), |ui| {
                    if toggle_switch(ui, &mut cfg.xhair_show_circle, lang.t("Показывать кольцо", "Show ring")).changed() {
                        changed = true;
                    }

                    ui.add_space(6.0);
                    ui.add_enabled_ui(cfg.xhair_show_circle, |ui| {
                        labeled_slider(
                            ui,
                            lang.t("Радиус", "Radius"),
                            egui::Slider::new(&mut cfg.xhair_circle_radius, 4..=120)
                                .suffix(px_suffix),
                            &mut changed,
                        );
                        labeled_slider(
                            ui,
                            lang.t("Толщина", "Thickness"),
                            egui::Slider::new(&mut cfg.xhair_circle_thickness, 1..=8)
                                .suffix(px_suffix),
                            &mut changed,
                        );
                    });
                });
            });
        });

        ui.add_space(8.0);

        // 6. Corner Brackets & Ballistic Ticks
        section_card(ui, lang.t("Уголки и деления", "Brackets & ticks"), |ui| {
            if toggle_switch(ui, &mut cfg.xhair_corner_brackets, lang.t("Уголки по краям", "Corner brackets")).changed() {
                changed = true;
            }
            ui.add_space(4.0);
            ui.add_enabled_ui(cfg.xhair_corner_brackets, |ui| {
                labeled_slider(
                    ui,
                    lang.t("Размер уголков", "Bracket size"),
                    egui::Slider::new(&mut cfg.xhair_bracket_size, 6..=50).suffix(px_suffix),
                    &mut changed,
                );
                labeled_slider(
                    ui,
                    lang.t("Отступ от центра", "Center offset"),
                    egui::Slider::new(&mut cfg.xhair_bracket_dist, 10..=120).suffix(px_suffix),
                    &mut changed,
                );
            });

            ui.add_space(6.0);
            ui.separator();
            ui.add_space(4.0);

            if toggle_switch(ui, &mut cfg.xhair_ticks, lang.t("Дальномерные деления", "Rangefinder ticks")).changed() {
                changed = true;
            }
            ui.add_space(4.0);
            ui.add_enabled_ui(cfg.xhair_ticks, |ui| {
                labeled_slider(
                    ui,
                    lang.t("Количество", "Count"),
                    egui::Slider::new(&mut cfg.xhair_tick_count, 1..=6),
                    &mut changed,
                );
                labeled_slider(
                    ui,
                    lang.t("Шаг делений", "Tick spacing"),
                    egui::Slider::new(&mut cfg.xhair_tick_step, 6..=35).suffix(px_suffix),
                    &mut changed,
                );
            });
        });

        ui.add_space(8.0);

        // 7. Presets with 48x48 visual thumbnails (no promotional subtitle)
        section_card(ui, lang.t("Готовые пресеты", "Ready presets"), |ui| {
            let presets = [
                ("cs2_pro", lang.t("Классический", "Classic")),
                ("valorant_cyan", lang.t("Бирюзовый", "Cyan")),
                ("headshot_t", lang.t("Т-образный", "T-Cross")),
                ("eotech_holo", lang.t("Коллиматор", "Holo Sight")),
                ("sniper_mildot", lang.t("Дальномерный", "Mil-Dot")),
                ("pso_chevron", lang.t("Угольник ПСО", "Chevron")),
                ("predator_box", lang.t("Тактическая рамка", "Tactical Box")),
                ("hitmarker_x", lang.t("Х-образный", "X-Cross")),
                ("minimal_dot", lang.t("Точка", "Dot")),
            ];

            let cols = 3usize;
            let spacing = 8.0;
            let tile_w = ((ui.available_width() - spacing * (cols as f32 - 1.0)) / cols as f32)
                .floor()
                .max(115.0);

            for row in presets.chunks(cols) {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = spacing;
                    for &(id, label) in row {
                        let mut preset_cfg = Config::default();
                        preset_cfg.apply_crosshair_preset(id);
                        if thumbnail_tile(ui, tile_w, false, label, &preset_cfg).clicked() {
                            cfg.apply_crosshair_preset(id);
                            self.sync_hex_buffers(cfg);
                            changed = true;
                            self.set_status(format!(
                                "{}: {}",
                                lang.t("Загружен пресет", "Loaded preset"),
                                label
                            ));
                        }
                    }
                });
                ui.add_space(spacing);
            }
        });

        changed
    }

    // =========================================================================
    // TAB 2: OPTICS & ZOOM
    // =========================================================================
    fn ui_optics_tab(&mut self, ui: &mut egui::Ui, cfg: &mut Config) -> bool {
        let mut changed = false;
        let lang = cfg.language;
        let px_suffix = lang.t(" пкс", " px");

        // 1. Unified Operating Mode
        section_card(ui, lang.t("Режим работы", "Operating mode"), |ui| {
            let mut op_mode = cfg.operating_mode();
            let modes = [
                (OperatingMode::ZoomWithCrosshair, lang.t("Прицел при приближении", "Crosshair on zoom")),
                (
                    OperatingMode::AlwaysCrosshairPlusZoom,
                    lang.t("Постоянный прицел и зум", "Always crosshair + zoom"),
                ),
                (OperatingMode::CrosshairOnly, lang.t("Только прицел (без зума)", "Crosshair only (no zoom)")),
                (OperatingMode::ZoomOnly, lang.t("Чистый зум (без прицела)", "Zoom only (no crosshair)")),
            ];
            if segmented_control_grid(ui, &mut op_mode, &modes, 2) {
                cfg.set_operating_mode(op_mode);
                changed = true;
            }
        });

        ui.add_space(8.0);

        // 2. Magnification
        section_card(ui, lang.t("Кратность приближения", "Zoom magnification"), |ui| {
            let zoom_label = if cfg.is_no_zoom_mode() {
                lang.t("1.0x (без зума)", "1.0x (no zoom)").to_string()
            } else {
                format!("{:.1}x", cfg.zoom_factor)
            };
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(lang.t("Текущая кратность:", "Current magnification:"))
                        .size(12.5)
                        .color(TEXT_SECONDARY),
                );
                ui.label(
                    egui::RichText::new(zoom_label)
                        .size(13.0)
                        .strong()
                        .color(TEXT_PRIMARY),
                );
            });
            ui.add_space(4.0);
            if ui
                .add(
                    egui::Slider::new(&mut cfg.zoom_factor, 1.0..=10.0)
                        .step_by(0.1)
                        .suffix("x"),
                )
                .changed()
            {
                cfg.crosshair_only = cfg.zoom_factor <= 1.02;
                changed = true;
            }

            ui.add_space(6.0);
            let mut zoom_preset_idx = if cfg.is_no_zoom_mode() {
                0i32
            } else {
                let presets = [1.0f32, 1.5, 2.0, 2.5, 3.0, 4.0, 5.0, 6.0, 8.0, 10.0];
                presets
                    .iter()
                    .position(|&z| z > 1.02 && (cfg.zoom_factor - z).abs() < 0.08)
                    .map(|i| i as i32)
                    .unwrap_or(-1)
            };
            if segmented_control(
                ui,
                &mut zoom_preset_idx,
                &[
                    (0, "1.0x"),
                    (1, "1.5x"),
                    (2, "2.0x"),
                    (3, "2.5x"),
                    (4, "3.0x"),
                    (5, "4.0x"),
                    (6, "5.0x"),
                    (7, "6.0x"),
                    (8, "8.0x"),
                    (9, "10.0x"),
                ],
            ) {
                let presets = [1.0f32, 1.5, 2.0, 2.5, 3.0, 4.0, 5.0, 6.0, 8.0, 10.0];
                if let Some(&z) = presets.get(zoom_preset_idx as usize) {
                    cfg.zoom_factor = z;
                    cfg.crosshair_only = z <= 1.02;
                    if cfg.crosshair_only && cfg.crosshair.eq_ignore_ascii_case("none") {
                        cfg.apply_crosshair_style("cross");
                    }
                    changed = true;
                }
            }
        });

        ui.add_space(8.0);

        // 3. Lens Size, Shape & Border
        section_card(ui, lang.t("Область приближения и рамка", "Scope area & border"), |ui| {
            labeled_slider(
                ui,
                lang.t("Размер области", "Scope size"),
                egui::Slider::new(&mut cfg.scope_size, 150..=1600)
                    .step_by(10.0)
                    .suffix(px_suffix),
                &mut changed,
            );

            ui.add_space(4.0);
            let sizes = [350, 500, 700, 900, 1150, 1400, 1600];
            let mut size_choice = sizes
                .iter()
                .position(|&sz| (cfg.scope_size - sz).abs() < 25)
                .map(|i| i as i32)
                .unwrap_or(-1);
            if segmented_control(
                ui,
                &mut size_choice,
                &[
                    (0, "350"),
                    (1, "500"),
                    (2, "700"),
                    (3, "900"),
                    (4, "1150"),
                    (5, "1400"),
                    (6, lang.t("1600 пкс", "1600 px")),
                ],
            ) {
                if let Some(&sz) = sizes.get(size_choice as usize) {
                    cfg.scope_size = sz;
                    changed = true;
                }
            }

            ui.add_space(8.0);
            ui.label(
                egui::RichText::new(lang.t("Форма области", "Scope shape"))
                    .size(12.5)
                    .color(TEXT_SECONDARY),
            );
            ui.add_space(3.0);
            let mut shape_idx = match cfg.shape.to_lowercase().as_str() {
                "square" => 1,
                "wide" => 2,
                "fullscreen" => 3,
                _ => 0,
            };
            if segmented_control(
                ui,
                &mut shape_idx,
                &[
                    (0, lang.t("Круг (1:1)", "Circle (1:1)")),
                    (1, lang.t("Квадрат (1:1)", "Square (1:1)")),
                    (2, lang.t("Панорама (16:9)", "Wide (16:9)")),
                    (3, lang.t("Весь экран", "Fullscreen")),
                ],
            ) {
                cfg.shape = match shape_idx {
                    1 => "square",
                    2 => "wide",
                    3 => "fullscreen",
                    _ => "circle",
                }
                .to_string();
                changed = true;
            }

            ui.add_space(8.0);
            ui.label(
                egui::RichText::new(lang.t("Рамка линзы", "Lens border"))
                    .size(12.5)
                    .color(TEXT_SECONDARY),
            );
            ui.add_space(3.0);
            let mut border_idx = match cfg.border_style.to_lowercase().as_str() {
                "neon" => 1,
                "thin" => 2,
                "none" => 3,
                _ => 0,
            };
            if segmented_control(
                ui,
                &mut border_idx,
                &[
                    (0, lang.t("Тактическая", "Tactical")),
                    (1, lang.t("В цвет прицела", "Crosshair color")),
                    (2, lang.t("Тонкая 1 пкс", "Thin 1 px")),
                    (3, lang.t("Без рамки", "None")),
                ],
            ) {
                cfg.border_style = match border_idx {
                    1 => "neon",
                    2 => "thin",
                    3 => "none",
                    _ => "tactical",
                }
                .to_string();
                changed = true;
            }
        });

        ui.add_space(8.0);

        // 4. Tracking & Filtering
        section_card(ui, lang.t("Позиционирование и сглаживание", "Positioning & filtering"), |ui| {
            if toggle_switch(
                ui,
                &mut cfg.follow_mouse,
                lang.t("Следовать за курсором мыши", "Follow mouse cursor"),
            )
            .changed()
            {
                changed = true;
            }

            ui.add_space(6.0);
            labeled_slider(
                ui,
                lang.t("Смещение по X", "Offset X"),
                egui::Slider::new(&mut cfg.offset_x, -300..=300).suffix(px_suffix),
                &mut changed,
            );
            labeled_slider(
                ui,
                lang.t("Смещение по Y", "Offset Y"),
                egui::Slider::new(&mut cfg.offset_y, -300..=300).suffix(px_suffix),
                &mut changed,
            );

            if cfg.offset_x != 0 || cfg.offset_y != 0 {
                ui.add_space(4.0);
                if action_button(
                    ui,
                    lang.t("Сбросить смещение (0, 0)", "Reset offset (0, 0)"),
                    Some(UiIcon::Reset),
                )
                .clicked()
                {
                    cfg.offset_x = 0;
                    cfg.offset_y = 0;
                    changed = true;
                }
            }

            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(lang.t("Фильтрация", "Scaling filter"))
                        .size(12.5)
                        .color(TEXT_SECONDARY),
                );
                let mut is_sharp = cfg.scaling.eq_ignore_ascii_case("sharp");
                if segmented_control(
                    ui,
                    &mut is_sharp,
                    &[
                        (false, lang.t("Плавная", "Smooth")),
                        (true, lang.t("Чёткие пиксели", "Sharp pixels")),
                    ],
                ) {
                    cfg.scaling = if is_sharp { "sharp" } else { "smooth" }.to_string();
                    changed = true;
                }
            });
        });

        changed
    }

    // =========================================================================
    // TAB 3: HOTKEYS
    // =========================================================================
    fn ui_hotkeys_tab(&mut self, ui: &mut egui::Ui, cfg: &mut Config) -> bool {
        let mut changed = false;
        let lang = cfg.language;

        section_card(ui, lang.t("Основные клавиши", "Primary hotkeys"), |ui| {
            ui.label(
                egui::RichText::new(lang.t(
                    "При назначении клавиши она автоматически отвязывается от других действий.",
                    "Assigning a key automatically unbinds it from any other action.",
                ))
                .size(12.0)
                .color(TEXT_SECONDARY),
            );
            ui.add_space(8.0);

            changed |= self.render_hotkey_row(
                ui,
                cfg,
                HotkeySlot::Zoom,
                Config::slot_label(HotkeySlot::Zoom, lang),
            );
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(lang.t("Режим клавиши:", "Key mode:"))
                        .size(12.5)
                        .color(TEXT_SECONDARY),
                );
                let mut mode_idx = match cfg.mode.to_lowercase().as_str() {
                    "toggle" => 1,
                    "always" => 2,
                    _ => 0,
                };
                if segmented_control(
                    ui,
                    &mut mode_idx,
                    &[
                        (0, lang.t("Удержание", "Hold")),
                        (1, lang.t("Переключение", "Toggle")),
                        (2, lang.t("Постоянно", "Always")),
                    ],
                ) {
                    cfg.mode = match mode_idx {
                        1 => "toggle",
                        2 => "always",
                        _ => "hold",
                    }
                    .to_string();
                    changed = true;
                }
            });

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(8.0);

            changed |= self.render_hotkey_row(
                ui,
                cfg,
                HotkeySlot::Crosshair,
                Config::slot_label(HotkeySlot::Crosshair, lang),
            );
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(lang.t("Режим клавиши:", "Key mode:"))
                        .size(12.5)
                        .color(TEXT_SECONDARY),
                );
                let mut is_hold = cfg.crosshair_mode.eq_ignore_ascii_case("hold");
                if segmented_control(
                    ui,
                    &mut is_hold,
                    &[
                        (false, lang.t("Переключение", "Toggle")),
                        (true, lang.t("Удержание", "Hold")),
                    ],
                ) {
                    cfg.crosshair_mode = if is_hold { "hold" } else { "toggle" }.to_string();
                    changed = true;
                }
            });
        });

        ui.add_space(8.0);

        section_card(ui, lang.t("Быстрые клавиши в игре", "In-game quick hotkeys"), |ui| {
            ui.horizontal(|ui| {
                if toggle_switch(
                    ui,
                    &mut cfg.in_game_hotkeys,
                    lang.t("Включить быстрые клавиши в игре", "Enable in-game quick hotkeys"),
                )
                .changed()
                {
                    changed = true;
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if destructive_button(ui, lang.t("Очистить все", "Clear all"), None)
                        .on_hover_text(lang.t("Отвязать все дополнительные клавиши", "Unbind all quick hotkeys"))
                        .clicked()
                    {
                        cfg.clear_quick_hotkeys();
                        changed = true;
                        self.set_status(lang.t("Дополнительные игровые клавиши отвязаны", "Quick hotkeys cleared"));
                    }
                    ui.add_space(4.0);
                    if action_button(ui, lang.t("По умолчанию", "Defaults"), Some(UiIcon::Reset))
                        .on_hover_text(lang.t(
                            "Восстановить стандартные клавиши (V, F7, [, ], -, =, F8–F11)",
                            "Restore default hotkeys (V, F7, [, ], -, =, F8–F11)",
                        ))
                        .clicked()
                    {
                        cfg.reset_hotkeys_to_default();
                        changed = true;
                        self.set_status(lang.t("Клавиши сброшены к значениям по умолчанию", "Hotkeys reset to defaults"));
                    }
                });
            });

            ui.add_space(8.0);

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

            ui.add_enabled_ui(cfg.in_game_hotkeys, |ui| {
                for slot in quick_slots {
                    changed |= self.render_compact_hotkey_row(ui, cfg, slot);
                }
            });
        });

        changed
    }

    fn render_hotkey_row(
        &mut self,
        ui: &mut egui::Ui,
        cfg: &mut Config,
        slot: HotkeySlot,
        label: &str,
    ) -> bool {
        let mut changed = false;
        let lang = cfg.language;
        let is_listening = self.listening_slot == Some(slot);
        let raw_key = cfg.get_hotkey(slot).to_string();
        let is_bound = !raw_key.trim().is_empty();
        let pretty = format_hotkey_display_lang(&raw_key, lang);

        ui.horizontal_wrapped(|ui| {
            ui.add_sized(
                [155.0, 28.0],
                egui::Label::new(egui::RichText::new(label).size(13.0).color(TEXT_PRIMARY)),
            );

            let btn_text = if is_listening {
                lang.t("Нажмите клавишу...", "Press a key...").to_string()
            } else if is_bound {
                format!("[ {} ] — {}", pretty, lang.t("Изменить", "Change"))
            } else {
                lang.t("Не назначено — Назначить", "Not set — Bind").to_string()
            };

            let resp = if is_listening {
                destructive_button(ui, &btn_text, None)
            } else {
                hotkey_bind_button(ui, &btn_text, is_bound, 195.0)
            };

            if resp.clicked() {
                if is_listening {
                    self.listening_slot = None;
                    self.listening_since = None;
                } else {
                    self.listening_slot = Some(slot);
                    self.listening_since = Some(Instant::now());
                }
            }

            ui.add_space(6.0);
            if ui
                .add_enabled_ui(is_bound, |ui| action_button(ui, lang.t("Отвязать", "Unbind"), None))
                .inner
                .on_hover_text(lang.t("Убрать привязку клавиши", "Remove key binding"))
                .clicked()
            {
                cfg.clear_hotkey(slot);
                if is_listening {
                    self.listening_slot = None;
                    self.listening_since = None;
                }
                changed = true;
                self.set_status(format!(
                    "{} «{}»",
                    lang.t("Клавиша отвязана:", "Key unbound:"),
                    Config::slot_label(slot, lang)
                ));
            }
        });

        changed
    }

    fn render_compact_hotkey_row(
        &mut self,
        ui: &mut egui::Ui,
        cfg: &mut Config,
        slot: HotkeySlot,
    ) -> bool {
        let mut changed = false;
        let lang = cfg.language;
        let is_listening = self.listening_slot == Some(slot);
        let raw_key = cfg.get_hotkey(slot).to_string();
        let is_bound = !raw_key.trim().is_empty();
        let pretty = format_hotkey_display_lang(&raw_key, lang);

        egui::Frame::new()
            .fill(BG_PANEL)
            .stroke(egui::Stroke::new(1.0, BORDER_SUBTLE))
            .corner_radius(5.0)
            .inner_margin(egui::Margin::symmetric(8, 6))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(Config::slot_label(slot, lang))
                            .size(12.0)
                            .color(TEXT_PRIMARY),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add_enabled_ui(is_bound, |ui| {
                                action_button(ui, "", Some(UiIcon::Close))
                            })
                            .inner
                            .on_hover_text(lang.t("Отвязать клавишу", "Unbind key"))
                            .clicked()
                        {
                            cfg.clear_hotkey(slot);
                            if is_listening {
                                self.listening_slot = None;
                                self.listening_since = None;
                            }
                            changed = true;
                            self.set_status(format!(
                                "{} «{}»",
                                lang.t("Клавиша отвязана:", "Key unbound:"),
                                Config::slot_label(slot, lang)
                            ));
                        }

                        let btn_text = if is_listening {
                            lang.t("Нажмите...", "Press...").to_string()
                        } else if is_bound {
                            pretty
                        } else {
                            lang.t("Нет", "None").to_string()
                        };
                        let resp = if is_listening {
                            destructive_button(ui, &btn_text, None)
                        } else {
                            hotkey_bind_button(ui, &btn_text, is_bound, 96.0)
                        };
                        if resp.clicked() {
                            if is_listening {
                                self.listening_slot = None;
                                self.listening_since = None;
                            } else {
                                self.listening_slot = Some(slot);
                                self.listening_since = Some(Instant::now());
                            }
                        }
                    });
                });
            });
        ui.add_space(4.0);

        changed
    }

    // =========================================================================
    // TAB 4: PROFILES
    // =========================================================================
    fn ui_profiles_tab(&mut self, ui: &mut egui::Ui, cfg: &mut Config) -> bool {
        let mut changed = false;
        let lang = cfg.language;

        section_card(ui, lang.t("Сохранить текущий профиль", "Save current profile"), |ui| {
            ui.label(
                egui::RichText::new(lang.t(
                    "Сохраняет текущие настройки области, кратности и прицельной сетки.",
                    "Saves the current scope area, magnification, and reticle settings.",
                ))
                .size(12.0)
                .color(TEXT_SECONDARY),
            );
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                let input_w = (ui.available_width() - 110.0).max(160.0);
                let resp = ui.add(
                    egui::TextEdit::singleline(&mut self.new_profile_name)
                        .desired_width(input_w)
                        .hint_text(lang.t("Название профиля...", "Profile name...")),
                );
                let enter_pressed =
                    resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));

                if primary_button_with_icon(ui, lang.t("Сохранить", "Save"), None).clicked()
                    || enter_pressed
                {
                    let saved_name = cfg.save_user_profile(&self.new_profile_name);
                    self.new_profile_name.clear();
                    changed = true;
                    self.set_status(format!(
                        "{} «{}»",
                        lang.t("Профиль сохранён:", "Profile saved:"),
                        saved_name
                    ));
                }
            });
        });

        ui.add_space(8.0);

        section_card(ui, lang.t("Мои профили", "My profiles"), |ui| {
            if cfg.user_profiles.is_empty() {
                ui.label(
                    egui::RichText::new(lang.t(
                        "Сохранённых профилей пока нет. Введите название выше и нажмите «Сохранить».",
                        "No saved profiles yet. Enter a name above and click Save.",
                    ))
                    .size(12.5)
                    .color(TEXT_SECONDARY),
                );
            } else {
                let mut apply_idx: Option<usize> = None;
                let mut update_idx: Option<usize> = None;
                let mut delete_idx: Option<usize> = None;

                for (idx, prof) in cfg.user_profiles.iter().enumerate() {
                    let shape_label = match prof.shape.as_str() {
                        "square" => lang.t("Квадрат", "Square"),
                        "wide" => lang.t("Панорама 16:9", "Wide 16:9"),
                        "fullscreen" => lang.t("Весь экран", "Fullscreen"),
                        _ => lang.t("Круг", "Circle"),
                    };
                    let summary = if prof.crosshair_only || prof.zoom_factor <= 1.02 {
                        lang.t("Только прицел (1.0x)", "Crosshair only (1.0x)").to_string()
                    } else {
                        format!(
                            "{:.1}x · {} {} · {}",
                            prof.zoom_factor,
                            prof.scope_size,
                            lang.t("пкс", "px"),
                            shape_label
                        )
                    };

                    egui::Frame::new()
                        .fill(BG_PANEL)
                        .stroke(egui::Stroke::new(1.0, BORDER_SUBTLE))
                        .corner_radius(6.0)
                        .inner_margin(egui::Margin::symmetric(10, 8))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.vertical(|ui| {
                                    ui.label(
                                        egui::RichText::new(&prof.name)
                                            .size(13.5)
                                            .strong()
                                            .color(TEXT_PRIMARY),
                                    );
                                    ui.label(
                                        egui::RichText::new(summary)
                                            .size(12.0)
                                            .color(TEXT_SECONDARY),
                                    );
                                });

                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if destructive_button(ui, "", Some(UiIcon::Close))
                                            .on_hover_text(lang.t("Удалить профиль", "Delete profile"))
                                            .clicked()
                                        {
                                            delete_idx = Some(idx);
                                        }
                                        ui.add_space(4.0);
                                        if action_button(ui, lang.t("Обновить", "Update"), None)
                                            .on_hover_text(lang.t(
                                                "Перезаписать текущими настройками",
                                                "Overwrite with current settings",
                                            ))
                                            .clicked()
                                        {
                                            update_idx = Some(idx);
                                        }
                                        ui.add_space(4.0);
                                        if primary_button_with_icon(
                                            ui,
                                            lang.t("Применить", "Apply"),
                                            None,
                                        )
                                        .clicked()
                                        {
                                            apply_idx = Some(idx);
                                        }
                                    },
                                );
                            });
                        });
                    ui.add_space(6.0);
                }

                if let Some(idx) = apply_idx {
                    if let Some(name) = cfg.apply_user_profile(idx) {
                        self.sync_hex_buffers(cfg);
                        changed = true;
                        self.set_status(format!(
                            "{} «{}»",
                            lang.t("Применён профиль:", "Applied profile:"),
                            name
                        ));
                    }
                } else if let Some(idx) = update_idx {
                    if let Some(name) = cfg.user_profiles.get(idx).map(|p| p.name.clone()) {
                        cfg.save_user_profile(&name);
                        changed = true;
                        self.set_status(format!(
                            "{} «{}»",
                            lang.t("Профиль обновлён:", "Profile updated:"),
                            name
                        ));
                    }
                } else if let Some(idx) = delete_idx {
                    if let Some(name) = cfg.delete_user_profile(idx) {
                        changed = true;
                        self.set_status(format!(
                            "{} «{}»",
                            lang.t("Профиль удалён:", "Profile deleted:"),
                            name
                        ));
                    }
                }
            }
        });

        ui.add_space(8.0);

        section_card(ui, lang.t("Сброс конфигурации", "Reset configuration"), |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(lang.t(
                        "Вернуть все настройки оптики, прицела и клавиш к исходным значениям.",
                        "Restore all optics, reticle, and hotkey settings to their defaults.",
                    ))
                    .size(12.5)
                    .color(TEXT_SECONDARY),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if destructive_button(
                        ui,
                        lang.t("Сбросить все настройки", "Reset all settings"),
                        Some(UiIcon::Reset),
                    )
                    .clicked()
                    {
                        let saved_profiles = cfg.user_profiles.clone();
                        let saved_lang = cfg.language;
                        *cfg = Config::default();
                        cfg.user_profiles = saved_profiles;
                        cfg.language = saved_lang;
                        self.sync_hex_buffers(cfg);
                        changed = true;
                        self.set_status(lang.t(
                            "Все настройки сброшены к исходным значениям",
                            "All settings reset to defaults",
                        ));
                    }
                });
            });
        });

        changed
    }

    // =========================================================================
    // TAB 5: SYSTEM & TRAY
    // =========================================================================
    fn ui_system_tab(&mut self, ui: &mut egui::Ui, cfg: &mut Config) -> bool {
        let mut changed = false;
        let lang = cfg.language;

        section_card(ui, lang.t("Язык интерфейса", "Interface language"), |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(lang.t("Язык программы", "Application language"))
                        .size(12.5)
                        .color(TEXT_SECONDARY),
                );
                let mut cur_lang = cfg.language;
                if segmented_control(
                    ui,
                    &mut cur_lang,
                    &[(Language::Ru, "Русский"), (Language::En, "English")],
                ) {
                    cfg.language = cur_lang;
                    changed = true;
                }
            });
        });

        ui.add_space(8.0);

        section_card(ui, lang.t("Производительность и индикация", "Performance & HUD"), |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    egui::RichText::new(lang.t("Частота обновления оверлея", "Overlay refresh rate"))
                        .size(12.5)
                        .color(TEXT_SECONDARY),
                );
                let mut fps = cfg.fps_limit;
                let hz = lang.t("Гц", "Hz");
                let opts = [
                    (60u32, format!("60 {}", hz)),
                    (120u32, format!("120 {}", hz)),
                    (144u32, format!("144 {}", hz)),
                    (240u32, format!("240 {}", hz)),
                    (360u32, format!("360 {}", hz)),
                ];
                let opt_refs: Vec<(u32, &str)> = opts.iter().map(|(v, s)| (*v, s.as_str())).collect();
                if segmented_control(ui, &mut fps, &opt_refs) {
                    cfg.fps_limit = fps;
                    changed = true;
                }
            });

            ui.add_space(8.0);
            if toggle_switch(
                ui,
                &mut cfg.show_hud,
                lang.t("Показывать кратность и подсказки поверх экрана", "Show magnification & OSD hints on overlay"),
            )
            .changed()
            {
                changed = true;
            }

            ui.add_space(4.0);
            if toggle_switch(
                ui,
                &mut cfg.beep,
                lang.t("Звуковой сигнал при переключении", "Play beep on toggle"),
            )
            .changed()
            {
                changed = true;
            }
        });

        ui.add_space(8.0);

        section_card(ui, lang.t("Системный трей и автозапуск", "System tray & autostart"), |ui| {
            if toggle_switch(
                ui,
                &mut cfg.close_to_tray,
                lang.t("Сворачивать в трей при закрытии окна", "Minimize to tray when closing window"),
            )
            .changed()
            {
                changed = true;
            }

            ui.add_space(4.0);
            if toggle_switch(
                ui,
                &mut cfg.start_minimized,
                lang.t("Запускать свёрнутым в трей", "Start minimized in tray"),
            )
            .changed()
            {
                changed = true;
            }

            ui.add_space(4.0);
            let mut autostart = self.autostart_cached;
            if toggle_switch(
                ui,
                &mut autostart,
                lang.t("Запускать вместе с Windows", "Launch at Windows startup"),
            )
            .changed()
            {
                if set_autostart(autostart) {
                    self.autostart_cached = autostart;
                    self.set_status(if autostart {
                        lang.t("Автозапуск с Windows включён", "Windows startup enabled")
                    } else {
                        lang.t("Автозапуск с Windows отключён", "Windows startup disabled")
                    });
                }
            }

            ui.add_space(8.0);
            ui.label(
                egui::RichText::new(lang.t(
                    "ЛКМ по значку в трее открывает или скрывает окно. ПКМ открывает меню быстрого переключения.",
                    "Left-click the tray icon to show or hide the window. Right-click to open the quick menu.",
                ))
                .size(12.0)
                .color(TEXT_SECONDARY),
            );
        });

        changed
    }
}

// =============================================================================
// UNIFORM LINEAR VECTOR ICONS (1.5px stroke, single monochrome color)
// =============================================================================

fn draw_icon(painter: &egui::Painter, rect: egui::Rect, icon: UiIcon, color: egui::Color32) {
    let c = rect.center();
    let r = (rect.width().min(rect.height()) * 0.5).max(4.0);
    let stroke = egui::Stroke::new(1.5, color);

    match icon {
        UiIcon::Crosshair => {
            let rad = r * 0.72;
            painter.circle_stroke(c, rad, stroke);
            let inner = rad * 0.48;
            let outer = r * 0.96;
            painter.line_segment([c - egui::vec2(outer, 0.0), c - egui::vec2(inner, 0.0)], stroke);
            painter.line_segment([c + egui::vec2(inner, 0.0), c + egui::vec2(outer, 0.0)], stroke);
            painter.line_segment([c - egui::vec2(0.0, outer), c - egui::vec2(0.0, inner)], stroke);
            painter.line_segment([c + egui::vec2(0.0, inner), c + egui::vec2(0.0, outer)], stroke);
            painter.circle_filled(c, 1.3, color);
        }
        UiIcon::Zoom => {
            let glass_c = c - egui::vec2(r * 0.18, r * 0.18);
            let glass_r = r * 0.54;
            painter.circle_stroke(glass_c, glass_r, stroke);
            let dir = egui::vec2(0.7071, 0.7071);
            painter.line_segment(
                [
                    glass_c + dir * glass_r,
                    c + egui::vec2(r * 0.82, r * 0.82),
                ],
                stroke,
            );
        }
        UiIcon::Keyboard => {
            let kb = egui::Rect::from_center_size(c, egui::vec2(r * 1.85, r * 1.25));
            painter.rect_stroke(kb, 2.0, stroke, egui::StrokeKind::Inside);
            for dx in [-0.52, -0.17, 0.17, 0.52] {
                let x = c.x + dx * r;
                painter.line_segment(
                    [egui::pos2(x - 0.6, c.y - r * 0.24), egui::pos2(x + 0.6, c.y - r * 0.24)],
                    stroke,
                );
            }
            painter.line_segment(
                [
                    egui::pos2(c.x - r * 0.45, c.y + r * 0.22),
                    egui::pos2(c.x + r * 0.45, c.y + r * 0.22),
                ],
                stroke,
            );
        }
        UiIcon::Profiles => {
            for (i, knob_x) in [-0.35f32, 0.3, -0.1].iter().enumerate() {
                let y = c.y + (i as f32 - 1.0) * (r * 0.52);
                painter.line_segment(
                    [egui::pos2(c.x - r * 0.78, y), egui::pos2(c.x + r * 0.78, y)],
                    stroke,
                );
                let kx = c.x + knob_x * r;
                painter.circle_filled(egui::pos2(kx, y), 2.1, color);
            }
        }
        UiIcon::System => {
            painter.circle_stroke(c, r * 0.36, stroke);
            let outer = r * 0.85;
            let mid = r * 0.62;
            for i in 0..6 {
                let ang = (i as f32) * std::f32::consts::PI / 3.0;
                let dir = egui::vec2(ang.cos(), ang.sin());
                painter.line_segment([c + dir * mid, c + dir * outer], stroke);
            }
            painter.circle_stroke(c, mid, stroke);
        }
        UiIcon::Play => {
            let p1 = egui::pos2(c.x - r * 0.42, c.y - r * 0.62);
            let p2 = egui::pos2(c.x + r * 0.58, c.y);
            let p3 = egui::pos2(c.x - r * 0.42, c.y + r * 0.62);
            painter.line(vec![p1, p2, p3, p1], stroke);
        }
        UiIcon::Pause => {
            painter.line_segment(
                [
                    egui::pos2(c.x - r * 0.34, c.y - r * 0.58),
                    egui::pos2(c.x - r * 0.34, c.y + r * 0.58),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    egui::pos2(c.x + r * 0.34, c.y - r * 0.58),
                    egui::pos2(c.x + r * 0.34, c.y + r * 0.58),
                ],
                stroke,
            );
        }
        UiIcon::Eye => {
            let mut top = Vec::with_capacity(9);
            let mut bot = Vec::with_capacity(9);
            for i in 0..=8 {
                let t = (i as f32 / 8.0) * 2.0 - 1.0;
                let x = c.x + t * r * 0.88;
                let dy = (1.0 - t * t) * r * 0.52;
                top.push(egui::pos2(x, c.y - dy));
                bot.push(egui::pos2(x, c.y + dy));
            }
            painter.line(top, stroke);
            painter.line(bot, stroke);
            painter.circle_stroke(c, r * 0.28, stroke);
        }
        UiIcon::Tray => {
            let pts = vec![
                egui::pos2(c.x - r * 0.78, c.y + r * 0.15),
                egui::pos2(c.x - r * 0.78, c.y + r * 0.68),
                egui::pos2(c.x + r * 0.78, c.y + r * 0.68),
                egui::pos2(c.x + r * 0.78, c.y + r * 0.15),
            ];
            painter.line(pts, stroke);
            painter.line_segment(
                [
                    egui::pos2(c.x, c.y - r * 0.75),
                    egui::pos2(c.x, c.y + r * 0.32),
                ],
                stroke,
            );
            painter.line(
                vec![
                    egui::pos2(c.x - r * 0.36, c.y - r * 0.05),
                    egui::pos2(c.x, c.y + r * 0.32),
                    egui::pos2(c.x + r * 0.36, c.y - r * 0.05),
                ],
                stroke,
            );
        }
        UiIcon::Copy => {
            let back = egui::Rect::from_min_max(
                egui::pos2(c.x - r * 0.72, c.y - r * 0.72),
                egui::pos2(c.x + r * 0.25, c.y + r * 0.25),
            );
            let front = egui::Rect::from_min_max(
                egui::pos2(c.x - r * 0.25, c.y - r * 0.25),
                egui::pos2(c.x + r * 0.72, c.y + r * 0.72),
            );
            painter.rect_stroke(front, 1.5, stroke, egui::StrokeKind::Inside);
            painter.line(
                vec![
                    egui::pos2(back.left(), front.top()),
                    back.left_top(),
                    egui::pos2(front.left(), back.top()),
                ],
                stroke,
            );
        }
        UiIcon::Import => {
            let pts = vec![
                egui::pos2(c.x - r * 0.75, c.y + r * 0.2),
                egui::pos2(c.x - r * 0.75, c.y + r * 0.7),
                egui::pos2(c.x + r * 0.75, c.y + r * 0.7),
                egui::pos2(c.x + r * 0.75, c.y + r * 0.2),
            ];
            painter.line(pts, stroke);
            painter.line_segment(
                [
                    egui::pos2(c.x, c.y - r * 0.75),
                    egui::pos2(c.x, c.y + r * 0.3),
                ],
                stroke,
            );
            painter.line(
                vec![
                    egui::pos2(c.x - r * 0.35, c.y - r * 0.05),
                    egui::pos2(c.x, c.y + r * 0.3),
                    egui::pos2(c.x + r * 0.35, c.y - r * 0.05),
                ],
                stroke,
            );
        }
        UiIcon::Reset => {
            let arc_r = r * 0.68;
            let mut pts = Vec::with_capacity(14);
            for i in 0..=13 {
                let t = i as f32 / 13.0;
                let ang = -2.2 + t * 4.7;
                pts.push(c + egui::vec2(ang.cos() * arc_r, ang.sin() * arc_r));
            }
            painter.line(pts, stroke);
            let tip = c + egui::vec2((-2.2f32).cos() * arc_r, (-2.2f32).sin() * arc_r);
            painter.line(
                vec![
                    tip + egui::vec2(-0.5, -r * 0.42),
                    tip,
                    tip + egui::vec2(r * 0.42, -0.5),
                ],
                stroke,
            );
        }
        UiIcon::Close => {
            let d = r * 0.56;
            painter.line_segment(
                [egui::pos2(c.x - d, c.y - d), egui::pos2(c.x + d, c.y + d)],
                stroke,
            );
            painter.line_segment(
                [egui::pos2(c.x + d, c.y - d), egui::pos2(c.x - d, c.y + d)],
                stroke,
            );
        }
    }
}

// =============================================================================
// CUSTOM WIDGET PRIMITIVES (Distinct styles for Tabs, Buttons, Segmented, Toggles)
// =============================================================================

fn section_card(ui: &mut egui::Ui, title: &str, add_contents: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::new()
        .fill(BG_CARD)
        .stroke(egui::Stroke::new(1.0, BORDER_SUBTLE))
        .corner_radius(6.0)
        .inner_margin(egui::Margin::same(12))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(
                egui::RichText::new(title)
                    .size(14.0)
                    .strong()
                    .color(TEXT_PRIMARY),
            );
            ui.add_space(8.0);
            add_contents(ui);
        });
}

fn status_indicator(ui: &mut egui::Ui, dot_color: egui::Color32, label: &str) {
    let font = egui::FontId::proportional(12.5);
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_string(), font, TEXT_PRIMARY);
    let width = 10.0 + 6.0 + galley.size().x + 8.0;
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, 26.0), egui::Sense::hover());
    let dot_center = egui::pos2(rect.left() + 8.0, rect.center().y);
    ui.painter().circle_filled(dot_center, 4.0, dot_color);
    let text_pos = egui::pos2(
        rect.left() + 17.0,
        rect.center().y - galley.size().y * 0.5,
    );
    ui.painter().galley(text_pos, galley, TEXT_PRIMARY);
}

fn nav_tab_button(
    ui: &mut egui::Ui,
    active: bool,
    icon: UiIcon,
    label: &str,
) -> egui::Response {
    let font = egui::FontId::proportional(13.0);
    let text_col = if active { TEXT_PRIMARY } else { TEXT_SECONDARY };
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_string(), font, text_col);
    let pad_x = 9.0;
    let icon_w = 14.0;
    let gap = 5.0;
    let width = pad_x + icon_w + gap + galley.size().x + pad_x;
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(width, 32.0), egui::Sense::click());

    let bg = if active {
        BG_CONTROL
    } else if resp.hovered() {
        egui::Color32::from_rgb(30, 34, 41)
    } else {
        egui::Color32::TRANSPARENT
    };

    if bg != egui::Color32::TRANSPARENT {
        ui.painter().rect_filled(rect, 5.0, bg);
    }

    if active {
        let bar_rect = egui::Rect::from_min_max(
            egui::pos2(rect.left() + 6.0, rect.bottom() - 2.5),
            egui::pos2(rect.right() - 6.0, rect.bottom()),
        );
        ui.painter().rect_filled(bar_rect, 1.0, ACCENT_BRIGHT);
    }

    let icon_col = if active {
        ACCENT_BRIGHT
    } else if resp.hovered() {
        TEXT_PRIMARY
    } else {
        TEXT_SECONDARY
    };
    let icon_rect = egui::Rect::from_center_size(
        egui::pos2(rect.left() + pad_x + icon_w * 0.5, rect.center().y),
        egui::vec2(icon_w, icon_w),
    );
    draw_icon(ui.painter(), icon_rect, icon, icon_col);

    let fg = if active || resp.hovered() {
        TEXT_PRIMARY
    } else {
        TEXT_SECONDARY
    };
    let text_pos = egui::pos2(
        rect.left() + pad_x + icon_w + gap,
        rect.center().y - galley.size().y * 0.5,
    );
    ui.painter().galley(text_pos, galley, fg);

    resp
}

fn styled_button(
    ui: &mut egui::Ui,
    label: &str,
    icon: Option<UiIcon>,
    fill: egui::Color32,
    hover_fill: egui::Color32,
    border: egui::Color32,
    text_col: egui::Color32,
    min_width: Option<f32>,
) -> egui::Response {
    let font = egui::FontId::proportional(12.5);
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_string(), font, text_col);
    let pad_x = if label.is_empty() { 7.5 } else { 9.0 };
    let icon_w = if icon.is_some() { 13.0 } else { 0.0 };
    let gap = if icon.is_some() && !label.is_empty() {
        6.0
    } else {
        0.0
    };
    let content_w = icon_w + gap + galley.size().x;
    let width = (pad_x * 2.0 + content_w).max(min_width.unwrap_or(0.0));
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(width, 28.0), egui::Sense::click());

    let cur_fill = if resp.hovered() && ui.is_enabled() {
        hover_fill
    } else {
        fill
    };
    let cur_border = if resp.hovered() && ui.is_enabled() {
        if border == DANGER_BORDER {
            DANGER_TEXT
        } else {
            ACCENT_BRIGHT
        }
    } else {
        border
    };

    ui.painter().rect_filled(rect, 5.0, cur_fill);
    ui.painter().rect_stroke(
        rect,
        5.0,
        egui::Stroke::new(1.0, cur_border),
        egui::StrokeKind::Inside,
    );

    let start_x = rect.center().x - content_w * 0.5;
    if let Some(ic) = icon {
        let icon_rect = egui::Rect::from_center_size(
            egui::pos2(start_x + icon_w * 0.5, rect.center().y),
            egui::vec2(icon_w, icon_w),
        );
        draw_icon(ui.painter(), icon_rect, ic, text_col);
    }
    let text_pos = egui::pos2(
        start_x + icon_w + gap,
        rect.center().y - galley.size().y * 0.5,
    );
    ui.painter().galley(text_pos, galley, text_col);

    resp
}

fn action_button(ui: &mut egui::Ui, label: &str, icon: Option<UiIcon>) -> egui::Response {
    styled_button(
        ui,
        label,
        icon,
        BG_CONTROL,
        BG_CONTROL_HOVER,
        BORDER_CONTROL,
        TEXT_PRIMARY,
        None,
    )
}

fn primary_button_with_icon(
    ui: &mut egui::Ui,
    label: &str,
    icon: Option<UiIcon>,
) -> egui::Response {
    styled_button(
        ui,
        label,
        icon,
        ACCENT,
        ACCENT_HOVER,
        ACCENT_BRIGHT,
        egui::Color32::WHITE,
        None,
    )
}

fn destructive_button(ui: &mut egui::Ui, label: &str, icon: Option<UiIcon>) -> egui::Response {
    styled_button(
        ui,
        label,
        icon,
        DANGER_BG,
        DANGER_BG_HOVER,
        DANGER_BORDER,
        DANGER_TEXT,
        None,
    )
}

fn hotkey_bind_button(
    ui: &mut egui::Ui,
    label: &str,
    is_bound: bool,
    width: f32,
) -> egui::Response {
    let (fill, border, text) = if is_bound {
        (ACCENT_TINT, ACCENT_BRIGHT, TEXT_PRIMARY)
    } else {
        (BG_CONTROL, BORDER_CONTROL, TEXT_SECONDARY)
    };
    styled_button(
        ui,
        label,
        None,
        fill,
        BG_CONTROL_HOVER,
        border,
        text,
        Some(width),
    )
}

/// Unified segmented control for 1-of-N selection (distinct from buttons and switches)
fn segmented_control<T: PartialEq + Copy>(
    ui: &mut egui::Ui,
    value: &mut T,
    options: &[(T, &str)],
) -> bool {
    let mut changed = false;
    egui::Frame::new()
        .fill(BG_PANEL)
        .stroke(egui::Stroke::new(1.0, BORDER_SUBTLE))
        .corner_radius(6.0)
        .inner_margin(egui::Margin::same(3))
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(2.0, 2.0);
                for &(opt_val, label) in options {
                    let selected = *value == opt_val;
                    let font = egui::FontId::proportional(12.0);
                    let text_col = if selected {
                        egui::Color32::WHITE
                    } else {
                        TEXT_SECONDARY
                    };
                    let galley =
                        ui.painter()
                            .layout_no_wrap(label.to_string(), font, text_col);
                    let seg_w = galley.size().x + 14.0;
                    let (rect, resp) =
                        ui.allocate_exact_size(egui::vec2(seg_w, 22.0), egui::Sense::click());

                    let bg = if selected {
                        ACCENT
                    } else if resp.hovered() && ui.is_enabled() {
                        BG_CONTROL_HOVER
                    } else {
                        egui::Color32::TRANSPARENT
                    };
                    if bg != egui::Color32::TRANSPARENT {
                        ui.painter().rect_filled(rect, 4.0, bg);
                    }
                    let fg = if selected {
                        egui::Color32::WHITE
                    } else if resp.hovered() && ui.is_enabled() {
                        TEXT_PRIMARY
                    } else {
                        TEXT_SECONDARY
                    };
                    let text_pos = rect.center() - galley.size() * 0.5;
                    ui.painter().galley(text_pos, galley, fg);

                    if resp.clicked() && !selected {
                        *value = opt_val;
                        changed = true;
                    }
                }
            });
        });
    changed
}

/// Multi-row segmented grid for longer option names (e.g., 4 operating modes in 2x2)
fn segmented_control_grid<T: PartialEq + Copy>(
    ui: &mut egui::Ui,
    value: &mut T,
    options: &[(T, &str)],
    cols: usize,
) -> bool {
    let mut changed = false;
    egui::Frame::new()
        .fill(BG_PANEL)
        .stroke(egui::Stroke::new(1.0, BORDER_SUBTLE))
        .corner_radius(6.0)
        .inner_margin(egui::Margin::same(3))
        .show(ui, |ui| {
            let gap = 3.0;
            let cell_w = ((ui.available_width() - gap * (cols as f32 - 1.0)) / cols as f32)
                .floor()
                .max(120.0);

            for row in options.chunks(cols) {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = gap;
                    for &(opt_val, label) in row {
                        let selected = *value == opt_val;
                        let font = egui::FontId::proportional(12.5);
                        let text_col = if selected {
                            egui::Color32::WHITE
                        } else {
                            TEXT_SECONDARY
                        };
                        let galley =
                            ui.painter()
                                .layout_no_wrap(label.to_string(), font, text_col);
                        let (rect, resp) =
                            ui.allocate_exact_size(egui::vec2(cell_w, 28.0), egui::Sense::click());
                        let bg = if selected {
                            ACCENT
                        } else if resp.hovered() && ui.is_enabled() {
                            BG_CONTROL_HOVER
                        } else {
                            egui::Color32::TRANSPARENT
                        };
                        if bg != egui::Color32::TRANSPARENT {
                            ui.painter().rect_filled(rect, 4.0, bg);
                        }
                        let fg = if selected {
                            egui::Color32::WHITE
                        } else if resp.hovered() && ui.is_enabled() {
                            TEXT_PRIMARY
                        } else {
                            TEXT_SECONDARY
                        };
                        let text_pos = rect.center() - galley.size() * 0.5;
                        ui.painter().galley(text_pos, galley, fg);

                        if resp.clicked() && !selected {
                            *value = opt_val;
                            changed = true;
                        }
                    }
                });
                ui.add_space(gap);
            }
        });
    changed
}

/// Pill toggle switch for boolean options (clearly distinct from buttons and segments)
fn toggle_switch(ui: &mut egui::Ui, on: &mut bool, label: &str) -> egui::Response {
    let font = egui::FontId::proportional(13.0);
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_string(), font, TEXT_PRIMARY);

    let track_w: f32 = 34.0;
    let track_h: f32 = 18.0;
    let gap: f32 = 10.0;
    let total_w = track_w + gap + galley.size().x + 4.0;
    let total_h = track_h.max(galley.size().y + 4.0);

    let (rect, mut resp) = ui.allocate_exact_size(egui::vec2(total_w, total_h), egui::Sense::click());
    if resp.clicked() {
        *on = !*on;
        resp.mark_changed();
    }

    let track_rect = egui::Rect::from_center_size(
        egui::pos2(rect.left() + track_w * 0.5, rect.center().y),
        egui::vec2(track_w, track_h),
    );
    let radius = track_h * 0.5;

    let (track_fill, track_stroke, knob_col) = if *on {
        let f = if resp.hovered() && ui.is_enabled() {
            ACCENT_HOVER
        } else {
            ACCENT
        };
        (f, ACCENT_BRIGHT, egui::Color32::WHITE)
    } else {
        let b = if resp.hovered() && ui.is_enabled() {
            TEXT_SECONDARY
        } else {
            BORDER_CONTROL
        };
        (BG_PANEL, b, TEXT_PRIMARY)
    };

    ui.painter().rect_filled(track_rect, radius, track_fill);
    ui.painter().rect_stroke(
        track_rect,
        radius,
        egui::Stroke::new(1.2, track_stroke),
        egui::StrokeKind::Inside,
    );

    let knob_x = if *on {
        track_rect.right() - radius
    } else {
        track_rect.left() + radius
    };
    ui.painter()
        .circle_filled(egui::pos2(knob_x, track_rect.center().y), radius - 3.0, knob_col);

    let text_pos = egui::pos2(
        rect.left() + track_w + gap,
        rect.center().y - galley.size().y * 0.5,
    );
    ui.painter().galley(text_pos, galley, TEXT_PRIMARY);

    resp
}

/// Accent-filled checkbox for compact multi-option flags (e.g., ray directions)
fn check_option(ui: &mut egui::Ui, checked: &mut bool, label: &str) -> egui::Response {
    let font = egui::FontId::proportional(12.5);
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_string(), font, TEXT_PRIMARY);
    let box_sz = 16.0;
    let gap = 7.0;
    let total_w = box_sz + gap + galley.size().x + 8.0;
    let (rect, mut resp) = ui.allocate_exact_size(egui::vec2(total_w, 24.0), egui::Sense::click());
    if resp.clicked() {
        *checked = !*checked;
        resp.mark_changed();
    }

    let box_rect = egui::Rect::from_center_size(
        egui::pos2(rect.left() + box_sz * 0.5, rect.center().y),
        egui::vec2(box_sz, box_sz),
    );
    if *checked {
        ui.painter().rect_filled(box_rect, 4.0, ACCENT);
        ui.painter().rect_stroke(
            box_rect,
            4.0,
            egui::Stroke::new(1.0, ACCENT_BRIGHT),
            egui::StrokeKind::Inside,
        );
        let c = box_rect.center();
        let check_stroke = egui::Stroke::new(1.8, egui::Color32::WHITE);
        ui.painter().line(
            vec![
                egui::pos2(c.x - 4.0, c.y + 0.2),
                egui::pos2(c.x - 1.2, c.y + 3.2),
                egui::pos2(c.x + 4.3, c.y - 3.2),
            ],
            check_stroke,
        );
    } else {
        ui.painter().rect_filled(box_rect, 4.0, BG_PANEL);
        let border = if resp.hovered() && ui.is_enabled() {
            ACCENT_BRIGHT
        } else {
            BORDER_CONTROL
        };
        ui.painter().rect_stroke(
            box_rect,
            4.0,
            egui::Stroke::new(1.2, border),
            egui::StrokeKind::Inside,
        );
    }

    let text_pos = egui::pos2(
        rect.left() + box_sz + gap,
        rect.center().y - galley.size().y * 0.5,
    );
    ui.painter().galley(text_pos, galley, TEXT_PRIMARY);

    resp
}

fn labeled_slider(
    ui: &mut egui::Ui,
    label: &str,
    slider: egui::Slider<'_>,
    changed: &mut bool,
) {
    let label_w = if ui.available_width() < 320.0 {
        76.0
    } else {
        130.0
    };
    ui.horizontal(|ui| {
        ui.add_sized(
            [label_w, 20.0],
            egui::Label::new(
                egui::RichText::new(label)
                    .size(12.5)
                    .color(TEXT_PRIMARY),
            ),
        );
        if ui.add(slider).changed() {
            *changed = true;
        }
    });
}

fn color_swatch_row(
    ui: &mut egui::Ui,
    target_hex: &mut String,
    buf_hex: &mut String,
    lang: Language,
) -> bool {
    let mut changed = false;
    let presets = [
        ("#00FF00", lang.t("Зелёный", "Green")),
        ("#FF0000", lang.t("Красный", "Red")),
        ("#00FFFF", lang.t("Бирюзовый", "Cyan")),
        ("#FFFF00", lang.t("Жёлтый", "Yellow")),
        ("#FFFFFF", lang.t("Белый", "White")),
        ("#FF00FF", lang.t("Фиолетовый", "Magenta")),
        ("#FF8800", lang.t("Оранжевый", "Orange")),
        ("#FF0055", lang.t("Малиновый", "Crimson")),
    ];

    ui.horizontal_wrapped(|ui| {
        for (hex, tip) in presets {
            let (r, g, b) = Config::parse_hex_rgb(hex, (0, 255, 0));
            let col = egui::Color32::from_rgb(r, g, b);
            let active = target_hex.eq_ignore_ascii_case(hex);
            let (rect, resp) =
                ui.allocate_exact_size(egui::vec2(22.0, 22.0), egui::Sense::click());
            ui.painter().rect_filled(rect, 4.0, col);
            if active {
                ui.painter().rect_stroke(
                    rect.expand(1.5),
                    5.0,
                    egui::Stroke::new(2.0, egui::Color32::WHITE),
                    egui::StrokeKind::Outside,
                );
            } else {
                ui.painter().rect_stroke(
                    rect,
                    4.0,
                    egui::Stroke::new(1.0, BORDER_CONTROL),
                    egui::StrokeKind::Inside,
                );
            }
            if resp.on_hover_text(tip).clicked() {
                *target_hex = hex.to_string();
                *buf_hex = hex.to_string();
                changed = true;
            }
        }

        ui.add_space(4.0);
        ui.label(
            egui::RichText::new("HEX")
                .size(12.0)
                .color(TEXT_SECONDARY),
        );
        let resp = ui.add(egui::TextEdit::singleline(buf_hex).desired_width(70.0));
        if resp.changed() {
            *target_hex = buf_hex.clone();
            changed = true;
        }
    });

    changed
}

// =============================================================================
// 48x48 THUMBNAIL TILE FOR SHAPES & PRESETS
// =============================================================================

fn thumbnail_tile(
    ui: &mut egui::Ui,
    tile_w: f32,
    selected: bool,
    label: &str,
    preview_cfg: &Config,
) -> egui::Response {
    let tile_h = 80.0;
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(tile_w, tile_h), egui::Sense::click());

    let fill = if selected {
        ACCENT_TINT
    } else if resp.hovered() {
        BG_CONTROL_HOVER
    } else {
        BG_PANEL
    };
    let border = if selected {
        egui::Stroke::new(1.5, ACCENT_BRIGHT)
    } else if resp.hovered() {
        egui::Stroke::new(1.0, BORDER_CONTROL)
    } else {
        egui::Stroke::new(1.0, BORDER_SUBTLE)
    };

    ui.painter().rect_filled(rect, 6.0, fill);
    ui.painter()
        .rect_stroke(rect, 6.0, border, egui::StrokeKind::Inside);

    // 48x48 thumbnail preview area centered in upper portion of tile
    let thumb_rect = egui::Rect::from_center_size(
        egui::pos2(rect.center().x, rect.top() + 30.0),
        egui::vec2(48.0, 48.0),
    );
    ui.painter().rect_filled(thumb_rect, 4.0, BG_APP);
    ui.painter().rect_stroke(
        thumb_rect,
        4.0,
        egui::Stroke::new(1.0, BORDER_SUBTLE),
        egui::StrokeKind::Inside,
    );
    draw_reticle_thumbnail(&ui.painter_at(thumb_rect), thumb_rect, preview_cfg);

    let font = egui::FontId::proportional(12.0);
    let text_col = if selected { TEXT_PRIMARY } else { TEXT_SECONDARY };
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_string(), font, text_col);
    let fg = if selected || resp.hovered() {
        TEXT_PRIMARY
    } else {
        TEXT_SECONDARY
    };
    let text_pos = egui::pos2(
        rect.center().x - galley.size().x * 0.5,
        rect.bottom() - galley.size().y - 6.0,
    );
    ui.painter().galley(text_pos, galley, fg);

    resp
}

fn draw_reticle_thumbnail(painter: &egui::Painter, rect: egui::Rect, cfg: &Config) {
    let center = rect.center();
    let (pr, pg, pb) = cfg.parse_color_rgb();
    let (sr, sg, sb) = cfg.parse_secondary_color_rgb();
    let primary = egui::Color32::from_rgb(pr, pg, pb);
    let secondary = egui::Color32::from_rgb(sr, sg, sb);
    let outline = egui::Color32::from_rgb(0, 0, 0);
    let th = (cfg.crosshair_size as f32).clamp(1.5, 3.0);

    let seg = |p1: egui::Pos2, p2: egui::Pos2, w: f32, col: egui::Color32| {
        if cfg.xhair_outline {
            painter.line_segment([p1, p2], egui::Stroke::new(w + 1.5, outline));
        }
        painter.line_segment([p1, p2], egui::Stroke::new(w, col));
    };

    if cfg.xhair_show_circle {
        let r = 15.0;
        if cfg.xhair_outline {
            painter.circle_stroke(center, r, egui::Stroke::new(3.0, outline));
        }
        painter.circle_stroke(center, r, egui::Stroke::new(1.8, primary));
        if cfg.crosshair.eq_ignore_ascii_case("circle_dot") {
            let tick = 3.5;
            seg(center - egui::vec2(r + tick, 0.0), center - egui::vec2(r, 0.0), 1.6, primary);
            seg(center + egui::vec2(r, 0.0), center + egui::vec2(r + tick, 0.0), 1.6, primary);
            seg(center - egui::vec2(0.0, r + tick), center - egui::vec2(0.0, r), 1.6, primary);
            seg(center + egui::vec2(0.0, r), center + egui::vec2(0.0, r + tick), 1.6, primary);
        }
    }

    if cfg.xhair_corner_brackets {
        let d = 15.0;
        let b = 7.0;
        for (sx, sy) in [(-1.0f32, -1.0f32), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
            let p = center + egui::vec2(sx * d, sy * d);
            seg(p, p - egui::vec2(sx * b, 0.0), 1.8, primary);
            seg(p, p - egui::vec2(0.0, sy * b), 1.8, primary);
        }
    }

    let is_mildot = cfg.crosshair.eq_ignore_ascii_case("mildot");
    if cfg.crosshair.eq_ignore_ascii_case("chevron") {
        let cw = 8.0;
        seg(center, center + egui::vec2(-cw, cw), th, primary);
        seg(center, center + egui::vec2(cw, cw), th, primary);
        seg(center - egui::vec2(17.0, 0.0), center - egui::vec2(11.0, 0.0), 1.3, primary);
        seg(center + egui::vec2(11.0, 0.0), center + egui::vec2(17.0, 0.0), 1.3, primary);
        seg(center + egui::vec2(0.0, 10.0), center + egui::vec2(0.0, 19.0), 1.3, primary);
        seg(center + egui::vec2(-3.0, 14.5), center + egui::vec2(3.0, 14.5), 1.2, primary);
    } else if is_mildot {
        let span = 19.0;
        let g = 3.5;
        seg(center - egui::vec2(span, 0.0), center - egui::vec2(g, 0.0), 1.2, primary);
        seg(center + egui::vec2(g, 0.0), center + egui::vec2(span, 0.0), 1.2, primary);
        seg(center - egui::vec2(0.0, span), center - egui::vec2(0.0, g), 1.2, primary);
        seg(center + egui::vec2(0.0, g), center + egui::vec2(0.0, span), 1.2, primary);
        for d in [7.5, 13.0] {
            painter.circle_filled(center - egui::vec2(d, 0.0), 1.8, primary);
            painter.circle_filled(center + egui::vec2(d, 0.0), 1.8, primary);
            painter.circle_filled(center - egui::vec2(0.0, d), 1.8, primary);
            painter.circle_filled(center + egui::vec2(0.0, d), 1.8, primary);
        }
    } else if cfg.xhair_show_lines {
        let gap = (cfg.crosshair_gap as f32).clamp(2.5, 6.0);
        let len = ((cfg.crosshair_length as f32) * 0.85).clamp(6.0, 12.0);
        if cfg.xhair_left_line {
            seg(center - egui::vec2(gap + len, 0.0), center - egui::vec2(gap, 0.0), th, primary);
        }
        if cfg.xhair_right_line {
            seg(center + egui::vec2(gap, 0.0), center + egui::vec2(gap + len, 0.0), th, primary);
        }
        if cfg.xhair_top_line {
            seg(center - egui::vec2(0.0, gap + len), center - egui::vec2(0.0, gap), th, primary);
        }
        if cfg.xhair_bottom_line {
            seg(center + egui::vec2(0.0, gap), center + egui::vec2(0.0, gap + len), th, primary);
        }
        if cfg.xhair_diagonal {
            let d0 = 4.0;
            let d1 = 13.5;
            for (sx, sy) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
                seg(
                    center + egui::vec2(sx * d0, sy * d0),
                    center + egui::vec2(sx * d1, sy * d1),
                    th,
                    primary,
                );
            }
        }
    }

    if cfg.xhair_center_dot {
        let dr = (cfg.xhair_dot_size as f32).clamp(2.0, 3.2);
        if cfg.xhair_dot_shape.eq_ignore_ascii_case("square") {
            let sz = egui::vec2(dr * 2.0, dr * 2.0);
            if cfg.xhair_outline {
                painter.rect_filled(
                    egui::Rect::from_center_size(center, sz + egui::vec2(2.0, 2.0)),
                    0.0,
                    outline,
                );
            }
            painter.rect_filled(egui::Rect::from_center_size(center, sz), 0.0, secondary);
        } else {
            if cfg.xhair_outline {
                painter.circle_filled(center, dr + 1.0, outline);
            }
            painter.circle_filled(center, dr, secondary);
        }
    }
}

// =============================================================================
// LIVE PREVIEW RENDERERS (Distinct Backgrounds + Magnified Crosshair & Lens)
// =============================================================================

fn paint_preview_background(painter: &egui::Painter, rect: egui::Rect, bg: PreviewBg) {
    let (sky, horizon, ground, silhouette, grid_col) = match bg {
        PreviewBg::Night => (
            egui::Color32::from_rgb(13, 18, 30),
            egui::Color32::from_rgb(30, 40, 62),
            egui::Color32::from_rgb(16, 20, 29),
            egui::Color32::from_rgb(38, 48, 68),
            egui::Color32::from_rgba_unmultiplied(255, 255, 255, 16),
        ),
        PreviewBg::City => (
            egui::Color32::from_rgb(68, 84, 104),
            egui::Color32::from_rgb(108, 122, 138),
            egui::Color32::from_rgb(58, 63, 71),
            egui::Color32::from_rgb(44, 50, 60),
            egui::Color32::from_rgba_unmultiplied(255, 255, 255, 24),
        ),
        PreviewBg::Desert => (
            egui::Color32::from_rgb(142, 178, 214),
            egui::Color32::from_rgb(218, 188, 132),
            egui::Color32::from_rgb(186, 148, 92),
            egui::Color32::from_rgb(122, 92, 54),
            egui::Color32::from_rgba_unmultiplied(70, 50, 25, 32),
        ),
        PreviewBg::Forest => (
            egui::Color32::from_rgb(76, 112, 98),
            egui::Color32::from_rgb(46, 82, 56),
            egui::Color32::from_rgb(32, 58, 36),
            egui::Color32::from_rgb(22, 42, 26),
            egui::Color32::from_rgba_unmultiplied(180, 230, 170, 22),
        ),
        PreviewBg::Snow => (
            egui::Color32::from_rgb(186, 206, 226),
            egui::Color32::from_rgb(232, 238, 245),
            egui::Color32::from_rgb(214, 224, 235),
            egui::Color32::from_rgb(96, 114, 136),
            egui::Color32::from_rgba_unmultiplied(60, 80, 110, 28),
        ),
    };

    painter.rect_filled(rect, 6.0, sky);
    let mid_y = rect.center().y + 18.0;
    let ground_rect = egui::Rect::from_min_max(egui::pos2(rect.left(), mid_y), rect.max);
    painter.rect_filled(ground_rect, 6.0, ground);

    // Distant horizon silhouettes matching scene
    let w = rect.width();
    for (x_frac, bw, bh) in [
        (0.10, 26.0, 38.0),
        (0.24, 34.0, 52.0),
        (0.72, 28.0, 44.0),
        (0.86, 36.0, 32.0),
    ] {
        let bx = rect.left() + w * x_frac;
        let b_rect = egui::Rect::from_min_max(
            egui::pos2(bx, mid_y - bh),
            egui::pos2(bx + bw, mid_y),
        );
        painter.rect_filled(b_rect, 1.0, silhouette);
    }

    painter.line_segment(
        [egui::pos2(rect.left(), mid_y), egui::pos2(rect.right(), mid_y)],
        egui::Stroke::new(2.0, horizon),
    );

    let c = rect.center();
    // Target silhouette centered behind crosshair to verify visibility
    painter.circle_filled(c - egui::vec2(0.0, 14.0), 10.0, silhouette);
    painter.rect_filled(
        egui::Rect::from_center_size(c + egui::vec2(0.0, 16.0), egui::vec2(26.0, 36.0)),
        4.0,
        silhouette,
    );

    // Calibration grid
    for step in [-60.0, -30.0, 0.0, 30.0, 60.0] {
        painter.line_segment(
            [egui::pos2(rect.left(), c.y + step), egui::pos2(rect.right(), c.y + step)],
            egui::Stroke::new(1.0, grid_col),
        );
        painter.line_segment(
            [egui::pos2(c.x + step, rect.top()), egui::pos2(c.x + step, rect.bottom())],
            egui::Stroke::new(1.0, grid_col),
        );
    }

    painter.rect_stroke(
        rect,
        6.0,
        egui::Stroke::new(1.0, BORDER_SUBTLE),
        egui::StrokeKind::Inside,
    );
}

fn draw_advanced_preview_canvas(
    ui: &egui::Ui,
    rect: egui::Rect,
    cfg: &Config,
    bg: PreviewBg,
    preview_zoom: f32,
) {
    let painter = ui.painter_at(rect);
    paint_preview_background(&painter, rect, bg);
    draw_crosshair_layers(&painter, rect.center(), cfg, preview_zoom);
}

fn draw_crosshair_layers(
    painter: &egui::Painter,
    center: egui::Pos2,
    cfg: &Config,
    preview_zoom: f32,
) {
    let scale = if cfg.xhair_scale_with_zoom {
        preview_zoom * cfg.zoom_factor.clamp(1.0, 2.0)
    } else {
        preview_zoom
    };

    let (pr, pg, pb) = cfg.parse_color_rgb();
    let (sr, sg, sb) = cfg.parse_secondary_color_rgb();
    let (or, og, ob) = cfg.parse_outline_color_rgb();

    let primary = egui::Color32::from_rgb(pr, pg, pb);
    let secondary = egui::Color32::from_rgb(sr, sg, sb);
    let outline = egui::Color32::from_rgb(or, og, ob);

    let use_out = cfg.xhair_outline;
    let out_th = (cfg.xhair_outline_thickness as f32).clamp(1.0, 4.0) * preview_zoom.min(1.8);
    let th = ((cfg.crosshair_size as f32) * scale).max(1.0);

    let draw_seg = |p1: egui::Pos2, p2: egui::Pos2, w: f32, col: egui::Color32| {
        if use_out {
            painter.line_segment([p1, p2], egui::Stroke::new(w + out_th * 2.0, outline));
        }
        painter.line_segment([p1, p2], egui::Stroke::new(w, col));
    };

    // 1. Outer Collimator Circle
    if cfg.xhair_show_circle {
        let r = ((cfg.xhair_circle_radius as f32) * scale).clamp(4.0, 102.0);
        let c_th = ((cfg.xhair_circle_thickness as f32) * scale).max(1.0);
        if use_out {
            painter.circle_stroke(center, r, egui::Stroke::new(c_th + out_th * 2.0, outline));
        }
        painter.circle_stroke(center, r, egui::Stroke::new(c_th, primary));

        if cfg.crosshair.eq_ignore_ascii_case("circle_dot") {
            let tick = 5.0 * scale;
            draw_seg(center - egui::vec2(r + tick, 0.0), center - egui::vec2(r, 0.0), c_th, primary);
            draw_seg(center + egui::vec2(r, 0.0), center + egui::vec2(r + tick, 0.0), c_th, primary);
            draw_seg(center - egui::vec2(0.0, r + tick), center - egui::vec2(0.0, r), c_th, primary);
            draw_seg(center + egui::vec2(0.0, r), center + egui::vec2(0.0, r + tick), c_th, primary);
        }
    }

    // 2. Corner Brackets [ ]
    if cfg.xhair_corner_brackets {
        let dist = ((cfg.xhair_bracket_dist as f32) * scale).clamp(8.0, 102.0);
        let bsize = ((cfg.xhair_bracket_size as f32) * scale).clamp(4.0, dist);
        let corners = [(-1.0f32, -1.0f32), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)];
        if use_out {
            for (sx, sy) in corners {
                let p = center + egui::vec2(sx * dist, sy * dist);
                painter.line_segment(
                    [p, p - egui::vec2(sx * bsize, 0.0)],
                    egui::Stroke::new(th + out_th * 2.0, outline),
                );
                painter.line_segment(
                    [p, p - egui::vec2(0.0, sy * bsize)],
                    egui::Stroke::new(th + out_th * 2.0, outline),
                );
            }
        }
        for (sx, sy) in corners {
            let p = center + egui::vec2(sx * dist, sy * dist);
            painter.line_segment([p, p - egui::vec2(sx * bsize, 0.0)], egui::Stroke::new(th, primary));
            painter.line_segment([p, p - egui::vec2(0.0, sy * bsize)], egui::Stroke::new(th, primary));
        }
    }

    // 3. Chevron / Mil-Dot
    let is_mildot = cfg.crosshair.eq_ignore_ascii_case("mildot");
    if cfg.crosshair.eq_ignore_ascii_case("chevron") {
        let cw = ((cfg.crosshair_length.clamp(6, 28) as f32) * scale).min(46.0);
        draw_seg(center, center + egui::vec2(-cw, cw), th, primary);
        draw_seg(center, center + egui::vec2(cw, cw), th, primary);
        let wg = cw + 10.0 * scale;
        let wl = 18.0 * scale;
        draw_seg(center - egui::vec2(wg + wl, 0.0), center - egui::vec2(wg, 0.0), 1.4, primary);
        draw_seg(center + egui::vec2(wg, 0.0), center + egui::vec2(wg + wl, 0.0), 1.4, primary);
        draw_seg(
            center + egui::vec2(0.0, cw + 6.0 * scale),
            center + egui::vec2(0.0, cw + 40.0 * scale),
            1.4,
            primary,
        );
    } else if is_mildot {
        let span = (68.0 * scale.min(1.6)).min(110.0);
        let g = ((cfg.crosshair_gap.max(3) as f32) * scale).min(28.0);
        if cfg.xhair_left_line {
            draw_seg(center - egui::vec2(span, 0.0), center - egui::vec2(g, 0.0), 1.4, primary);
        }
        if cfg.xhair_right_line {
            draw_seg(center + egui::vec2(g, 0.0), center + egui::vec2(span, 0.0), 1.4, primary);
        }
        if cfg.xhair_top_line {
            draw_seg(center - egui::vec2(0.0, span), center - egui::vec2(0.0, g), 1.4, primary);
        }
        if cfg.xhair_bottom_line {
            draw_seg(center + egui::vec2(0.0, g), center + egui::vec2(0.0, span), 1.4, primary);
        }
        let dot_step = 14.0 * scale.min(1.6);
        let mil_r = (2.2 * scale.min(1.6)).max(2.0);
        for idx in 1..=3 {
            let d = (idx as f32) * dot_step;
            if cfg.xhair_left_line {
                painter.circle_filled(center - egui::vec2(d, 0.0), mil_r, primary);
            }
            if cfg.xhair_right_line {
                painter.circle_filled(center + egui::vec2(d, 0.0), mil_r, primary);
            }
            if cfg.xhair_top_line {
                painter.circle_filled(center - egui::vec2(0.0, d), mil_r, primary);
            }
            if cfg.xhair_bottom_line {
                painter.circle_filled(center + egui::vec2(0.0, d), mil_r, primary);
            }
        }
    }

    // 4. Orthogonal & Diagonal Lines
    let gap = (cfg.crosshair_gap as f32) * scale;
    let h_len = ((cfg.crosshair_length as f32) * scale).clamp(2.0, 110.0);
    let v_len = if cfg.xhair_unlink_axis {
        ((cfg.xhair_v_length as f32) * scale).clamp(2.0, 110.0)
    } else {
        h_len
    };

    if cfg.xhair_show_lines {
        if !is_mildot {
            let mut segs = Vec::new();
            if cfg.xhair_left_line {
                segs.push((center - egui::vec2(gap + h_len, 0.0), center - egui::vec2(gap, 0.0)));
            }
            if cfg.xhair_right_line {
                segs.push((center + egui::vec2(gap, 0.0), center + egui::vec2(gap + h_len, 0.0)));
            }
            if cfg.xhair_top_line {
                segs.push((center - egui::vec2(0.0, gap + v_len), center - egui::vec2(0.0, gap)));
            }
            if cfg.xhair_bottom_line {
                segs.push((center + egui::vec2(0.0, gap), center + egui::vec2(0.0, gap + v_len)));
            }
            if use_out {
                for &(a, b) in &segs {
                    painter.line_segment([a, b], egui::Stroke::new(th + out_th * 2.0, outline));
                }
            }
            for &(a, b) in &segs {
                painter.line_segment([a, b], egui::Stroke::new(th, primary));
            }
        }

        if cfg.xhair_diagonal {
            let d0 = gap.max(0.0) * 0.7071;
            let d1 = (gap.max(0.0) + h_len) * 0.7071;
            for (sx, sy) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
                draw_seg(
                    center + egui::vec2(sx * d0, sy * d0),
                    center + egui::vec2(sx * d1, sy * d1),
                    th,
                    primary,
                );
            }
        }
    }

    // 5. Ballistic Ticks
    if cfg.xhair_ticks {
        let count = cfg.xhair_tick_count.clamp(1, 6);
        let step = (cfg.xhair_tick_step as f32) * scale;
        let base_gap = gap.max(4.0);
        let has_dir = cfg.xhair_show_lines
            && (cfg.xhair_top_line
                || cfg.xhair_bottom_line
                || cfg.xhair_left_line
                || cfg.xhair_right_line);
        for i in 1..=count {
            let d = base_gap + (i as f32) * step;
            let rw = ((6 - i).max(3) as f32) * scale;
            if cfg.xhair_bottom_line || !has_dir {
                draw_seg(
                    center + egui::vec2(-rw, d),
                    center + egui::vec2(rw, d),
                    (th * 0.5).max(1.0),
                    primary,
                );
            }
            if cfg.xhair_top_line && has_dir {
                draw_seg(
                    center + egui::vec2(-rw, -d),
                    center + egui::vec2(rw, -d),
                    (th * 0.5).max(1.0),
                    primary,
                );
            }
            if cfg.xhair_left_line || !has_dir {
                draw_seg(
                    center + egui::vec2(-d, -rw),
                    center + egui::vec2(-d, rw),
                    (th * 0.5).max(1.0),
                    primary,
                );
            }
            if cfg.xhair_right_line || !has_dir {
                draw_seg(
                    center + egui::vec2(d, -rw),
                    center + egui::vec2(d, rw),
                    (th * 0.5).max(1.0),
                    primary,
                );
            }
        }
    }

    // 6. Center Dot
    if cfg.xhair_center_dot {
        let dot_r = ((cfg.xhair_dot_size as f32) * scale).max(1.8);
        if cfg.xhair_dot_shape.eq_ignore_ascii_case("square") {
            let sz = egui::vec2(dot_r * 2.0, dot_r * 2.0);
            if use_out {
                painter.rect_filled(
                    egui::Rect::from_center_size(
                        center,
                        sz + egui::vec2(out_th * 2.0, out_th * 2.0),
                    ),
                    0.0,
                    outline,
                );
            }
            painter.rect_filled(egui::Rect::from_center_size(center, sz), 0.0, secondary);
        } else {
            if use_out {
                painter.circle_filled(center, dot_r + out_th, outline);
            }
            painter.circle_filled(center, dot_r, secondary);
        }
    }
}

fn draw_lens_preview_canvas(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    cfg: &Config,
    bg: PreviewBg,
) {
    let painter = ui.painter_at(rect);
    paint_preview_background(&painter, rect, bg);

    let center = rect.center();
    let max_h = (rect.height() * 0.42).min(90.0);
    let size_norm = ((cfg.scope_size as f32 - 150.0) / 1450.0).clamp(0.0, 1.0);
    let preview_h = (36.0 + size_norm * (max_h - 36.0)).min(max_h);
    let preview_w = if cfg.shape == "wide" || cfg.shape == "fullscreen" {
        (preview_h * 16.0 / 9.0).min(rect.width() * 0.46)
    } else {
        preview_h
    };

    let (r, g, b) = cfg.parse_color_rgb();
    let reticle_color = egui::Color32::from_rgb(r, g, b);

    if !cfg.is_no_zoom_mode() {
        let border_color = match cfg.border_style.as_str() {
            "neon" => reticle_color,
            "thin" => egui::Color32::from_rgb(95, 105, 120),
            "none" => egui::Color32::TRANSPARENT,
            _ => egui::Color32::from_rgb(56, 64, 80),
        };

        // Subtle magnified target inside the lens to illustrate zoom factor
        let z_scale = cfg.zoom_factor.clamp(1.2, 3.0);
        let lens_tint = egui::Color32::from_rgba_unmultiplied(15, 22, 34, 95);

        if cfg.shape == "circle" {
            painter.circle_filled(center, preview_h, lens_tint);
            painter.circle_filled(
                center - egui::vec2(0.0, 6.0 * z_scale),
                5.0 * z_scale,
                egui::Color32::from_rgba_unmultiplied(255, 255, 255, 28),
            );
            if cfg.border_style != "none" {
                painter.circle_stroke(center, preview_h, egui::Stroke::new(2.5, border_color));
            }
        } else {
            let lens_rect = egui::Rect::from_center_size(
                center,
                egui::vec2(preview_w * 2.0, preview_h * 2.0),
            );
            painter.rect_filled(lens_rect, 8.0, lens_tint);
            painter.circle_filled(
                center - egui::vec2(0.0, 6.0 * z_scale),
                5.0 * z_scale,
                egui::Color32::from_rgba_unmultiplied(255, 255, 255, 28),
            );
            if cfg.border_style != "none" {
                painter.rect_stroke(
                    lens_rect,
                    8.0,
                    egui::Stroke::new(2.5, border_color),
                    egui::StrokeKind::Inside,
                );
            }
        }
    }

    if !cfg.hide_crosshair_on_zoom || cfg.is_no_zoom_mode() {
        draw_crosshair_layers(&painter, center, cfg, 1.35);
    }

    if cfg.show_hud && !cfg.is_no_zoom_mode() {
        painter.text(
            center + egui::vec2(0.0, preview_h - 12.0),
            egui::Align2::CENTER_CENTER,
            format!("{:.1}x", cfg.zoom_factor),
            egui::FontId::monospace(11.5),
            reticle_color,
        );
    }
}
