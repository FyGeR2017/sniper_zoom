#![windows_subsystem = "windows"]

#[path = "../icon.rs"]
mod icon;

use eframe::egui;
use icon::{create_egui_icon, generate_ico_file_bytes};
use std::fs;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

const CREATE_NO_WINDOW: u32 = 0x08000000;

/// Embedded release executable of Sniper Zoom
const APP_EXE_BYTES: &[u8] = include_bytes!("../../target/release/sniper_zoom.exe");

// Flat neutral dark palette matching Sniper Zoom main window
const BG_APP: egui::Color32 = egui::Color32::from_rgb(19, 21, 25); // #131519
const BG_PANEL: egui::Color32 = egui::Color32::from_rgb(22, 25, 30); // #16191E
const BG_CARD: egui::Color32 = egui::Color32::from_rgb(27, 30, 36); // #1B1E24
const BG_CONTROL: egui::Color32 = egui::Color32::from_rgb(35, 39, 47); // #23272F
const BG_CONTROL_HOVER: egui::Color32 = egui::Color32::from_rgb(46, 52, 63);
const BORDER_SUBTLE: egui::Color32 = egui::Color32::from_rgb(46, 52, 64); // #2E3440
const BORDER_CONTROL: egui::Color32 = egui::Color32::from_rgb(68, 76, 92);

const ACCENT: egui::Color32 = egui::Color32::from_rgb(33, 94, 224);
const ACCENT_HOVER: egui::Color32 = egui::Color32::from_rgb(49, 110, 238);
const ACCENT_BRIGHT: egui::Color32 = egui::Color32::from_rgb(76, 138, 255);

const TEXT_PRIMARY: egui::Color32 = egui::Color32::from_rgb(243, 244, 246);
const TEXT_SECONDARY: egui::Color32 = egui::Color32::from_rgb(166, 176, 192);

const DANGER_BG: egui::Color32 = egui::Color32::from_rgb(56, 24, 28);
const DANGER_BG_HOVER: egui::Color32 = egui::Color32::from_rgb(76, 31, 36);
const DANGER_BORDER: egui::Color32 = egui::Color32::from_rgb(185, 68, 68);
const DANGER_TEXT: egui::Color32 = egui::Color32::from_rgb(248, 148, 148);

const LOG_BG: egui::Color32 = egui::Color32::from_rgb(22, 40, 33);
const LOG_BORDER: egui::Color32 = egui::Color32::from_rgb(46, 140, 96);
const LOG_TEXT: egui::Color32 = egui::Color32::from_rgb(134, 239, 188);

#[derive(Clone, Copy, PartialEq, Eq)]
enum Lang {
    Ru,
    En,
}

#[link(name = "user32")]
unsafe extern "system" {
    fn SetProcessDpiAwarenessContext(value: isize) -> i32;
    fn SetProcessDPIAware() -> i32;
}

fn set_dpi_aware() {
    unsafe {
        if SetProcessDpiAwarenessContext(-4) == 0 {
            SetProcessDPIAware();
        }
    }
}

struct SetupApp {
    lang: Lang,
    install_dir: String,
    create_desktop_shortcut: bool,
    create_start_menu_shortcut: bool,
    enable_autostart: bool,
    launch_after_install: bool,
    install_finished: bool,
    status_log: Vec<String>,
    error_msg: Option<String>,
}

impl SetupApp {
    fn new() -> Self {
        let default_dir = std::env::var("LOCALAPPDATA")
            .map(|p| Path::new(&p).join("Programs").join("SniperZoom"))
            .unwrap_or_else(|_| PathBuf::from("C:\\SniperZoom"));

        Self {
            lang: Lang::Ru,
            install_dir: default_dir.to_string_lossy().to_string(),
            create_desktop_shortcut: true,
            create_start_menu_shortcut: true,
            enable_autostart: true,
            launch_after_install: true,
            install_finished: false,
            status_log: Vec::new(),
            error_msg: None,
        }
    }

