use crate::config::Config;
use std::ptr::null_mut;
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Graphics::Gdi::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

const CLASS_NAME: [u16; 16] = [
    'S' as u16, 'n' as u16, 'i' as u16, 'p' as u16, 'e' as u16, 'r' as u16,
    'Z' as u16, 'o' as u16, 'o' as u16, 'm' as u16, 'W' as u16, 'i' as u16,
    'n' as u16, 'd' as u16, 'o' as u16, 0,
];

/// Near-black chroma key (R=1, G=2, B=1) for 100% transparent background in crosshair-only mode.
/// Distinct from pure black (0x000000) so crosshair shadows remain opaque and sharp.
const TRANSPARENT_COLOR_KEY: u32 = 0x00010201;

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_NCHITTEST => HTTRANSPARENT as LRESULT,
        WM_DESTROY => {
            unsafe {
                PostQuitMessage(0);
            }
            0
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

pub struct Overlay {
    pub hwnd: HWND,
    pub screen_dc: HDC,
    pub mem_dc: HDC,
    pub bitmap: HBITMAP,
    pub old_bitmap: HGDIOBJ,
    pub hud_font: HFONT,
    pub current_w: i32,
    pub current_h: i32,
    pub last_win_x: i32,
    pub last_win_y: i32,
    pub screen_w: i32,
    pub screen_h: i32,
    pub is_visible: bool,
    pub is_colorkey_mode: bool,
}

pub fn compute_dimensions(scope_size: i32, shape: &str, screen_w: i32, screen_h: i32) -> (i32, i32) {
    let clamped = scope_size.clamp(150, 1600);
    if shape.eq_ignore_ascii_case("fullscreen") {
        (screen_w, screen_h)
    } else if shape.eq_ignore_ascii_case("wide") {
        let w = clamped.min(screen_w);
        let h = ((w * 9) / 16).max(120).min(screen_h);
        (w, h)
    } else {
        let s = clamped.min(screen_w).min(screen_h);
        (s, s)
    }
}

impl Overlay {
    pub fn new(initial_size: i32, shape: &str) -> Result<Self, String> {
        unsafe {
            let hinstance = 0 as HINSTANCE;

            let wc = WNDCLASSW {
                style: CS_HREDRAW | CS_VREDRAW,
                lpfnWndProc: Some(wndproc),
                cbClsExtra: 0,
                cbWndExtra: 0,
                hInstance: hinstance,
                hIcon: 0 as HICON,
                hCursor: 0 as HCURSOR,
                hbrBackground: 0 as HBRUSH,
                lpszMenuName: null_mut(),
                lpszClassName: CLASS_NAME.as_ptr(),
            };

            RegisterClassW(&wc);

            let screen_w = GetSystemMetrics(SM_CXSCREEN);
            let screen_h = GetSystemMetrics(SM_CYSCREEN);

            let (w, h) = compute_dimensions(initial_size, shape, screen_w, screen_h);
            let x = screen_w / 2 - w / 2;
            let y = screen_h / 2 - h / 2;

            let ex_style = WS_EX_TOPMOST
                | WS_EX_TOOLWINDOW
                | WS_EX_NOACTIVATE
                | WS_EX_LAYERED
                | WS_EX_TRANSPARENT;

            let hwnd = CreateWindowExW(
                ex_style,
                CLASS_NAME.as_ptr(),
                CLASS_NAME.as_ptr(),
                WS_POPUP,
                x,
                y,
                w,
                h,
                0 as HWND,
                0 as HMENU,
                hinstance,
                null_mut(),
            );

            if hwnd.is_null() {
                return Err("Не удалось создать окно оверлея".to_string());
            }

            // Exclude window from screen capture (prevents recursive mirror feedback)
            SetWindowDisplayAffinity(hwnd, WDA_EXCLUDEFROMCAPTURE);

            // Layered window setup
            SetLayeredWindowAttributes(hwnd, 0, 255, LWA_ALPHA);

            // Setup initial shape region
            apply_shape_region(hwnd, w, h, shape);

            // GDI double buffering resources
            let screen_dc = GetDC(0 as HWND);
            let mem_dc = CreateCompatibleDC(screen_dc);
            let bitmap = CreateCompatibleBitmap(screen_dc, w, h);
            let old_bitmap = SelectObject(mem_dc, bitmap);

            // Create tactical HUD font
            let font_name: Vec<u16> = "Consolas\0".encode_utf16().collect();
            let hud_font = CreateFontW(
                15,
                0,
                0,
                0,
                FW_BOLD as i32,
                0,
                0,
                0,
                DEFAULT_CHARSET as u32,
                OUT_DEFAULT_PRECIS as u32,
                CLIP_DEFAULT_PRECIS as u32,
                NONANTIALIASED_QUALITY as u32,
                DEFAULT_PITCH as u32,
                font_name.as_ptr(),
            );

            Ok(Self {
                hwnd,
                screen_dc,
                mem_dc,
                bitmap,
                old_bitmap,
                hud_font,
                current_w: w,
                current_h: h,
                last_win_x: x,
                last_win_y: y,
                screen_w,
                screen_h,
                is_visible: false,
                is_colorkey_mode: false,
            })
        }
    }

    pub fn show(&mut self) {
        if !self.is_visible {
            unsafe {
                ShowWindow(self.hwnd, SW_SHOWNOACTIVATE);
            }
            self.is_visible = true;
        }
    }

    pub fn hide(&mut self) {
        if self.is_visible {
            unsafe {
                ShowWindow(self.hwnd, SW_HIDE);
            }
            self.is_visible = false;
        }
    }

    fn set_colorkey_mode(&mut self, enable: bool, shape: &str) {
        if self.is_colorkey_mode != enable {
            self.is_colorkey_mode = enable;
            unsafe {
                if enable {
                    SetLayeredWindowAttributes(
                        self.hwnd,
                        TRANSPARENT_COLOR_KEY,
                        255,
                        LWA_COLORKEY,
                    );
                } else {
                    SetLayeredWindowAttributes(self.hwnd, 0, 255, LWA_ALPHA);
                    apply_shape_region(self.hwnd, self.current_w, self.current_h, shape);
                }
            }
        }
    }

    pub fn update_geometry(&mut self, new_size: i32, shape: &str, target_cx: i32, target_cy: i32) {
        let (new_w, new_h) = compute_dimensions(new_size, shape, self.screen_w, self.screen_h);
        let win_x = if shape.eq_ignore_ascii_case("fullscreen") {
            0
        } else {
            target_cx - new_w / 2
        };
        let win_y = if shape.eq_ignore_ascii_case("fullscreen") {
            0
        } else {
            target_cy - new_h / 2
        };

        let size_changed = new_w != self.current_w || new_h != self.current_h;
        let pos_changed = win_x != self.last_win_x || win_y != self.last_win_y;

        if size_changed || pos_changed {
            self.current_w = new_w;
            self.current_h = new_h;
            self.last_win_x = win_x;
            self.last_win_y = win_y;

            unsafe {
                SetWindowPos(
                    self.hwnd,
                    HWND_TOPMOST,
                    win_x,
                    win_y,
                    new_w,
                    new_h,
                    SWP_NOACTIVATE | SWP_NOZORDER,
                );

                if size_changed {
                    apply_shape_region(self.hwnd, new_w, new_h, shape);

                    let new_bitmap = CreateCompatibleBitmap(self.screen_dc, new_w, new_h);
                    SelectObject(self.mem_dc, new_bitmap);
                    DeleteObject(self.bitmap);
                    self.bitmap = new_bitmap;
                }
            }
        }
    }

    pub fn force_region_update(&self, shape: &str) {
        unsafe {
            apply_shape_region(self.hwnd, self.current_w, self.current_h, shape);
        }
    }

    /// Renders ONLY the transparent hardware crosshair overlay WITHOUT zooming/screen capture.
    pub fn render_crosshair_only(&mut self, cfg: &Config, osd_text: Option<&str>) {
        self.set_colorkey_mode(true, &cfg.shape);

        unsafe {
            let (target_cx, target_cy) = if cfg.follow_mouse {
                let mut pt = POINT {
                    x: self.screen_w / 2,
                    y: self.screen_h / 2,
                };
                GetCursorPos(&mut pt);
                (pt.x + cfg.offset_x, pt.y + cfg.offset_y)
            } else {
                (
                    self.screen_w / 2 + cfg.offset_x,
                    self.screen_h / 2 + cfg.offset_y,
                )
            };

            self.update_geometry(cfg.scope_size, &cfg.shape, target_cx, target_cy);

            let w = self.current_w;
            let h = self.current_h;

            // Fill background with transparent color key
            let key_brush = CreateSolidBrush(TRANSPARENT_COLOR_KEY);
            let rect = RECT {
                left: 0,
                top: 0,
                right: w,
                bottom: h,
            };
            FillRect(self.mem_dc, &rect, key_brush);
            DeleteObject(key_brush);

            // Ensure a visible reticle even if config had "none" with no active layers
            if cfg.crosshair.eq_ignore_ascii_case("none") && !cfg.has_any_xhair_layer() {
                let mut fallback = cfg.clone();
                fallback.apply_crosshair_style("cross");
                draw_crosshair(self.mem_dc, w, h, &fallback, false);
            } else {
                draw_crosshair(self.mem_dc, w, h, cfg, false);
            }

            let rgb = cfg.parse_color_rgb();
            // Show OSD notification if user changed a setting on the fly
            if osd_text.is_some() {
                draw_hud_and_osd(
                    self.mem_dc,
                    self.hud_font,
                    w,
                    h,
                    1.0,
                    false,
                    osd_text,
                    rgb,
                );
            }

            let win_dc = GetDC(self.hwnd);
            BitBlt(win_dc, 0, 0, w, h, self.mem_dc, 0, 0, SRCCOPY);
            ReleaseDC(self.hwnd, win_dc);
        }
    }

    /// Renders the optical zoom scope + bezel + reticle + HUD.
    pub fn render_frame(&mut self, cfg: &Config, osd_text: Option<&str>) {
        self.set_colorkey_mode(false, &cfg.shape);

        unsafe {
            let (target_cx, target_cy) = if cfg.follow_mouse {
                let mut pt = POINT { x: self.screen_w / 2, y: self.screen_h / 2 };
                GetCursorPos(&mut pt);
                (pt.x + cfg.offset_x, pt.y + cfg.offset_y)
            } else {
                (self.screen_w / 2 + cfg.offset_x, self.screen_h / 2 + cfg.offset_y)
            };

            self.update_geometry(cfg.scope_size, &cfg.shape, target_cx, target_cy);

            let w = self.current_w;
            let h = self.current_h;
            let zoom = cfg.zoom_factor.max(1.05);

            let src_w = ((w as f32) / zoom).round().max(16.0) as i32;
            let src_h = ((h as f32) / zoom).round().max(16.0) as i32;

            let src_x = (target_cx - src_w / 2).clamp(0, (self.screen_w - src_w).max(0));
            let src_y = (target_cy - src_h / 2).clamp(0, (self.screen_h - src_h).max(0));

            let smooth = cfg.scaling.eq_ignore_ascii_case("smooth");
            if smooth {
                SetStretchBltMode(self.mem_dc, HALFTONE as i32);
                SetBrushOrgEx(self.mem_dc, 0, 0, null_mut());
            } else {
                SetStretchBltMode(self.mem_dc, COLORONCOLOR as i32);
            }

            // Capture and scale the area under the screen
            StretchBlt(
                self.mem_dc,
                0,
                0,
                w,
                h,
                self.screen_dc,
                src_x,
                src_y,
                src_w,
                src_h,
                SRCCOPY,
            );

            let rgb = cfg.parse_color_rgb();

            // Draw scope bezel / borders
            draw_scope_bezel(self.mem_dc, w, h, &cfg.shape, &cfg.border_style, rgb);

            // Draw reticle (suppressed if hide_crosshair_on_zoom is enabled)
            draw_crosshair(self.mem_dc, w, h, cfg, true);

            // Draw tactical HUD & OSD notifications
            if cfg.show_hud || osd_text.is_some() {
                draw_hud_and_osd(
                    self.mem_dc,
                    self.hud_font,
                    w,
                    h,
                    cfg.zoom_factor,
                    cfg.show_hud,
                    osd_text,
                    rgb,
                );
            }

            // Blit buffer directly to window
            let win_dc = GetDC(self.hwnd);
            BitBlt(win_dc, 0, 0, w, h, self.mem_dc, 0, 0, SRCCOPY);
            ReleaseDC(self.hwnd, win_dc);
        }
    }
}

impl Drop for Overlay {
    fn drop(&mut self) {
        unsafe {
            ShowWindow(self.hwnd, SW_HIDE);
            SelectObject(self.mem_dc, self.old_bitmap);
            DeleteObject(self.bitmap);
            DeleteObject(self.hud_font);
            DeleteDC(self.mem_dc);
            ReleaseDC(0 as HWND, self.screen_dc);
            DestroyWindow(self.hwnd);
        }
    }
}

unsafe fn apply_shape_region(hwnd: HWND, w: i32, h: i32, shape: &str) {
    unsafe {
        let rgn = if shape.eq_ignore_ascii_case("fullscreen") {
            CreateRectRgn(0, 0, w, h)
        } else if shape.eq_ignore_ascii_case("square") || shape.eq_ignore_ascii_case("wide") {
            CreateRoundRectRgn(0, 0, w, h, 28, 28)
        } else {
            CreateEllipticRgn(0, 0, w, h)
        };
        SetWindowRgn(hwnd, rgn, 1);
    }
}

unsafe fn draw_scope_bezel(
    hdc: HDC,
    w: i32,
    h: i32,
    shape: &str,
    border_style: &str,
    (r, g, b): (u8, u8, u8),
) {
    if border_style.eq_ignore_ascii_case("none") || shape.eq_ignore_ascii_case("fullscreen") {
        return;
    }

    unsafe {
        let null_brush = GetStockObject(NULL_BRUSH as i32);
        let old_brush = SelectObject(hdc, null_brush);

        let is_round_rect =
            shape.eq_ignore_ascii_case("square") || shape.eq_ignore_ascii_case("wide");
        let color_ref = (r as u32) | ((g as u32) << 8) | ((b as u32) << 16);

        match border_style.to_lowercase().as_str() {
            "neon" => {
                let outer_pen = CreatePen(PS_SOLID as i32, 4, 0x101010);
                let old_pen = SelectObject(hdc, outer_pen);
                if is_round_rect {
                    RoundRect(hdc, 2, 2, w - 2, h - 2, 28, 28);
                } else {
                    Ellipse(hdc, 2, 2, w - 2, h - 2);
                }
                SelectObject(hdc, old_pen);
                DeleteObject(outer_pen);

                let neon_pen = CreatePen(PS_SOLID as i32, 2, color_ref);
                let old_pen = SelectObject(hdc, neon_pen);
                if is_round_rect {
                    RoundRect(hdc, 4, 4, w - 4, h - 4, 24, 24);
                } else {
                    Ellipse(hdc, 4, 4, w - 4, h - 4);
                }
                SelectObject(hdc, old_pen);
                DeleteObject(neon_pen);
            }
            "thin" => {
                let thin_pen = CreatePen(PS_SOLID as i32, 2, 0x2A2A2A);
                let old_pen = SelectObject(hdc, thin_pen);
                if is_round_rect {
                    RoundRect(hdc, 1, 1, w - 1, h - 1, 26, 26);
                } else {
                    Ellipse(hdc, 1, 1, w - 1, h - 1);
                }
                SelectObject(hdc, old_pen);
                DeleteObject(thin_pen);
            }
            _ => {
                // Default "tactical"
                let dark_pen = CreatePen(PS_SOLID as i32, 5, 0x121417);
                let old_pen = SelectObject(hdc, dark_pen);
                if is_round_rect {
                    RoundRect(hdc, 2, 2, w - 2, h - 2, 28, 28);
                } else {
                    Ellipse(hdc, 2, 2, w - 2, h - 2);
                }
                SelectObject(hdc, old_pen);
                DeleteObject(dark_pen);

                let accent_pen = CreatePen(PS_SOLID as i32, 1, 0x424852);
                let old_pen = SelectObject(hdc, accent_pen);
                if is_round_rect {
                    RoundRect(hdc, 5, 5, w - 5, h - 5, 22, 22);
                } else {
                    Ellipse(hdc, 5, 5, w - 5, h - 5);
                }
                SelectObject(hdc, old_pen);
                DeleteObject(accent_pen);
            }
        }

        SelectObject(hdc, old_brush);
    }
}

fn safe_color_ref((r, g, b): (u8, u8, u8)) -> u32 {
    // Avoid exact match with TRANSPARENT_COLOR_KEY (R=1, G=2, B=1 -> 0x00010201)
    let safe_g = if r == 1 && g == 2 && b == 1 { 3 } else { g };
    (r as u32) | ((safe_g as u32) << 8) | ((b as u32) << 16)
}

fn sc(val: i32, scale: f32) -> i32 {
    ((val as f32) * scale).round() as i32
}

unsafe fn draw_line_with_shadow(
    hdc: HDC,
    x1: i32,
    y1: i32,
    x2: i32,
    y2: i32,
    use_outline: bool,
    shadow_pen: HPEN,
    color_pen: HPEN,
) {
    unsafe {
        if use_outline {
            SelectObject(hdc, shadow_pen);
            MoveToEx(hdc, x1, y1, null_mut());
            LineTo(hdc, x2, y2);
        }
        SelectObject(hdc, color_pen);
        MoveToEx(hdc, x1, y1, null_mut());
        LineTo(hdc, x2, y2);
    }
}

unsafe fn fill_rect_box(hdc: HDC, l: i32, t: i32, r: i32, b: i32, brush: HBRUSH) {
    if l < r && t < b {
        let rc = RECT {
            left: l,
            top: t,
            right: r,
            bottom: b,
        };
        unsafe {
            FillRect(hdc, &rc, brush);
        }
    }
}

unsafe fn draw_rects_layered(
    hdc: HDC,
    rects: &[(i32, i32, i32, i32)],
    use_outline: bool,
    out_th: i32,
    out_brush: HBRUSH,
    fill_brush: HBRUSH,
) {
    if use_outline {
        for &(l, t, r, b) in rects {
            if l < r && t < b {
                unsafe {
                    fill_rect_box(hdc, l - out_th, t - out_th, r + out_th, b + out_th, out_brush);
                }
            }
        }
    }
    for &(l, t, r, b) in rects {
        unsafe {
            fill_rect_box(hdc, l, t, r, b, fill_brush);
        }
    }
}

unsafe fn draw_crosshair(hdc: HDC, w: i32, h: i32, cfg: &Config, is_zoomed: bool) {
    if is_zoomed && cfg.hide_crosshair_on_zoom {
        return;
    }

    if cfg.crosshair.eq_ignore_ascii_case("none") && !cfg.has_any_xhair_layer() {
        return;
    }

    unsafe {
        let cx = w / 2;
        let cy = h / 2;
        let scale = if is_zoomed && cfg.xhair_scale_with_zoom {
            cfg.zoom_factor.clamp(1.0, 4.0)
        } else {
            1.0
        };

        let primary_ref = safe_color_ref(cfg.parse_color_rgb());
        let secondary_ref = safe_color_ref(cfg.parse_secondary_color_rgb());
        let outline_ref = safe_color_ref(cfg.parse_outline_color_rgb());

        let use_outline = cfg.xhair_outline;
        let out_th = cfg.xhair_outline_thickness.clamp(1, 4);
        let th = sc(cfg.crosshair_size, scale).max(1);

        let pen = CreatePen(PS_SOLID as i32, th, primary_ref);
        let shadow_pen = CreatePen(PS_SOLID as i32, th + out_th * 2, outline_ref);
        let thin_pen = CreatePen(PS_SOLID as i32, 1, primary_ref);
        let thin_shadow_pen = CreatePen(PS_SOLID as i32, 1 + out_th * 2, outline_ref);

        let c_th = sc(cfg.xhair_circle_thickness, scale).max(1);
        let circle_pen = CreatePen(PS_SOLID as i32, c_th, primary_ref);
        let circle_shadow_pen = CreatePen(PS_SOLID as i32, c_th + out_th * 2, outline_ref);

        let primary_brush = CreateSolidBrush(primary_ref);
        let dot_brush = CreateSolidBrush(secondary_ref);
        let out_brush = CreateSolidBrush(outline_ref);

        let old_pen = SelectObject(hdc, pen);
        let old_brush = SelectObject(hdc, primary_brush);

        // 1. Outer Collimator Circle
        if cfg.xhair_show_circle {
            let radius = sc(cfg.xhair_circle_radius, scale).max(4);
            let null_brush = GetStockObject(NULL_BRUSH as i32);
            SelectObject(hdc, null_brush);

            if use_outline {
                SelectObject(hdc, circle_shadow_pen);
                Ellipse(hdc, cx - radius, cy - radius, cx + radius, cy + radius);
            }
            SelectObject(hdc, circle_pen);
            Ellipse(hdc, cx - radius, cy - radius, cx + radius, cy + radius);

            if cfg.crosshair.eq_ignore_ascii_case("circle_dot") {
                let tick_len = sc(6, scale).max(3);
                let half_cth = c_th / 2;
                let compass = [
                    (cx - radius - tick_len, cy - half_cth, cx - radius, cy - half_cth + c_th),
                    (cx + radius, cy - half_cth, cx + radius + tick_len, cy - half_cth + c_th),
                    (cx - half_cth, cy - radius - tick_len, cx - half_cth + c_th, cy - radius),
                    (cx - half_cth, cy + radius, cx - half_cth + c_th, cy + radius + tick_len),
                ];
                draw_rects_layered(hdc, &compass, use_outline, out_th, out_brush, primary_brush);
            }
        }

        // 2. Tactical Corner Brackets [ ]
        if cfg.xhair_corner_brackets {
            let b_dist = sc(cfg.xhair_bracket_dist, scale).max(8);
            let b_size = sc(cfg.xhair_bracket_size, scale).clamp(4, b_dist);
            let brackets = [
                // Top-left
                (cx - b_dist, cy - b_dist, cx - b_dist + b_size, cy - b_dist + th),
                (cx - b_dist, cy - b_dist, cx - b_dist + th, cy - b_dist + b_size),
                // Top-right
                (cx + b_dist - b_size, cy - b_dist, cx + b_dist, cy - b_dist + th),
                (cx + b_dist - th, cy - b_dist, cx + b_dist, cy - b_dist + b_size),
                // Bottom-left
                (cx - b_dist, cy + b_dist - th, cx - b_dist + b_size, cy + b_dist),
                (cx - b_dist, cy + b_dist - b_size, cx - b_dist + th, cy + b_dist),
                // Bottom-right
                (cx + b_dist - b_size, cy + b_dist - th, cx + b_dist, cy + b_dist),
                (cx + b_dist - th, cy + b_dist - b_size, cx + b_dist, cy + b_dist),
            ];
            draw_rects_layered(hdc, &brackets, use_outline, out_th, out_brush, primary_brush);
        }

        // 3. Specialized Reticle Elements: Chevron (PSO-1/ACOG) & Mil-Dot
        let is_mildot = cfg.crosshair.eq_ignore_ascii_case("mildot");
        if cfg.crosshair.eq_ignore_ascii_case("chevron") {
            let chev_w = sc(cfg.crosshair_length.clamp(6, 36), scale).max(6);
            let chev_h = chev_w;

            draw_line_with_shadow(hdc, cx, cy, cx - chev_w, cy + chev_h, use_outline, shadow_pen, pen);
            draw_line_with_shadow(hdc, cx, cy, cx + chev_w, cy + chev_h, use_outline, shadow_pen, pen);

            let wing_gap = chev_w + sc(14, scale);
            let wing_len = sc(24, scale);
            draw_line_with_shadow(
                hdc,
                cx - wing_gap - wing_len,
                cy,
                cx - wing_gap,
                cy,
                use_outline,
                thin_shadow_pen,
                thin_pen,
            );
            draw_line_with_shadow(
                hdc,
                cx + wing_gap,
                cy,
                cx + wing_gap + wing_len,
                cy,
                use_outline,
                thin_shadow_pen,
                thin_pen,
            );

            let bdc_top = cy + chev_h + sc(10, scale);
            let bdc_bottom = (cy + chev_h + sc(75, scale)).min(h - 15);
            if bdc_bottom > bdc_top {
                draw_line_with_shadow(hdc, cx, bdc_top, cx, bdc_bottom, use_outline, thin_shadow_pen, thin_pen);
                for step_idx in 0..3 {
                    let ry = bdc_top + step_idx * sc(18, scale);
                    let rw = sc(6 - step_idx, scale).max(2);
                    if ry < bdc_bottom {
                        draw_line_with_shadow(
                            hdc,
                            cx - rw,
                            ry,
                            cx + rw + 1,
                            ry,
                            use_outline,
                            thin_shadow_pen,
                            thin_pen,
                        );
                    }
                }
            }
        } else if is_mildot {
            let half_span = (w.min(h) / 2 - 14).max(40);
            let center_gap = sc(cfg.crosshair_gap.max(3), scale).max(2);

            if cfg.xhair_left_line {
                draw_line_with_shadow(
                    hdc,
                    cx - half_span,
                    cy,
                    cx - center_gap,
                    cy,
                    use_outline,
                    thin_shadow_pen,
                    thin_pen,
                );
            }
            if cfg.xhair_right_line {
                draw_line_with_shadow(
                    hdc,
                    cx + center_gap,
                    cy,
                    cx + half_span,
                    cy,
                    use_outline,
                    thin_shadow_pen,
                    thin_pen,
                );
            }
            if cfg.xhair_top_line {
                draw_line_with_shadow(
                    hdc,
                    cx,
                    cy - half_span,
                    cx,
                    cy - center_gap,
                    use_outline,
                    thin_shadow_pen,
                    thin_pen,
                );
            }
            if cfg.xhair_bottom_line {
                draw_line_with_shadow(
                    hdc,
                    cx,
                    cy + center_gap,
                    cx,
                    cy + half_span,
                    use_outline,
                    thin_shadow_pen,
                    thin_pen,
                );
            }

            let post_start = (half_span * 58) / 100;
            if cfg.xhair_left_line {
                draw_line_with_shadow(hdc, cx - half_span, cy, cx - post_start, cy, use_outline, shadow_pen, pen);
            }
            if cfg.xhair_right_line {
                draw_line_with_shadow(hdc, cx + post_start, cy, cx + half_span, cy, use_outline, shadow_pen, pen);
            }
            if cfg.xhair_bottom_line {
                draw_line_with_shadow(hdc, cx, cy + post_start, cx, cy + half_span, use_outline, shadow_pen, pen);
            }

            let step = (half_span / 6).clamp(16, 36);
            let mil_r = (th / 2 + 1).max(2);

            for i in 1..=4 {
                let offset = i * step;
                if offset < post_start - 4 {
                    let mut pts = Vec::with_capacity(4);
                    if cfg.xhair_left_line {
                        pts.push((cx - offset, cy));
                    }
                    if cfg.xhair_right_line {
                        pts.push((cx + offset, cy));
                    }
                    if cfg.xhair_top_line {
                        pts.push((cx, cy - offset));
                    }
                    if cfg.xhair_bottom_line {
                        pts.push((cx, cy + offset));
                    }
                    for (mx, my) in pts {
                        SelectObject(hdc, GetStockObject(NULL_PEN as i32));
                        if use_outline {
                            SelectObject(hdc, out_brush);
                            Ellipse(
                                hdc,
                                mx - mil_r - out_th,
                                my - mil_r - out_th,
                                mx + mil_r + 1 + out_th,
                                my + mil_r + 1 + out_th,
                            );
                        }
                        SelectObject(hdc, primary_brush);
                        Ellipse(hdc, mx - mil_r, my - mil_r, mx + mil_r + 1, my + mil_r + 1);
                    }
                }
            }
        }

        // 4. Orthogonal & Diagonal Crosshair Lines
        let gap = sc(cfg.crosshair_gap, scale);
        let h_len = sc(cfg.crosshair_length, scale).max(2);
        let v_len = if cfg.xhair_unlink_axis {
            sc(cfg.xhair_v_length, scale).max(2)
        } else {
            h_len
        };

        if cfg.xhair_show_lines {
            if !is_mildot {
                let half_th = th / 2;
                let mut arms: Vec<(i32, i32, i32, i32)> = Vec::with_capacity(4);
                if cfg.xhair_left_line {
                    arms.push((cx - gap - h_len, cy - half_th, cx - gap, cy - half_th + th));
                }
                if cfg.xhair_right_line {
                    arms.push((cx + gap, cy - half_th, cx + gap + h_len, cy - half_th + th));
                }
                if cfg.xhair_top_line {
                    arms.push((cx - half_th, cy - gap - v_len, cx - half_th + th, cy - gap));
                }
                if cfg.xhair_bottom_line {
                    arms.push((cx - half_th, cy + gap, cx - half_th + th, cy + gap + v_len));
                }
                draw_rects_layered(hdc, &arms, use_outline, out_th, out_brush, primary_brush);
            }

            if cfg.xhair_diagonal {
                let d_start = ((gap.max(0) as f32) * 0.7071).round() as i32;
                let d_end = (((gap.max(0) + h_len) as f32) * 0.7071).round() as i32;
                for (sx, sy) in [(-1, -1), (1, -1), (-1, 1), (1, 1)] {
                    draw_line_with_shadow(
                        hdc,
                        cx + sx * d_start,
                        cy + sy * d_start,
                        cx + sx * d_end,
                        cy + sy * d_end,
                        use_outline,
                        shadow_pen,
                        pen,
                    );
                }
            }
        }

        // 5. Ballistic Axis Ticks
        if cfg.xhair_ticks {
            let count = cfg.xhair_tick_count.clamp(1, 6);
            let step = sc(cfg.xhair_tick_step, scale).max(5);
            let tick_th = (th / 2).max(1);
            let half_tth = tick_th / 2;
            let base_gap = gap.max(4);
            let mut ticks = Vec::new();

            let has_dir_lines = cfg.xhair_show_lines
                && (cfg.xhair_top_line || cfg.xhair_bottom_line || cfg.xhair_left_line || cfg.xhair_right_line);

            for i in 1..=count {
                let d = base_gap + i * step;
                let rw = sc((6 - i).max(3), scale).max(2);
                if cfg.xhair_bottom_line || !has_dir_lines {
                    ticks.push((cx - rw, cy + d - half_tth, cx + rw + 1, cy + d - half_tth + tick_th));
                }
                if cfg.xhair_top_line && has_dir_lines {
                    ticks.push((cx - rw, cy - d - half_tth, cx + rw + 1, cy - d - half_tth + tick_th));
                }
                if cfg.xhair_left_line || !has_dir_lines {
                    ticks.push((cx - d - half_tth, cy - rw, cx - d - half_tth + tick_th, cy + rw + 1));
                }
                if cfg.xhair_right_line || !has_dir_lines {
                    ticks.push((cx + d - half_tth, cy - rw, cx + d - half_tth + tick_th, cy + rw + 1));
                }
            }
            draw_rects_layered(hdc, &ticks, use_outline, out_th, out_brush, primary_brush);
        }

        // 6. Center Dot (Circle or Square)
        if cfg.xhair_center_dot {
            let dot_r = sc(cfg.xhair_dot_size, scale).max(1);
            if cfg.xhair_dot_shape.eq_ignore_ascii_case("square") {
                let dot_rect = [(cx - dot_r, cy - dot_r, cx + dot_r + 1, cy + dot_r + 1)];
                draw_rects_layered(hdc, &dot_rect, use_outline, out_th, out_brush, dot_brush);
            } else {
                SelectObject(hdc, GetStockObject(NULL_PEN as i32));
                if use_outline {
                    SelectObject(hdc, out_brush);
                    Ellipse(
                        hdc,
                        cx - dot_r - out_th,
                        cy - dot_r - out_th,
                        cx + dot_r + 1 + out_th,
                        cy + dot_r + 1 + out_th,
                    );
                }
                SelectObject(hdc, dot_brush);
                Ellipse(hdc, cx - dot_r, cy - dot_r, cx + dot_r + 1, cy + dot_r + 1);
            }
        }

        SelectObject(hdc, old_pen);
        SelectObject(hdc, old_brush);
        DeleteObject(pen);
        DeleteObject(shadow_pen);
        DeleteObject(thin_pen);
        DeleteObject(thin_shadow_pen);
        DeleteObject(circle_pen);
        DeleteObject(circle_shadow_pen);
        DeleteObject(primary_brush);
        DeleteObject(dot_brush);
        DeleteObject(out_brush);
    }
}

unsafe fn draw_hud_and_osd(
    hdc: HDC,
    font: HFONT,
    w: i32,
    h: i32,
    zoom_factor: f32,
    show_hud: bool,
    osd_text: Option<&str>,
    (r, g, b): (u8, u8, u8),
) {
    unsafe {
        let old_font = SelectObject(hdc, font);
        let color_ref = (r as u32) | ((g as u32) << 8) | ((b as u32) << 16);

        // 1. Bottom telemetry badge (e.g., "2.5X")
        if show_hud && h >= 180 {
            let label = format!(" {:.1}X ", zoom_factor);
            let wide: Vec<u16> = label.encode_utf16().collect();

            SetBkMode(hdc, OPAQUE as i32);
            SetBkColor(hdc, 0x161210);
            SetTextColor(hdc, color_ref);

            let approx_w = (wide.len() as i32) * 8;
            let tx = w / 2 - approx_w / 2;
            let ty = h - 28;
            TextOutW(hdc, tx, ty, wide.as_ptr(), wide.len() as i32);
        }

        // 2. Top OSD notification pill when changing settings on-the-fly
        if let Some(msg) = osd_text {
            let padded = format!("  {}  ", msg);
            let wide: Vec<u16> = padded.encode_utf16().collect();

            SetBkMode(hdc, OPAQUE as i32);
            SetBkColor(hdc, 0x181410);
            SetTextColor(hdc, 0xFFFF00);

            let approx_w = (wide.len() as i32) * 8;
            let tx = (w / 2 - approx_w / 2).max(10);
            let ty = 22.min(h / 4);
            TextOutW(hdc, tx, ty, wide.as_ptr(), wide.len() as i32);
        }

        SelectObject(hdc, old_font);
    }
}
