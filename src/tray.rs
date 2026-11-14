use crate::config::{Config, OperatingMode};
use crate::icon::create_tray_hicon;
use crate::keys::format_hotkey_display_lang;
use std::ptr::null_mut;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::Instant;
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

const WM_TRAYICON: u32 = WM_USER + 101;
const TRAY_ICON_ID: u32 = 1;

const NIM_ADD: u32 = 0x00000000;
const NIM_MODIFY: u32 = 0x00000001;
const NIM_DELETE: u32 = 0x00000002;

const NIF_MESSAGE: u32 = 0x00000001;
const NIF_ICON: u32 = 0x00000002;
const NIF_TIP: u32 = 0x00000004;
const NIF_INFO: u32 = 0x00000010;
const NIIF_INFO: u32 = 0x00000001;

#[repr(C)]
#[allow(non_snake_case)]
struct NOTIFYICONDATAW {
    cbSize: u32,
    hWnd: HWND,
    uID: u32,
    uFlags: u32,
    uCallbackMessage: u32,
    hIcon: HICON,
    szTip: [u16; 128],
    dwState: u32,
    dwStateMask: u32,
    szInfo: [u16; 256],
    uTimeoutOrVersion: u32,
    szInfoTitle: [u16; 64],
    dwInfoFlags: u32,
    guidItem: [u8; 16],
    hBalloonIcon: HICON,
}

#[link(name = "shell32")]
unsafe extern "system" {
    fn Shell_NotifyIconW(dwMessage: u32, lpData: *const NOTIFYICONDATAW) -> BOOL;
}

// Menu Command IDs
const CMD_TOGGLE_WINDOW: usize = 1001;
const CMD_TOGGLE_PAUSE: usize = 1002;
const CMD_TOGGLE_ALWAYS_CROSSHAIR: usize = 1005;

const CMD_OPMODE_ZOOM_XHAIR: usize = 1020;
const CMD_OPMODE_HYBRID: usize = 1021;
const CMD_OPMODE_XHAIR_ONLY: usize = 1022;
const CMD_OPMODE_ZOOM_ONLY: usize = 1023;

const CMD_ZOOM_BASE: usize = 1100; // 1100..1115
const ZOOM_PRESETS: [f32; 10] = [1.0, 1.5, 2.0, 2.5, 3.0, 4.0, 5.0, 6.0, 8.0, 10.0];

const CMD_USER_PROFILE_BASE: usize = 1700;
const MAX_TRAY_USER_PROFILES: usize = 30;

const CMD_EXIT: usize = 1999;

pub const MAIN_WINDOW_TITLE: &str = "Sniper Zoom";

fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn fill_wide_buf(buf: &mut [u16], text: &str) {
    for v in buf.iter_mut() {
        *v = 0;
    }
    for (i, ch) in text.encode_utf16().enumerate() {
        if i + 1 < buf.len() {
            buf[i] = ch;
        } else {
            break;
        }
    }
}

/// Finds the native HWND of the main Sniper Zoom GUI window.
pub fn find_main_window() -> HWND {
    unsafe {
        let title = to_wide(MAIN_WINDOW_TITLE);
        FindWindowW(null_mut(), title.as_ptr())
    }
}

/// Hides the main GUI window so the app lives solely in the System Tray (no taskbar entry).
pub fn hide_main_window_to_tray(window_visible: &AtomicBool, show_balloon: &AtomicBool) {
    let hwnd = find_main_window();
    if !hwnd.is_null() {
        unsafe {
            ShowWindow(hwnd, SW_HIDE);
        }
        window_visible.store(false, Ordering::Relaxed);
        show_balloon.store(true, Ordering::Relaxed);
    }
}

/// Restores and focuses the main GUI window from the System Tray.
pub fn restore_main_window_from_tray(window_visible: &AtomicBool) {
    let hwnd = find_main_window();
    if !hwnd.is_null() {
        unsafe {
            ShowWindow(hwnd, SW_SHOW);
            ShowWindow(hwnd, SW_RESTORE);
            SetForegroundWindow(hwnd);
        }
        window_visible.store(true, Ordering::Relaxed);
    }
}

struct TraySharedState {
    config: Arc<RwLock<Config>>,
    is_paused: Arc<AtomicBool>,
    window_visible: Arc<AtomicBool>,
    should_exit: Arc<AtomicBool>,
    osd_message: Arc<RwLock<Option<(String, Instant)>>>,
}

static mut TRAY_STATE_PTR: *const TraySharedState = std::ptr::null();