    fn t(&self, ru: &'static str, en: &'static str) -> &'static str {
        match self.lang {
            Lang::Ru => ru,
            Lang::En => en,
        }
    }

    fn perform_install(&mut self) {
        self.status_log.clear();
        self.error_msg = None;

        let target_dir = PathBuf::from(self.install_dir.trim());

        // 1. Stop any running instance of sniper_zoom.exe so file isn't locked
        let _ = Command::new("taskkill")
            .args(["/F", "/IM", "sniper_zoom.exe"])
            .creation_flags(CREATE_NO_WINDOW)
            .output();

        std::thread::sleep(std::time::Duration::from_millis(200));

        // 2. Create target directory
        if let Err(e) = fs::create_dir_all(&target_dir) {
            self.error_msg = Some(match self.lang {
                Lang::Ru => format!("Не удалось создать папку установки: {}", e),
                Lang::En => format!("Failed to create install folder: {}", e),
            });
            return;
        }
        self.status_log.push(match self.lang {
            Lang::Ru => format!("Создана папка: {}", target_dir.display()),
            Lang::En => format!("Created folder: {}", target_dir.display()),
        });

        // 3. Write sniper_zoom.exe
        let exe_path = target_dir.join("sniper_zoom.exe");
        if let Err(e) = fs::write(&exe_path, APP_EXE_BYTES) {
            self.error_msg = Some(match self.lang {
                Lang::Ru => format!("Ошибка записи sniper_zoom.exe: {}", e),
                Lang::En => format!("Failed to write sniper_zoom.exe: {}", e),
            });
            return;
        }
        self.status_log.push(
            self.t(
                "Установлен файл: sniper_zoom.exe",
                "Installed executable: sniper_zoom.exe",
            )
            .to_string(),
        );

        // 4. Write custom icon file sniper_zoom.ico
        let ico_path = target_dir.join("sniper_zoom.ico");
        let ico_bytes = generate_ico_file_bytes();
        let _ = fs::write(&ico_path, ico_bytes);
        self.status_log.push(
            self.t(
                "Установлена иконка: sniper_zoom.ico",
                "Installed icon: sniper_zoom.ico",
            )
            .to_string(),
        );

        // 5. Create uninstaller script Uninstall.cmd
        let uninstaller_path = target_dir.join("Uninstall.cmd");
        let uninstall_script = format!(
            "@echo off\r\n\
             taskkill /F /IM sniper_zoom.exe >nul 2>&1\r\n\
             reg delete \"HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run\" /v \"SniperZoom\" /f >nul 2>&1\r\n\
             reg delete \"HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\SniperZoom\" /f >nul 2>&1\r\n\
             del /F /Q \"%USERPROFILE%\\Desktop\\Sniper Zoom.lnk\" >nul 2>&1\r\n\
             del /F /Q \"%APPDATA%\\Microsoft\\Windows\\Start Menu\\Programs\\Sniper Zoom.lnk\" >nul 2>&1\r\n\
             cd /d \"%LOCALAPPDATA%\"\r\n\
             rmdir /S /Q \"{}\"\r\n",
            target_dir.display()
        );
        let _ = fs::write(&uninstaller_path, uninstall_script);

        // 6. Create Desktop & Start Menu shortcuts
        if self.create_desktop_shortcut {
            if let Ok(userprofile) = std::env::var("USERPROFILE") {
                let desktop_lnk = Path::new(&userprofile)
                    .join("Desktop")
                    .join("Sniper Zoom.lnk");
                create_windows_shortcut(&desktop_lnk, &exe_path, &target_dir, &ico_path);
                self.status_log.push(
                    self.t(
                        "Создан ярлык на рабочем столе",
                        "Created Desktop shortcut",
                    )
                    .to_string(),
                );
            }
        }

        if self.create_start_menu_shortcut {
            if let Ok(appdata) = std::env::var("APPDATA") {
                let start_lnk = Path::new(&appdata)
                    .join("Microsoft\\Windows\\Start Menu\\Programs")
                    .join("Sniper Zoom.lnk");
                create_windows_shortcut(&start_lnk, &exe_path, &target_dir, &ico_path);
                self.status_log.push(
                    self.t(
                        "Создан ярлык в меню «Пуск»",
                        "Created Start Menu shortcut",
                    )
                    .to_string(),
                );
            }
        }

        // 7. Configure Windows Autostart
        if self.enable_autostart {
            let run_val = format!("\"{}\" --minimized", exe_path.display());
            let _ = Command::new("reg")
                .args([
                    "add",
                    "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run",
                    "/v",
                    "SniperZoom",
                    "/t",
                    "REG_SZ",
                    "/d",
                    &run_val,
                    "/f",
                ])
                .creation_flags(CREATE_NO_WINDOW)
                .output();
            self.status_log.push(
                self.t(
                    "Включён автозапуск с Windows",
                    "Enabled Windows startup launch",
                )
                .to_string(),
            );
        } else {
            let _ = Command::new("reg")
                .args([
                    "delete",
                    "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run",
                    "/v",
                    "SniperZoom",
                    "/f",
                ])
                .creation_flags(CREATE_NO_WINDOW)
                .output();
        }

        // 8. Register in Windows "Apps & Features"
        let uninstall_key =
            "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\SniperZoom";
        for (val_name, val_data) in [
            ("DisplayName", "Sniper Zoom 2.0".to_string()),
            ("DisplayVersion", "2.0.0".to_string()),
            ("Publisher", "Sniper Zoom Project".to_string()),
            ("DisplayIcon", ico_path.to_string_lossy().to_string()),
            ("InstallLocation", target_dir.to_string_lossy().to_string()),
            (
                "UninstallString",
                format!("cmd.exe /c \"{}\"", uninstaller_path.display()),
            ),
        ] {
            let _ = Command::new("reg")
                .args([
                    "add",
                    uninstall_key,
                    "/v",
                    val_name,
                    "/t",
                    "REG_SZ",
                    "/d",
                    &val_data,
                    "/f",
                ])
                .creation_flags(CREATE_NO_WINDOW)
                .output();
        }
        self.status_log.push(
            self.t(
                "Программа зарегистрирована в системе",
                "Registered in Windows Apps & Features",
            )
            .to_string(),
        );

        // 9. Launch installed application if requested
        if self.launch_after_install {
            let _ = Command::new(&exe_path)
                .current_dir(&target_dir)
                .creation_flags(CREATE_NO_WINDOW)
                .spawn();
            self.status_log.push(
                self.t(
                    "Sniper Zoom запущен (значок в системном трее)",
                    "Sniper Zoom launched (check system tray)",
                )
                .to_string(),
            );
        }

        self.install_finished = true;
    }

    fn perform_uninstall(&mut self) {
        self.status_log.clear();
        self.error_msg = None;

        let target_dir = PathBuf::from(self.install_dir.trim());

        let _ = Command::new("taskkill")
            .args(["/F", "/IM", "sniper_zoom.exe"])
            .creation_flags(CREATE_NO_WINDOW)
            .output();

        std::thread::sleep(std::time::Duration::from_millis(200));

        let _ = Command::new("reg")
            .args([
                "delete",
                "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run",
                "/v",
                "SniperZoom",
                "/f",
            ])
            .creation_flags(CREATE_NO_WINDOW)
            .output();

        let _ = Command::new("reg")
            .args([
                "delete",
                "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\SniperZoom",
                "/f",
            ])
            .creation_flags(CREATE_NO_WINDOW)
            .output();

        if let Ok(userprofile) = std::env::var("USERPROFILE") {
            let _ = fs::remove_file(
                Path::new(&userprofile)
                    .join("Desktop")
                    .join("Sniper Zoom.lnk"),
            );
        }
        if let Ok(appdata) = std::env::var("APPDATA") {
            let _ = fs::remove_file(
                Path::new(&appdata)
                    .join("Microsoft\\Windows\\Start Menu\\Programs")
                    .join("Sniper Zoom.lnk"),
            );
        }

        if target_dir.exists() {
            let _ = fs::remove_dir_all(&target_dir);
        }

        self.install_finished = false;
        self.status_log.push(
            self.t(
                "Sniper Zoom и все ярлыки удалены",
                "Sniper Zoom and all shortcuts removed",
            )
            .to_string(),
        );
    }
}

