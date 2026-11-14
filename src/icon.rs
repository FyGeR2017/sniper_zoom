use eframe::egui;
use std::ptr::null_mut;
use windows_sys::Win32::Graphics::Gdi::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

/// Generates RGBA pixels (size x size) for the Sniper Zoom icon.
/// If `paused` is true, renders an amber/grey theme instead of neon cyan/green.
pub fn generate_icon_rgba(size: u32, paused: bool) -> Vec<u8> {
    let mut rgba = vec![0u8; (size * size * 4) as usize];
    let center = (size as f32 - 1.0) / 2.0;
    let max_r = size as f32 / 2.0 - 1.5;

    let (ring_r, ring_g, ring_b) = if paused {
        (255u8, 170u8, 0u8) // Amber when paused
    } else {
        (0u8, 229u8, 255u8) // Neon cyan when active/ready
    };

    let (dot_r, dot_g, dot_b) = if paused {
        (255u8, 120u8, 0u8)
    } else {
        (0u8, 255u8, 128u8) // Bright tactical green center
    };

    for y in 0..size {
        for x in 0..size {
            let dx = x as f32 - center;
            let dy = y as f32 - center;
            let dist = (dx * dx + dy * dy).sqrt();
            let idx = ((y * size + x) * 4) as usize;

            if dist <= max_r + 0.8 {
                // Smooth outer edge alpha
                let edge_alpha = ((max_r + 0.8 - dist).clamp(0.0, 1.0) * 255.0) as u8;

                // Dark tactical lens background
                let mut r = 16u8;
                let mut g = 22u8;
                let mut b = 30u8;
                let a = edge_alpha;

                // Subtle radial lens sheen
                let norm_d = (dist / max_r).clamp(0.0, 1.0);
                r = (r as f32 + (1.0 - norm_d) * 14.0) as u8;
                g = (g as f32 + (1.0 - norm_d) * 22.0) as u8;
                b = (b as f32 + (1.0 - norm_d) * 32.0) as u8;

                // Outer glowing ring
                let ring_thick = (size as f32 * 0.085).max(1.8);
                if (dist - (max_r - ring_thick * 0.6)).abs() <= ring_thick * 0.65 {
                    r = ring_r;
                    g = ring_g;
                    b = ring_b;
                }

                // Inner tactical ring
                let inner_r = max_r * 0.58;
                if (dist - inner_r).abs() <= (size as f32 * 0.035).max(0.9) {
                    r = (ring_r as u16 * 2 / 3) as u8;
                    g = (ring_g as u16 * 2 / 3) as u8;
                    b = (ring_b as u16 * 2 / 3) as u8;
                }

                // Crosshair lines with center gap
                let line_half_w = (size as f32 * 0.042).max(0.85);
                let gap = max_r * 0.22;
                let is_h_line = dy.abs() <= line_half_w && dx.abs() >= gap && dist <= max_r * 0.88;
                let is_v_line = dx.abs() <= line_half_w && dy.abs() >= gap && dist <= max_r * 0.88;

                if is_h_line || is_v_line {
                    r = ring_r;
                    g = ring_g;
                    b = ring_b;
                }

                // Center sniper dot
                let dot_radius = (size as f32 * 0.10).max(1.6);
                if dist <= dot_radius {
                    r = dot_r;
                    g = dot_g;
                    b = dot_b;
                }

                rgba[idx] = r;
                rgba[idx + 1] = g;
                rgba[idx + 2] = b;
                rgba[idx + 3] = a;
            }
        }
    }

    rgba
}

/// Creates `egui::IconData` for the application window titlebar and taskbar.
pub fn create_egui_icon() -> egui::IconData {
    let size = 64;
    let rgba = generate_icon_rgba(size, false);
    egui::IconData {
        rgba,
        width: size,
        height: size,
    }
}