unsafe extern "system" fn tray_wndproc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    unsafe {
        if msg == WM_TRAYICON {
            let event = (lparam as u32) & 0xFFFF;
            if !TRAY_STATE_PTR.is_null() {
                let state = &*TRAY_STATE_PTR;
                match event {
                    WM_LBUTTONUP | WM_LBUTTONDBLCLK => {
                        let main_hwnd = find_main_window();
                        if !main_hwnd.is_null() {
                            let currently_vis = IsWindowVisible(main_hwnd) != 0;
                            if currently_vis {
                                ShowWindow(main_hwnd, SW_HIDE);
                                state.window_visible.store(false, Ordering::Relaxed);
                            } else {
                                restore_main_window_from_tray(&state.window_visible);
                            }
                        }
                    }
                    WM_RBUTTONUP | WM_CONTEXTMENU => {
                        show_tray_context_menu(hwnd, state);
                    }
                    _ => {}
                }
            }
            return 0;
        }
        DefWindowProcW(hwnd, msg, wparam, lparam)
    }
}

unsafe fn append_item(menu: HMENU, flags: u32, id: usize, text: &str) {
    let w = to_wide(text);
    unsafe {
        AppendMenuW(menu, flags, id, w.as_ptr());
    }
}

unsafe fn show_tray_context_menu(hwnd: HWND, state: &TraySharedState) {
    unsafe {
        let menu = CreatePopupMenu();
        if menu.is_null() {
            return;
        }

        let cfg = state.config.read().unwrap().clone();
        let lang = cfg.language;
        let paused = state.is_paused.load(Ordering::Relaxed);
        let main_hwnd = find_main_window();
        let is_win_vis = !main_hwnd.is_null() && IsWindowVisible(main_hwnd) != 0;

        // 1. Pause / Resume (action label matches current state, no checkmark or parentheses)
        append_item(
            menu,
            MF_STRING,
            CMD_TOGGLE_PAUSE,
            if paused {
                lang.t("Продолжить", "Resume")
            } else {
                lang.t("Приостановить", "Pause")
            },
        );

        // 2. Always-on crosshair toggle with right-aligned hotkey
        let xhair_on = if cfg.mode.eq_ignore_ascii_case("always") && cfg.is_no_zoom_mode() {
            cfg.crosshair_enabled
        } else {
            cfg.always_show_crosshair
        };
        let xhair_title = lang.t("Постоянный прицел", "Always-on crosshair");
        let xhair_label = if !cfg.crosshair_hotkey.trim().is_empty() {
            format!(
                "{}\t{}",
                xhair_title,
                format_hotkey_display_lang(&cfg.crosshair_hotkey, lang)
            )
        } else {
            xhair_title.to_string()
        };
        append_item(
            menu,
            MF_STRING | if xhair_on { MF_CHECKED } else { 0 },
            CMD_TOGGLE_ALWAYS_CROSSHAIR,
            &xhair_label,
        );

        AppendMenuW(menu, MF_SEPARATOR, 0, null_mut());

        // 3. Submenu: User Profiles
        let menu_profiles = CreatePopupMenu();
        if cfg.user_profiles.is_empty() {
            append_item(
                menu_profiles,
                MF_STRING | MF_GRAYED,
                0,
                lang.t("Нет сохранённых профилей", "No saved profiles"),
            );
        } else {
            for (idx, prof) in cfg
                .user_profiles
                .iter()
                .take(MAX_TRAY_USER_PROFILES)
                .enumerate()
            {
                append_item(
                    menu_profiles,
                    MF_STRING,
                    CMD_USER_PROFILE_BASE + idx,
                    &prof.name,
                );
            }
        }
        let profiles_title = if cfg.user_profiles.is_empty() {
            lang.t("Мои профили", "My profiles").to_string()
        } else {
            format!(
                "{} ({})",
                lang.t("Мои профили", "My profiles"),
                cfg.user_profiles.len()
            )
        };
        append_item(menu, MF_POPUP, menu_profiles as usize, &profiles_title);

        // 4. Submenu: Unified operating mode
        let menu_opmode = CreatePopupMenu();
        let op_mode = cfg.operating_mode();
        let opmode_items = [
            (
                CMD_OPMODE_ZOOM_XHAIR,
                OperatingMode::ZoomWithCrosshair,
                lang.t("Прицел при приближении", "Crosshair on zoom"),
            ),
            (
                CMD_OPMODE_HYBRID,
                OperatingMode::AlwaysCrosshairPlusZoom,
                lang.t("Постоянный прицел и зум", "Always crosshair + zoom"),
            ),
            (
                CMD_OPMODE_XHAIR_ONLY,
                OperatingMode::CrosshairOnly,
                lang.t("Только прицел (без зума)", "Crosshair only (no zoom)"),
            ),
            (
                CMD_OPMODE_ZOOM_ONLY,
                OperatingMode::ZoomOnly,
                lang.t("Чистый зум (без прицела)", "Zoom only (no crosshair)"),
            ),
        ];
        for (cmd_id, mode_val, label) in opmode_items {
            append_item(
                menu_opmode,
                MF_STRING | if op_mode == mode_val { MF_CHECKED } else { 0 },
                cmd_id,
                label,
            );
        }
        append_item(
            menu,
            MF_POPUP,
            menu_opmode as usize,
            &format!(
                "{} ({})",
                lang.t("Режим работы", "Operating mode"),
                cfg.operating_mode_short_label()
            ),
        );

        // 5. Submenu: Zoom Factor
        let menu_zoom = CreatePopupMenu();
        for (idx, &z) in ZOOM_PRESETS.iter().enumerate() {
            let checked = if z <= 1.02 {
                cfg.is_no_zoom_mode()
            } else {
                !cfg.crosshair_only && (cfg.zoom_factor - z).abs() < 0.08
            };
            let label = if z <= 1.02 {
                lang.t("1.0x (Без приближения)", "1.0x (No zoom)").to_string()
            } else {
                format!("{:.1}x", z)
            };
            append_item(
                menu_zoom,
                MF_STRING | if checked { MF_CHECKED } else { 0 },
                CMD_ZOOM_BASE + idx,
                &label,
            );
        }
        let zoom_title = if cfg.is_no_zoom_mode() {
            format!("{} (1.0x)", lang.t("Кратность зума", "Zoom factor"))
        } else {
            format!(
                "{} ({:.1}x)",
                lang.t("Кратность зума", "Zoom factor"),
                cfg.zoom_factor
            )
        };
        append_item(menu, MF_POPUP, menu_zoom as usize, &zoom_title);

        AppendMenuW(menu, MF_SEPARATOR, 0, null_mut());

        // 6. Open / Hide Settings Window
        append_item(
            menu,
            MF_STRING,
            CMD_TOGGLE_WINDOW,
            if is_win_vis {
                lang.t("Скрыть настройки", "Hide settings")
            } else {
                lang.t("Открыть настройки", "Open settings")
            },
        );

        // 7. Exit
        append_item(menu, MF_STRING, CMD_EXIT, lang.t("Выход", "Exit"));

        let mut pt = POINT { x: 0, y: 0 };
        GetCursorPos(&mut pt);
        SetForegroundWindow(hwnd);

        let cmd = TrackPopupMenu(
            menu,
            TPM_RETURNCMD | TPM_NONOTIFY | TPM_BOTTOMALIGN | TPM_LEFTALIGN,
            pt.x,
            pt.y,
            0,
            hwnd,
            null_mut(),
        ) as usize;

        PostMessageW(hwnd, WM_NULL, 0, 0);
        DestroyMenu(menu);

        if cmd != 0 {
            handle_tray_command(cmd, state);
        }
    }
}

