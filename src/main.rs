#![windows_subsystem = "windows"]

mod config;
mod engine;
mod gui;
mod icon;
mod keys;
mod overlay;
mod tray;

use config::Config;
use engine::run_overlay_thread;
use gui::SniperZoomApp;
use icon::create_egui_icon;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use tray::{find_main_window, restore_main_window_from_tray, run_tray_thread, MAIN_WINDOW_TITLE};

#[link(name = "user32")]
unsafe extern "system" {
    fn SetProcessDpiAwarenessContext(value: isize) -> i32;
    fn SetProcessDPIAware() -> i32;
}

fn set_dpi_aware() {
    unsafe {
        // DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2 = -4
        if SetProcessDpiAwarenessContext(-4) == 0 {
            SetProcessDPIAware();
        }
    }
}

fn main() -> eframe::Result {
    set_dpi_aware();

    // Single-instance check: if Sniper Zoom is already running in tray, restore its window and exit
    let existing_hwnd = find_main_window();
    if !existing_hwnd.is_null() {
        let dummy = AtomicBool::new(true);
        restore_main_window_from_tray(&dummy);
        return Ok(());
    }

    let args: Vec<String> = std::env::args().collect();
    let arg_minimized = args
        .iter()
        .any(|a| a.eq_ignore_ascii_case("--minimized") || a.eq_ignore_ascii_case("--tray"));

    let cfg = Config::load_or_create();
    let start_hidden = arg_minimized || cfg.start_minimized;

    let config_shared = Arc::new(RwLock::new(cfg));
    let is_active_indicator = Arc::new(AtomicBool::new(false));
    let is_paused = Arc::new(AtomicBool::new(false));
    let force_test_active = Arc::new(AtomicBool::new(false));
    let window_visible = Arc::new(AtomicBool::new(!start_hidden));
    let show_balloon_flag = Arc::new(AtomicBool::new(false));
    let should_exit = Arc::new(AtomicBool::new(false));
    let osd_message = Arc::new(RwLock::new(None));

    // 1. Launch background overlay engine
    run_overlay_thread(
        Arc::clone(&config_shared),
        Arc::clone(&is_active_indicator),
        Arc::clone(&is_paused),
        Arc::clone(&force_test_active),
        Arc::clone(&should_exit),
        Arc::clone(&osd_message),
    );

    // 2. Launch native Windows System Tray icon & context menu thread
    run_tray_thread(
        Arc::clone(&config_shared),
        Arc::clone(&is_paused),
        Arc::clone(&force_test_active),
        Arc::clone(&window_visible),
        Arc::clone(&show_balloon_flag),
        Arc::clone(&should_exit),
        Arc::clone(&osd_message),
    );

    // 3. Setup native GUI window options with custom icon
    let native_options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([940.0, 720.0])
            .with_min_inner_size([780.0, 560.0])
            .with_title(MAIN_WINDOW_TITLE)
            .with_icon(Arc::new(create_egui_icon()))
            .with_active(!start_hidden),
        ..Default::default()
    };

    let app = SniperZoomApp::new(
        config_shared,
        is_active_indicator,
        is_paused,
        force_test_active,
        window_visible,
        show_balloon_flag,
        Arc::clone(&should_exit),
        osd_message,
        start_hidden,
    );

    let result = eframe::run_native(
        "Sniper Zoom",
        native_options,
        Box::new(|_cc| Ok(Box::new(app))),
    );

    should_exit.store(true, Ordering::Relaxed);
    result
}
