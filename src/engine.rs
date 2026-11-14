use crate::config::Config;
use crate::keys::Hotkey;
use crate::overlay::Overlay;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::{Duration, Instant};
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

#[link(name = "winmm")]
unsafe extern "system" {
    fn timeBeginPeriod(uPeriod: u32) -> u32;
    fn timeEndPeriod(uPeriod: u32) -> u32;
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn Beep(dwFreq: u32, dwDuration: u32) -> BOOL;
}

fn play_beep(freq: u32) {
    thread::spawn(move || {
        unsafe {
            Beep(freq, 25);
        }
    });
}

const RETICLES: [(&str, &str, &str); 9] = [
    ("dot", "ТОЧКА", "DOT"),
    ("cross", "КРЕСТИК", "CROSSHAIR"),
    ("t_cross", "Т-ПРИЦЕЛ", "T-CROSS"),
    ("x_cross", "X-КРЕСТ", "X-CROSS"),
    ("circle_dot", "КОЛЛИМАТОР", "RED DOT"),
    ("box", "ТАКТИЧЕСКАЯ РАМКА", "TACTICAL BOX"),
    ("mildot", "MIL-DOT СЕТКА", "MIL-DOT"),
    ("chevron", "ПСО УГОЛЬНИК", "CHEVRON"),
    ("none", "БЕЗ ПРИЦЕЛА", "NO RETICLE"),
];

const SHAPES: [(&str, &str, &str); 4] = [
    ("circle", "КРУГЛАЯ ЛИНЗА", "CIRCLE"),
    ("square", "КВАДРАТ", "SQUARE"),
    ("wide", "ПАНОРАМА 16:9", "WIDE 16:9"),
    ("fullscreen", "ПОЛНЫЙ ЭКРАН", "FULLSCREEN"),
];

const COLORS: [(&str, &str, &str); 6] = [
    ("#00FF00", "ЗЕЛЕНЫЙ", "GREEN"),
    ("#FF0000", "КРАСНЫЙ", "RED"),
    ("#00FFFF", "БИРЮЗОВЫЙ", "CYAN"),
    ("#FFFF00", "ЖЕЛТЫЙ", "YELLOW"),
    ("#FFFFFF", "БЕЛЫЙ", "WHITE"),
    ("#FF00FF", "ФИОЛЕТОВЫЙ", "MAGENTA"),
];