fn handle_tray_command(cmd: usize, state: &TraySharedState) {
    match cmd {
        CMD_TOGGLE_WINDOW => {
            let main_hwnd = find_main_window();
            if !main_hwnd.is_null() {
                unsafe {
                    if IsWindowVisible(main_hwnd) != 0 {
                        ShowWindow(main_hwnd, SW_HIDE);
                        state.window_visible.store(false, Ordering::Relaxed);
                    } else {
                        restore_main_window_from_tray(&state.window_visible);
                    }
                }
            }
        }
        CMD_TOGGLE_PAUSE => {
            let prev = state.is_paused.load(Ordering::Relaxed);
            state.is_paused.store(!prev, Ordering::Relaxed);
        }
        CMD_EXIT => {
            state.should_exit.store(true, Ordering::Relaxed);
            let main_hwnd = find_main_window();
            if !main_hwnd.is_null() {
                unsafe {
                    PostMessageW(main_hwnd, WM_CLOSE, 0, 0);
                }
            }
            std::process::exit(0);
        }
        _ => {
            let mut cfg = state.config.write().unwrap();
            let lang = cfg.language;
            let mut osd_text: Option<String> = None;

            if cmd == CMD_TOGGLE_ALWAYS_CROSSHAIR {
                let cur_on = if cfg.mode.eq_ignore_ascii_case("always") && cfg.is_no_zoom_mode() {
                    cfg.crosshair_enabled
                } else {
                    cfg.always_show_crosshair
                };
                let next_on = !cur_on;
                cfg.always_show_crosshair = next_on;
                cfg.crosshair_enabled = next_on;
                if next_on && cfg.crosshair == "none" {
                    cfg.apply_crosshair_style("cross");
                }
                osd_text = Some(if next_on {
                    lang.t("ПРИЦЕЛ: ВКЛЮЧЁН", "CROSSHAIR: ON").to_string()
                } else {
                    lang.t("ПРИЦЕЛ: ВЫКЛЮЧЕН", "CROSSHAIR: OFF").to_string()
                });
            } else if cmd == CMD_OPMODE_ZOOM_XHAIR {
                cfg.set_operating_mode(OperatingMode::ZoomWithCrosshair);
                osd_text = Some(
                    lang.t(
                        "РЕЖИМ: ПРИЦЕЛ ПРИ ПРИБЛИЖЕНИИ",
                        "MODE: CROSSHAIR ON ZOOM",
                    )
                    .to_string(),
                );
            } else if cmd == CMD_OPMODE_HYBRID {
                cfg.set_operating_mode(OperatingMode::AlwaysCrosshairPlusZoom);
                osd_text = Some(
                    lang.t(
                        "РЕЖИМ: ПОСТОЯННЫЙ ПРИЦЕЛ И ЗУМ",
                        "MODE: ALWAYS CROSSHAIR + ZOOM",
                    )
                    .to_string(),
                );
            } else if cmd == CMD_OPMODE_XHAIR_ONLY {
                cfg.set_operating_mode(OperatingMode::CrosshairOnly);
                osd_text = Some(
                    lang.t("РЕЖИМ: ТОЛЬКО ПРИЦЕЛ (1.0x)", "MODE: CROSSHAIR ONLY (1.0x)")
                        .to_string(),
                );
            } else if cmd == CMD_OPMODE_ZOOM_ONLY {
                cfg.set_operating_mode(OperatingMode::ZoomOnly);
                osd_text = Some(
                    lang.t(
                        "РЕЖИМ: ЧИСТЫЙ ЗУМ (БЕЗ ПРИЦЕЛА)",
                        "MODE: ZOOM ONLY (NO CROSSHAIR)",
                    )
                    .to_string(),
                );
            } else if cmd >= CMD_ZOOM_BASE && cmd < CMD_ZOOM_BASE + ZOOM_PRESETS.len() {
                let z = ZOOM_PRESETS[cmd - CMD_ZOOM_BASE];
                cfg.zoom_factor = z;
                if z <= 1.02 {
                    cfg.crosshair_only = true;
                    if cfg.crosshair == "none" {
                        cfg.apply_crosshair_style("cross");
                    }
                    osd_text = Some(
                        lang.t(
                            "КРАТНОСТЬ: 1.0x (БЕЗ ПРИБЛИЖЕНИЯ)",
                            "ZOOM: 1.0x (NO ZOOM)",
                        )
                        .to_string(),
                    );
                } else {
                    cfg.crosshair_only = false;
                    osd_text = Some(format!(
                        "{}: {:.1}x",
                        lang.t("КРАТНОСТЬ ЗУМА", "ZOOM"),
                        z
                    ));
                }
            } else if cmd >= CMD_USER_PROFILE_BASE
                && cmd < CMD_USER_PROFILE_BASE + MAX_TRAY_USER_PROFILES
            {
                let idx = cmd - CMD_USER_PROFILE_BASE;
                if let Some(name) = cfg.apply_user_profile(idx) {
                    osd_text = Some(format!(
                        "{}: {}",
                        lang.t("ПРОФИЛЬ", "PROFILE"),
                        name.to_uppercase()
                    ));
                }
            }

            let _ = cfg.save();
            drop(cfg);

            if let Some(msg) = osd_text {
                if let Ok(mut osd) = state.osd_message.write() {
                    *osd = Some((msg, Instant::now()));
                }
            }
        }
    }
}