fn create_windows_shortcut(
    lnk_path: &Path,
    target_exe: &Path,
    work_dir: &Path,
    icon_path: &Path,
) {
    let ps_cmd = format!(
        "$ws = New-Object -ComObject WScript.Shell; \
         $s = $ws.CreateShortcut('{}'); \
         $s.TargetPath = '{}'; \
         $s.WorkingDirectory = '{}'; \
         $s.IconLocation = '{}'; \
         $s.Description = 'Sniper Zoom'; \
         $s.Save()",
        lnk_path.display().to_string().replace('\'', "''"),
        target_exe.display().to_string().replace('\'', "''"),
        work_dir.display().to_string().replace('\'', "''"),
        icon_path.display().to_string().replace('\'', "''"),
    );

    let _ = Command::new("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-WindowStyle",
            "Hidden",
            "-Command",
            &ps_cmd,
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .output();
}

fn configure_theme(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.dark_mode = true;
    visuals.panel_fill = BG_APP;
    visuals.window_fill = BG_CARD;
    visuals.extreme_bg_color = BG_PANEL;
    visuals.faint_bg_color = BG_CONTROL;
    visuals.weak_text_color = Some(TEXT_SECONDARY);
    visuals.disabled_alpha = 0.72;

    visuals.widgets.noninteractive.bg_fill = BG_CARD;
    visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, BORDER_SUBTLE);
    visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, TEXT_PRIMARY);

    visuals.widgets.inactive.bg_fill = BG_CONTROL;
    visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, BORDER_CONTROL);
    visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, TEXT_PRIMARY);
    visuals.widgets.inactive.corner_radius = egui::CornerRadius::same(5);

    visuals.widgets.hovered.bg_fill = BG_CONTROL_HOVER;
    visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, ACCENT_BRIGHT);
    visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.5, egui::Color32::WHITE);
    visuals.widgets.hovered.corner_radius = egui::CornerRadius::same(5);

    visuals.widgets.active.bg_fill = ACCENT;
    visuals.widgets.active.bg_stroke = egui::Stroke::new(1.0, ACCENT_BRIGHT);
    visuals.widgets.active.fg_stroke = egui::Stroke::new(1.5, egui::Color32::WHITE);
    visuals.widgets.active.corner_radius = egui::CornerRadius::same(5);

    visuals.selection.bg_fill = ACCENT;
    visuals.selection.stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);

    ctx.set_visuals(visuals);
}

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
                    .size(13.5)
                    .strong()
                    .color(TEXT_PRIMARY),
            );
            ui.add_space(8.0);
            add_contents(ui);
        });
}

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
            ui.horizontal(|ui| {
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
                    } else if resp.hovered() {
                        BG_CONTROL_HOVER
                    } else {
                        egui::Color32::TRANSPARENT
                    };
                    if bg != egui::Color32::TRANSPARENT {
                        ui.painter().rect_filled(rect, 4.0, bg);
                    }
                    let fg = if selected {
                        egui::Color32::WHITE
                    } else if resp.hovered() {
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
        let f = if resp.hovered() { ACCENT_HOVER } else { ACCENT };
        (f, ACCENT_BRIGHT, egui::Color32::WHITE)
    } else {
        let b = if resp.hovered() {
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

fn styled_button(
    ui: &mut egui::Ui,
    label: &str,
    fill: egui::Color32,
    hover_fill: egui::Color32,
    border: egui::Color32,
    text_col: egui::Color32,
    width: f32,
    height: f32,
) -> egui::Response {
    let font = egui::FontId::proportional(13.0);
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_string(), font, text_col);
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::click());
    let cur_fill = if resp.hovered() { hover_fill } else { fill };
    let cur_border = if resp.hovered() {
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
    let text_pos = rect.center() - galley.size() * 0.5;
    ui.painter().galley(text_pos, galley, text_col);
    resp
}

impl eframe::App for SetupApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        configure_theme(ui.ctx());

        // Top header with title & language selector
        egui::Panel::top("setup_header")
            .frame(
                egui::Frame::new()
                    .fill(BG_PANEL)
                    .stroke(egui::Stroke::new(1.0, BORDER_SUBTLE))
                    .inner_margin(egui::Margin::symmetric(16, 12)),
            )
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(
                            egui::RichText::new("Sniper Zoom 2.0")
                                .size(17.0)
                                .strong()
                                .color(TEXT_PRIMARY),
                        );
                        ui.label(
                            egui::RichText::new(self.t(
                                "Установка программы для Windows",
                                "Windows Installer",
                            ))
                            .size(12.0)
                            .color(TEXT_SECONDARY),
                        );
                    });

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        segmented_control(
                            ui,
                            &mut self.lang,
                            &[(Lang::Ru, "Русский"), (Lang::En, "English")],
                        );
                    });
                });
            });

        // Bottom pinned action bar (never clipped off-screen)
        egui::Panel::bottom("setup_footer")
            .frame(
                egui::Frame::new()
                    .fill(BG_PANEL)
                    .stroke(egui::Stroke::new(1.0, BORDER_SUBTLE))
                    .inner_margin(egui::Margin::symmetric(16, 10)),
            )
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let install_btn_text = if self.install_finished {
                        self.t("Переустановить", "Reinstall")
                    } else {
                        self.t("Установить", "Install")
                    };

                    if styled_button(
                        ui,
                        install_btn_text,
                        ACCENT,
                        ACCENT_HOVER,
                        ACCENT_BRIGHT,
                        egui::Color32::WHITE,
                        180.0,
                        34.0,
                    )
                    .clicked()
                    {
                        self.perform_install();
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if styled_button(
                            ui,
                            self.t("Закрыть", "Close"),
                            BG_CONTROL,
                            BG_CONTROL_HOVER,
                            BORDER_CONTROL,
                            TEXT_PRIMARY,
                            96.0,
                            34.0,
                        )
                        .clicked()
                        {
                            std::process::exit(0);
                        }

                        ui.add_space(6.0);

                        if styled_button(
                            ui,
                            self.t("Удалить", "Uninstall"),
                            DANGER_BG,
                            DANGER_BG_HOVER,
                            DANGER_BORDER,
                            DANGER_TEXT,
                            104.0,
                            34.0,
                        )
                        .clicked()
                        {
                            self.perform_uninstall();
                        }
                    });
                });
            });

        // Main scrollable content area
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(BG_APP)
                    .inner_margin(egui::Margin::symmetric(16, 12)),
            )
            .show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        section_card(
                            ui,
                            self.t("Папка установки", "Install location"),
                            |ui| {
                                ui.add(
                                    egui::TextEdit::singleline(&mut self.install_dir)
                                        .desired_width(f32::INFINITY),
                                );
                            },
                        );

                        ui.add_space(10.0);

                        let lang = self.lang;
                        let tr = |ru: &'static str, en: &'static str| match lang {
                            Lang::Ru => ru,
                            Lang::En => en,
                        };
                        section_card(
                            ui,
                            tr("Параметры установки", "Installation options"),
                            |ui| {
                                toggle_switch(
                                    ui,
                                    &mut self.create_desktop_shortcut,
                                    tr(
                                        "Создать ярлык на рабочем столе",
                                        "Create Desktop shortcut",
                                    ),
                                );
                                ui.add_space(6.0);
                                toggle_switch(
                                    ui,
                                    &mut self.create_start_menu_shortcut,
                                    tr(
                                        "Создать ярлык в меню «Пуск»",
                                        "Create Start Menu shortcut",
                                    ),
                                );
                                ui.add_space(6.0);
                                toggle_switch(
                                    ui,
                                    &mut self.enable_autostart,
                                    tr(
                                        "Запускать вместе с Windows (в трее)",
                                        "Launch at Windows startup (in tray)",
                                    ),
                                );
                                ui.add_space(6.0);
                                toggle_switch(
                                    ui,
                                    &mut self.launch_after_install,
                                    tr(
                                        "Запустить Sniper Zoom после установки",
                                        "Launch Sniper Zoom after installation",
                                    ),
                                );
                            },
                        );

                        if let Some(ref err) = self.error_msg {
                            ui.add_space(10.0);
                            egui::Frame::new()
                                .fill(DANGER_BG)
                                .stroke(egui::Stroke::new(1.0, DANGER_BORDER))
                                .corner_radius(6.0)
                                .inner_margin(egui::Margin::same(10))
                                .show(ui, |ui| {
                                    ui.set_width(ui.available_width());
                                    ui.label(
                                        egui::RichText::new(err)
                                            .size(12.5)
                                            .color(DANGER_TEXT),
                                    );
                                });
                        }

                        if !self.status_log.is_empty() {
                            ui.add_space(10.0);
                            egui::Frame::new()
                                .fill(LOG_BG)
                                .stroke(egui::Stroke::new(1.0, LOG_BORDER))
                                .corner_radius(6.0)
                                .inner_margin(egui::Margin::same(10))
                                .show(ui, |ui| {
                                    ui.set_width(ui.available_width());
                                    for line in &self.status_log {
                                        ui.label(
                                            egui::RichText::new(format!("• {}", line))
                                                .size(12.0)
                                                .color(LOG_TEXT),
                                        );
                                    }
                                });
                        }
                    });
            });
    }
}

fn main() -> eframe::Result {
    set_dpi_aware();

    let native_options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([520.0, 460.0])
            .with_min_inner_size([480.0, 420.0])
            .with_title("Sniper Zoom — Setup")
            .with_icon(Arc::new(create_egui_icon()))
            .with_active(true),
        ..Default::default()
    };

    eframe::run_native(
        "Sniper Zoom Setup",
        native_options,
        Box::new(|_cc| Ok(Box::new(SetupApp::new()))),
    )
}