/// Creates a native Windows `HICON` (32x32) for the System Tray.
#[allow(dead_code)]
pub unsafe fn create_tray_hicon(paused: bool) -> HICON {
    let size: i32 = 32;
    let rgba = generate_icon_rgba(size as u32, paused);

    // Convert RGBA to BGRA (premultiplied/standard 32-bit ARGB for Windows icon)
    let mut bgra = vec![0u8; rgba.len()];
    for i in (0..rgba.len()).step_by(4) {
        bgra[i] = rgba[i + 2];     // B
        bgra[i + 1] = rgba[i + 1]; // G
        bgra[i + 2] = rgba[i];     // R
        bgra[i + 3] = rgba[i + 3]; // A
    }

    unsafe {
        let screen_dc = GetDC(null_mut());
        let color_bmp = CreateCompatibleBitmap(screen_dc, size, size);
        let mask_bmp = CreateBitmap(size, size, 1, 1, null_mut());

        let bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: size,
                biHeight: -size, // top-down DIB
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB,
                biSizeImage: 0,
                biXPelsPerMeter: 0,
                biYPelsPerMeter: 0,
                biClrUsed: 0,
                biClrImportant: 0,
            },
            bmiColors: [RGBQUAD {
                rgbBlue: 0,
                rgbGreen: 0,
                rgbRed: 0,
                rgbReserved: 0,
            }],
        };

        SetDIBits(
            screen_dc,
            color_bmp,
            0,
            size as u32,
            bgra.as_ptr() as *const _,
            &bmi,
            DIB_RGB_COLORS,
        );

        ReleaseDC(null_mut(), screen_dc);

        let icon_info = ICONINFO {
            fIcon: 1,
            xHotspot: 0,
            yHotspot: 0,
            hbmMask: mask_bmp,
            hbmColor: color_bmp,
        };

        let hicon = CreateIconIndirect(&icon_info);
        DeleteObject(color_bmp);
        DeleteObject(mask_bmp);
        hicon
    }
}

/// Generates a valid `.ico` binary file (containing a 64x64 32-bit BGRA icon)
/// for Windows shortcuts (.lnk) and installer files.
#[allow(dead_code)]
pub fn generate_ico_file_bytes() -> Vec<u8> {
    let size: u32 = 64;
    let rgba = generate_icon_rgba(size, false);

    let pixel_bytes_len = (size * size * 4) as usize;
    let mask_row_bytes = ((size + 31) / 32 * 4) as usize;
    let mask_bytes_len = mask_row_bytes * (size as usize);
    let dib_header_size = 40usize;
    let image_data_size = dib_header_size + pixel_bytes_len + mask_bytes_len;

    let mut ico = Vec::with_capacity(22 + image_data_size);

    // ICONDIR (6 bytes)
    ico.extend_from_slice(&0u16.to_le_bytes()); // Reserved
    ico.extend_from_slice(&1u16.to_le_bytes()); // Type: 1 = Icon
    ico.extend_from_slice(&1u16.to_le_bytes()); // Count: 1 image

    // ICONDIRENTRY (16 bytes)
    ico.push(size as u8); // Width (64)
    ico.push(size as u8); // Height (64)
    ico.push(0);          // Color count
    ico.push(0);          // Reserved
    ico.extend_from_slice(&1u16.to_le_bytes()); // Color planes
    ico.extend_from_slice(&32u16.to_le_bytes()); // Bits per pixel
    ico.extend_from_slice(&(image_data_size as u32).to_le_bytes()); // Size of image data
    ico.extend_from_slice(&22u32.to_le_bytes()); // Offset to image data (6 + 16 = 22)

    // BITMAPINFOHEADER (40 bytes)
    ico.extend_from_slice(&40u32.to_le_bytes()); // biSize
    ico.extend_from_slice(&(size as i32).to_le_bytes()); // biWidth
    ico.extend_from_slice(&((size * 2) as i32).to_le_bytes()); // biHeight (doubled for XOR + AND mask)
    ico.extend_from_slice(&1u16.to_le_bytes()); // biPlanes
    ico.extend_from_slice(&32u16.to_le_bytes()); // biBitCount
    ico.extend_from_slice(&0u32.to_le_bytes()); // biCompression (BI_RGB)
    ico.extend_from_slice(&(pixel_bytes_len as u32).to_le_bytes()); // biSizeImage
    ico.extend_from_slice(&0i32.to_le_bytes()); // biXPelsPerMeter
    ico.extend_from_slice(&0i32.to_le_bytes()); // biYPelsPerMeter
    ico.extend_from_slice(&0u32.to_le_bytes()); // biClrUsed
    ico.extend_from_slice(&0u32.to_le_bytes()); // biClrImportant

    // Bottom-up BGRA pixel data
    for y in (0..size).rev() {
        for x in 0..size {
            let idx = ((y * size + x) * 4) as usize;
            let r = rgba[idx];
            let g = rgba[idx + 1];
            let b = rgba[idx + 2];
            let a = rgba[idx + 3];
            ico.push(b);
            ico.push(g);
            ico.push(r);
            ico.push(a);
        }
    }

    // AND mask (all 0s since 32-bit alpha channel is used)
    ico.resize(ico.len() + mask_bytes_len, 0u8);

    ico
}