pub fn run_tray_thread(
    config: Arc<RwLock<Config>>,
    is_paused: Arc<AtomicBool>,
    _force_test_active: Arc<AtomicBool>,
    window_visible: Arc<AtomicBool>,
    show_balloon_flag: Arc<AtomicBool>,
    should_exit: Arc<AtomicBool>,
    osd_message: Arc<RwLock<Option<(String, Instant)>>>,
) {
    thread::spawn(move || {
        unsafe {
            let class_name = to_wide("SniperZoomTrayHiddenWnd");
            let wc = WNDCLASSW {
                style: 0,
                lpfnWndProc: Some(tray_wndproc),
                cbClsExtra: 0,
                cbWndExtra: 0,
                hInstance: null_mut(),
                hIcon: null_mut(),
                hCursor: null_mut(),
                hbrBackground: null_mut(),
                lpszMenuName: null_mut(),
                lpszClassName: class_name.as_ptr(),
            };
            RegisterClassW(&wc);

            let hwnd = CreateWindowExW(
                0,
                class_name.as_ptr(),
                class_name.as_ptr(),
                WS_POPUP,
                0,
                0,
                0,
                0,
                null_mut(),
                null_mut(),
                null_mut(),
                null_mut(),
            );

            if hwnd.is_null() {
                return;
            }

            let state_box = Box::new(TraySharedState {
                config: Arc::clone(&config),
                is_paused: Arc::clone(&is_paused),
                window_visible: Arc::clone(&window_visible),
                should_exit: Arc::clone(&should_exit),
                osd_message: Arc::clone(&osd_message),
            });
            TRAY_STATE_PTR = Box::into_raw(state_box);

            let hicon_active = create_tray_hicon(false);
            let hicon_paused = create_tray_hicon(true);

            let mut nid: NOTIFYICONDATAW = std::mem::zeroed();
            nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
            nid.hWnd = hwnd;
            nid.uID = TRAY_ICON_ID;
            nid.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
            nid.uCallbackMessage = WM_TRAYICON;
            nid.hIcon = hicon_active;
            let init_lang = config.read().map(|c| c.language).unwrap_or_default();
            fill_wide_buf(
                &mut nid.szTip,
                init_lang.t(
                    "Sniper Zoom — ЛКМ: окно | ПКМ: меню",
                    "Sniper Zoom — LMB: window | RMB: menu",
                ),
            );

            Shell_NotifyIconW(NIM_ADD, &nid);

            let mut last_paused = false;
            let mut last_lang = init_lang;

            while !should_exit.load(Ordering::Relaxed) {
                let mut msg: MSG = std::mem::zeroed();
                while PeekMessageW(&mut msg, null_mut(), 0, 0, PM_REMOVE) != 0 {
                    if msg.message == WM_QUIT {
                        break;
                    }
                    TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }

                // Update tray icon if pause state or language changed
                let cur_paused = is_paused.load(Ordering::Relaxed);
                let cur_lang = config.read().map(|c| c.language).unwrap_or_default();
                if cur_paused != last_paused || cur_lang != last_lang {
                    last_paused = cur_paused;
                    last_lang = cur_lang;
                    nid.uFlags = NIF_ICON | NIF_TIP;
                    nid.hIcon = if cur_paused {
                        hicon_paused
                    } else {
                        hicon_active
                    };
                    fill_wide_buf(
                        &mut nid.szTip,
                        if cur_paused {
                            cur_lang.t(
                                "Sniper Zoom (на паузе) — ПКМ: меню",
                                "Sniper Zoom (paused) — RMB: menu",
                            )
                        } else {
                            cur_lang.t(
                                "Sniper Zoom — ЛКМ: окно | ПКМ: меню",
                                "Sniper Zoom — LMB: window | RMB: menu",
                            )
                        },
                    );
                    Shell_NotifyIconW(NIM_MODIFY, &nid);
                }

                // Show balloon notification when minimized to tray
                if show_balloon_flag.swap(false, Ordering::Relaxed) {
                    nid.uFlags = NIF_INFO;
                    fill_wide_buf(
                        &mut nid.szInfoTitle,
                        cur_lang.t("Sniper Zoom работает в фоне", "Sniper Zoom is running in background"),
                    );
                    fill_wide_buf(
                        &mut nid.szInfo,
                        cur_lang.t(
                            "Прицел активен. Нажмите ПКМ по значку в трее для быстрого управления.",
                            "Overlay is active. Right-click the tray icon for quick controls.",
                        ),
                    );
                    nid.dwInfoFlags = NIIF_INFO;
                    Shell_NotifyIconW(NIM_MODIFY, &nid);
                }

                thread::sleep(std::time::Duration::from_millis(25));
            }

            nid.uFlags = 0;
            Shell_NotifyIconW(NIM_DELETE, &nid);
            DestroyIcon(hicon_active);
            DestroyIcon(hicon_paused);
            DestroyWindow(hwnd);
        }
    });
}