pub fn run_overlay_thread(
    config_shared: Arc<RwLock<Config>>,
    is_active_indicator: Arc<AtomicBool>,
    is_paused: Arc<AtomicBool>,
    force_test_active: Arc<AtomicBool>,
    should_exit: Arc<AtomicBool>,
    osd_message: Arc<RwLock<Option<(String, Instant)>>>,
) {
    thread::spawn(move || {
        unsafe {
            timeBeginPeriod(1);
        }

        let initial_cfg = config_shared.read().unwrap().clone();
        let mut overlay = match Overlay::new(initial_cfg.scope_size, &initial_cfg.shape) {
            Ok(o) => o,
            Err(e) => {
                eprintln!("Не удалось создать оверлей: {}", e);
                return;
            }
        };

        let mut current_hotkey_str = initial_cfg.hotkey.clone();
        let mut hotkey = Hotkey::parse(&current_hotkey_str);

        let mut current_xhair_hotkey_str = initial_cfg.crosshair_hotkey.clone();
        let mut xhair_hotkey = Hotkey::parse(&current_xhair_hotkey_str);

        let mut cur_hk_zoom_out_str = initial_cfg.hk_zoom_out.clone();
        let mut hk_zoom_out = Hotkey::parse(&cur_hk_zoom_out_str);
        let mut cur_hk_zoom_in_str = initial_cfg.hk_zoom_in.clone();
        let mut hk_zoom_in = Hotkey::parse(&cur_hk_zoom_in_str);
        let mut cur_hk_size_dec_str = initial_cfg.hk_size_dec.clone();
        let mut hk_size_dec = Hotkey::parse(&cur_hk_size_dec_str);
        let mut cur_hk_size_inc_str = initial_cfg.hk_size_inc.clone();
        let mut hk_size_inc = Hotkey::parse(&cur_hk_size_inc_str);
        let mut cur_hk_no_zoom_str = initial_cfg.hk_toggle_no_zoom.clone();
        let mut hk_no_zoom = Hotkey::parse(&cur_hk_no_zoom_str);
        let mut cur_hk_reticle_str = initial_cfg.hk_cycle_reticle.clone();
        let mut hk_reticle = Hotkey::parse(&cur_hk_reticle_str);
        let mut cur_hk_shape_str = initial_cfg.hk_cycle_shape.clone();
        let mut hk_shape = Hotkey::parse(&cur_hk_shape_str);
        let mut cur_hk_color_str = initial_cfg.hk_cycle_color.clone();
        let mut hk_color = Hotkey::parse(&cur_hk_color_str);

        let mut is_zoom_active = false;
        let mut last_hotkey_pressed = false;
        let mut last_xhair_hotkey_pressed = false;
        let mut cached_shape = initial_cfg.shape.clone();

        // Edge detection state for in-game quick adjustment hotkeys
        let mut prev_zoom_out = false;
        let mut prev_zoom_in = false;
        let mut prev_size_dec = false;
        let mut prev_size_inc = false;
        let mut prev_no_zoom = false;
        let mut prev_reticle = false;
        let mut prev_shape = false;
        let mut prev_color = false;

        while !should_exit.load(Ordering::Relaxed) {
            let loop_start = Instant::now();

            // 1. Process Windows Messages for the overlay window
            unsafe {
                let mut msg: MSG = std::mem::zeroed();
                while PeekMessageW(&mut msg, 0 as HWND, 0, 0, PM_REMOVE) != 0 {
                    if msg.message == WM_QUIT {
                        return;
                    }
                    TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            }

            let paused = is_paused.load(Ordering::Relaxed);
            let mut cfg = config_shared.read().unwrap().clone();

            // Re-parse hotkeys if changed in GUI
            if cfg.hotkey != current_hotkey_str {
                current_hotkey_str = cfg.hotkey.clone();
                hotkey = Hotkey::parse(&current_hotkey_str);
                last_hotkey_pressed = false;
            }
            if cfg.crosshair_hotkey != current_xhair_hotkey_str {
                current_xhair_hotkey_str = cfg.crosshair_hotkey.clone();
                xhair_hotkey = Hotkey::parse(&current_xhair_hotkey_str);
                last_xhair_hotkey_pressed = false;
            }
            if cfg.hk_zoom_out != cur_hk_zoom_out_str {
                cur_hk_zoom_out_str = cfg.hk_zoom_out.clone();
                hk_zoom_out = Hotkey::parse(&cur_hk_zoom_out_str);
                prev_zoom_out = false;
            }
            if cfg.hk_zoom_in != cur_hk_zoom_in_str {
                cur_hk_zoom_in_str = cfg.hk_zoom_in.clone();
                hk_zoom_in = Hotkey::parse(&cur_hk_zoom_in_str);
                prev_zoom_in = false;
            }
            if cfg.hk_size_dec != cur_hk_size_dec_str {
                cur_hk_size_dec_str = cfg.hk_size_dec.clone();
                hk_size_dec = Hotkey::parse(&cur_hk_size_dec_str);
                prev_size_dec = false;
            }
            if cfg.hk_size_inc != cur_hk_size_inc_str {
                cur_hk_size_inc_str = cfg.hk_size_inc.clone();
                hk_size_inc = Hotkey::parse(&cur_hk_size_inc_str);
                prev_size_inc = false;
            }
            if cfg.hk_toggle_no_zoom != cur_hk_no_zoom_str {
                cur_hk_no_zoom_str = cfg.hk_toggle_no_zoom.clone();
                hk_no_zoom = Hotkey::parse(&cur_hk_no_zoom_str);
                prev_no_zoom = false;
            }
            if cfg.hk_cycle_reticle != cur_hk_reticle_str {
                cur_hk_reticle_str = cfg.hk_cycle_reticle.clone();
                hk_reticle = Hotkey::parse(&cur_hk_reticle_str);
                prev_reticle = false;
            }
            if cfg.hk_cycle_shape != cur_hk_shape_str {
                cur_hk_shape_str = cfg.hk_cycle_shape.clone();
                hk_shape = Hotkey::parse(&cur_hk_shape_str);
                prev_shape = false;
            }
            if cfg.hk_cycle_color != cur_hk_color_str {
                cur_hk_color_str = cfg.hk_cycle_color.clone();
                hk_color = Hotkey::parse(&cur_hk_color_str);
                prev_color = false;
            }

            // 2. Check Dedicated Crosshair Toggle/Hold Hotkey
            let xhair_key_down = !paused && xhair_hotkey.is_bound() && xhair_hotkey.is_pressed();
            let is_xhair_hold = cfg.crosshair_mode.eq_ignore_ascii_case("hold");

            if !paused {
                if is_xhair_hold {
                    if xhair_hotkey.is_bound() && xhair_key_down != cfg.always_show_crosshair {
                        cfg.always_show_crosshair = xhair_key_down;
                        if let Ok(mut writer) = config_shared.write() {
                            writer.always_show_crosshair = xhair_key_down;
                        }
                    }
                } else {
                    // Toggle mode for Crosshair Hotkey
                    if xhair_key_down && !last_xhair_hotkey_pressed {
                        if cfg.mode.eq_ignore_ascii_case("always") && cfg.is_no_zoom_mode() {
                            cfg.crosshair_enabled = !cfg.crosshair_enabled;
                        } else {
                            cfg.always_show_crosshair = !cfg.always_show_crosshair;
                            cfg.crosshair_enabled = cfg.always_show_crosshair;
                        }

                        if (cfg.always_show_crosshair || cfg.crosshair_enabled) && cfg.crosshair == "none" {
                            cfg.apply_crosshair_style("cross");
                        }

                        let now_on = if cfg.mode.eq_ignore_ascii_case("always") && cfg.is_no_zoom_mode() {
                            cfg.crosshair_enabled
                        } else {
                            cfg.always_show_crosshair
                        };

                        let _ = cfg.save();
                        if let Ok(mut writer) = config_shared.write() {
                            *writer = cfg.clone();
                        }

                        if cfg.beep {
                            play_beep(if now_on { 1300 } else { 750 });
                        }
                    }
                    last_xhair_hotkey_pressed = xhair_key_down;
                }
            }

            // 3. Check In-Game Quick Adjustment Hotkeys (without opening GUI)
            if cfg.in_game_hotkeys && !paused {
                let k_zoom_out = hk_zoom_out.is_pressed();
                let k_zoom_in = hk_zoom_in.is_pressed();
                let k_size_dec = hk_size_dec.is_pressed();
                let k_size_inc = hk_size_inc.is_pressed();
                let k_no_zoom = hk_no_zoom.is_pressed();
                let k_reticle = hk_reticle.is_pressed();
                let k_shape = hk_shape.is_pressed();
                let k_color = hk_color.is_pressed();

                let mut quick_changed = false;
                let mut new_osd: Option<String> = None;
                let lang = cfg.language;

                if k_zoom_out && !prev_zoom_out {
                    cfg.zoom_factor = ((cfg.zoom_factor - 0.25) * 10.0).round() / 10.0;
                    cfg.zoom_factor = cfg.zoom_factor.clamp(1.0, 10.0);
                    if cfg.zoom_factor <= 1.02 {
                        cfg.zoom_factor = 1.0;
                        new_osd = Some(
                            lang.t(
                                "РЕЖИМ: 1.0x (ТОЛЬКО ПРИЦЕЛ БЕЗ ЗУМА)",
                                "MODE: 1.0x (CROSSHAIR ONLY)",
                            )
                            .to_string(),
                        );
                    } else {
                        cfg.crosshair_only = false;
                        new_osd = Some(format!("{}: {:.1}x", lang.t("ЗУМ", "ZOOM"), cfg.zoom_factor));
                    }
                    quick_changed = true;
                }
                if k_zoom_in && !prev_zoom_in {
                    cfg.crosshair_only = false;
                    cfg.zoom_factor = ((cfg.zoom_factor + 0.25) * 10.0).round() / 10.0;
                    cfg.zoom_factor = cfg.zoom_factor.clamp(1.0, 10.0);
                    quick_changed = true;
                    new_osd = Some(format!("{}: {:.1}x", lang.t("ЗУМ", "ZOOM"), cfg.zoom_factor));
                }
                if k_size_dec && !prev_size_dec {
                    cfg.scope_size = (cfg.scope_size - 60).clamp(150, 1600);
                    quick_changed = true;
                    new_osd = Some(format!(
                        "{}: {} px",
                        lang.t("ЗОНА SCOPE", "SCOPE SIZE"),
                        cfg.scope_size
                    ));
                }
                if k_size_inc && !prev_size_inc {
                    cfg.scope_size = (cfg.scope_size + 60).clamp(150, 1600);
                    quick_changed = true;
                    new_osd = Some(format!(
                        "{}: {} px",
                        lang.t("ЗОНА SCOPE", "SCOPE SIZE"),
                        cfg.scope_size
                    ));
                }
                if k_no_zoom && !prev_no_zoom {
                    cfg.crosshair_only = !cfg.crosshair_only;
                    if cfg.crosshair_only && cfg.crosshair == "none" {
                        cfg.apply_crosshair_style("cross");
                    }
                    quick_changed = true;
                    new_osd = Some(if cfg.crosshair_only {
                        lang.t("РЕЖИМ: ПРИЦЕЛ БЕЗ ПРИБЛИЖЕНИЯ", "MODE: CROSSHAIR ONLY")
                            .to_string()
                    } else {
                        format!(
                            "{}: {:.1}x",
                            lang.t("РЕЖИМ: ОПТИЧЕСКИЙ ЗУМ", "MODE: OPTICAL ZOOM"),
                            cfg.zoom_factor
                        )
                    });
                }
                if k_reticle && !prev_reticle {
                    let cur_idx = RETICLES
                        .iter()
                        .position(|(id, _, _)| cfg.crosshair.eq_ignore_ascii_case(id))
                        .unwrap_or(0);
                    let next_idx = (cur_idx + 1) % RETICLES.len();
                    cfg.apply_crosshair_style(RETICLES[next_idx].0);
                    quick_changed = true;
                    let item = RETICLES[next_idx];
                    new_osd = Some(format!(
                        "{}: {}",
                        lang.t("ПРИЦЕЛ", "RETICLE"),
                        lang.t(item.1, item.2)
                    ));
                }
                if k_shape && !prev_shape {
                    let cur_idx = SHAPES
                        .iter()
                        .position(|(id, _, _)| cfg.shape.eq_ignore_ascii_case(id))
                        .unwrap_or(0);
                    let next_idx = (cur_idx + 1) % SHAPES.len();
                    cfg.shape = SHAPES[next_idx].0.to_string();
                    quick_changed = true;
                    let item = SHAPES[next_idx];
                    new_osd = Some(format!(
                        "{}: {}",
                        lang.t("ФОРМА", "SHAPE"),
                        lang.t(item.1, item.2)
                    ));
                }
                if k_color && !prev_color {
                    let cur_idx = COLORS
                        .iter()
                        .position(|(hex, _, _)| cfg.crosshair_color.eq_ignore_ascii_case(hex))
                        .unwrap_or(0);
                    let next_idx = (cur_idx + 1) % COLORS.len();
                    cfg.crosshair_color = COLORS[next_idx].0.to_string();
                    quick_changed = true;
                    let item = COLORS[next_idx];
                    new_osd = Some(format!(
                        "{}: {}",
                        lang.t("ЦВЕТ", "COLOR"),
                        lang.t(item.1, item.2)
                    ));
                }

                prev_zoom_out = k_zoom_out;
                prev_zoom_in = k_zoom_in;
                prev_size_dec = k_size_dec;
                prev_size_inc = k_size_inc;
                prev_no_zoom = k_no_zoom;
                prev_reticle = k_reticle;
                prev_shape = k_shape;
                prev_color = k_color;

                if quick_changed {
                    let _ = cfg.save();
                    if let Ok(mut writer) = config_shared.write() {
                        *writer = cfg.clone();
                    }
                    if let Some(msg) = new_osd {
                        if let Ok(mut osd) = osd_message.write() {
                            *osd = Some((msg, Instant::now()));
                        }
                    }
                    if cfg.beep {
                        play_beep(1050);
                    }
                }
            }

            // Update shape region if shape changed
            if cfg.shape != cached_shape {
                cached_shape = cfg.shape.clone();
                overlay.force_region_update(&cached_shape);
            }

            let is_hold_mode = cfg.mode.eq_ignore_ascii_case("hold");
            let is_always_mode = cfg.mode.eq_ignore_ascii_case("always");

            // 4. Determine if primary activation (Zoom / Main Key) should be active
            let hotkey_is_down = !paused && hotkey.is_bound() && hotkey.is_pressed();
            let forced = force_test_active.load(Ordering::Relaxed);

            // Check if an OSD notification was recently triggered (within 1.5s)
            let active_osd: Option<String> = {
                if let Ok(osd_guard) = osd_message.read() {
                    if let Some((ref text, ts)) = *osd_guard {
                        if ts.elapsed() < Duration::from_millis(1600) {
                            Some(text.clone())
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            };

            let should_be_active = if forced {
                true
            } else if paused {
                is_zoom_active = false;
                false
            } else if is_always_mode {
                cfg.crosshair_enabled
            } else if is_hold_mode {
                hotkey_is_down
            } else {
                // Toggle mode
                if hotkey_is_down && !last_hotkey_pressed {
                    is_zoom_active = !is_zoom_active;
                    if cfg.beep {
                        play_beep(if is_zoom_active { 1200 } else { 800 });
                    }
                }
                last_hotkey_pressed = hotkey_is_down;
                is_zoom_active
            };

            if is_hold_mode || is_always_mode {
                if should_be_active != is_zoom_active {
                    is_zoom_active = should_be_active;
                    if cfg.beep && !is_always_mode {
                        play_beep(if is_zoom_active { 1200 } else { 800 });
                    }
                }
            } else if forced && !is_zoom_active {
                is_zoom_active = true;
            }

            let standalone_crosshair_active = !paused && cfg.always_show_crosshair;
            let render_now = is_zoom_active || standalone_crosshair_active || active_osd.is_some();

            is_active_indicator.store(is_zoom_active || standalone_crosshair_active, Ordering::Relaxed);

            // 5. Update overlay visibility & render frame
            if render_now {
                overlay.show();
                let use_crosshair_only = if is_zoom_active || active_osd.is_some() {
                    cfg.is_no_zoom_mode()
                } else {
                    // Standalone crosshair without zoom toggled on by Crosshair Hotkey
                    true
                };

                if use_crosshair_only {
                    overlay.render_crosshair_only(&cfg, active_osd.as_deref());
                } else {
                    overlay.render_frame(&cfg, active_osd.as_deref());
                }
            } else {
                overlay.hide();
            }

            // 6. FPS pacing
            if render_now {
                let is_zoom_rendering = (is_zoom_active || active_osd.is_some()) && !cfg.is_no_zoom_mode();
                let target_fps = if is_zoom_rendering || cfg.follow_mouse {
                    cfg.fps_limit.max(30)
                } else {
                    30
                };
                let frame_duration = Duration::from_nanos(1_000_000_000 / target_fps as u64);
                let elapsed = loop_start.elapsed();
                if elapsed < frame_duration {
                    thread::sleep(frame_duration - elapsed);
                }
            } else {
                // Idle sleep
                thread::sleep(Duration::from_millis(10));
            }
        }

        overlay.hide();
        unsafe {
            timeEndPeriod(1);
        }
    });
}
