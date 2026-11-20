//! Canvas peripheral device for the Intel 8085 microprocessor.
//!
//! Exposes a 2D graphics canvas, game loop timing, and keyboard input over
//! 3 hardware I/O ports:
//! - `0x10`: `CANVAS_CMD_PORT`
//! - `0x11`: `CANVAS_DATA_PORT`
//! - `0x12`: `CANVAS_STATUS_PORT`

use super::Device;
use std::collections::VecDeque;
use std::time::Instant;

pub const DEFAULT_CANVAS_CMD_PORT: u8 = 0x10;
pub const DEFAULT_CANVAS_DATA_PORT: u8 = 0x11;
pub const DEFAULT_CANVAS_STATUS_PORT: u8 = 0x12;

pub const CANVAS_CMD_PORT: u8 = DEFAULT_CANVAS_CMD_PORT;
pub const CANVAS_DATA_PORT: u8 = DEFAULT_CANVAS_DATA_PORT;
pub const CANVAS_STATUS_PORT: u8 = DEFAULT_CANVAS_STATUS_PORT;

// Standard 16-bit RGBA4444 Colors
pub const COLOR_BLACK: u16 = 0x000F;
pub const COLOR_WHITE: u16 = 0xFFFF;
pub const COLOR_RED: u16 = 0xF00F;
pub const COLOR_GREEN: u16 = 0x0F0F;
pub const COLOR_BLUE: u16 = 0x00FF;
pub const COLOR_YELLOW: u16 = 0xFF0F;
pub const COLOR_CYAN: u16 = 0x0FFF;
pub const COLOR_MAGENTA: u16 = 0xF0FF;
pub const COLOR_GRAY: u16 = 0x888F;
pub const COLOR_TRANSPARENT: u16 = 0x0000;

// Key Codes
pub const K_A: u8 = 0x41;
pub const K_B: u8 = 0x42;
pub const K_C: u8 = 0x43;
pub const K_D: u8 = 0x44;
pub const K_E: u8 = 0x45;
pub const K_F: u8 = 0x46;
pub const K_G: u8 = 0x47;
pub const K_H: u8 = 0x48;
pub const K_I: u8 = 0x49;
pub const K_J: u8 = 0x4A;
pub const K_K: u8 = 0x4B;
pub const K_L: u8 = 0x4C;
pub const K_M: u8 = 0x4D;
pub const K_N: u8 = 0x4E;
pub const K_O: u8 = 0x4F;
pub const K_P: u8 = 0x50;
pub const K_Q: u8 = 0x51;
pub const K_R: u8 = 0x52;
pub const K_S: u8 = 0x53;
pub const K_T: u8 = 0x54;
pub const K_U: u8 = 0x55;
pub const K_V: u8 = 0x56;
pub const K_W: u8 = 0x57;
pub const K_X: u8 = 0x58;
pub const K_Y: u8 = 0x59;
pub const K_Z: u8 = 0x5A;
pub const K_0: u8 = 0x30;
pub const K_1: u8 = 0x31;
pub const K_2: u8 = 0x32;
pub const K_3: u8 = 0x33;
pub const K_4: u8 = 0x34;
pub const K_5: u8 = 0x35;
pub const K_6: u8 = 0x36;
pub const K_7: u8 = 0x37;
pub const K_8: u8 = 0x38;
pub const K_9: u8 = 0x39;

// Command Codes
pub const CMD_INIT_CANVAS: u8 = 0x01;
pub const CMD_DISPOSE_CANVAS: u8 = 0x02;
pub const CMD_GET_CANVAS_SIZE: u8 = 0x03;
pub const CMD_GET_CANVAS_TITLE: u8 = 0x05;
pub const CMD_SET_CANVAS_TITLE: u8 = 0x06;
pub const CMD_UPDATE_CANVAS: u8 = 0x07;
pub const CMD_IS_CANVAS_OPEN: u8 = 0x08;
pub const CMD_GET_FPS: u8 = 0x09;
pub const CMD_SET_FPS: u8 = 0x0A;
pub const CMD_GET_DELTA: u8 = 0x0B;

pub const CMD_CLEAR: u8 = 0x10;
pub const CMD_SET_PIXEL: u8 = 0x11;
pub const CMD_GET_PIXEL: u8 = 0x12;
pub const CMD_LINE: u8 = 0x13;
pub const CMD_FILL_RECT: u8 = 0x14;
pub const CMD_STROKE_RECT: u8 = 0x15;
pub const CMD_FILL_ELLIPSE: u8 = 0x16;
pub const CMD_STROKE_ELLIPSE: u8 = 0x17;
pub const CMD_PAINT_MASK: u8 = 0x18;

pub const CMD_FILL_TEXT: u8 = 0x20;
pub const CMD_TEXT_WIDTH: u8 = 0x21;
pub const CMD_STROKE_TEXT: u8 = 0x22;

pub const FONT_SM: u8 = 0;
pub const FONT_MD: u8 = 1;
pub const FONT_LG: u8 = 2;

pub const CMD_KEY_PRESS: u8 = 0x30;
pub const CMD_KEY_DOWN: u8 = 0x31;
pub const CMD_KEY_RELEASE: u8 = 0x32;
pub const CMD_KEY_INPUT: u8 = 0x33;

pub const CMD_MOUSE_PRESS: u8 = 0x40;
pub const CMD_MOUSE_DOWN: u8 = 0x41;
pub const CMD_MOUSE_RELEASE: u8 = 0x42;
pub const CMD_MOUSE_POS: u8 = 0x43;

pub const MOUSE_BUTTON_LEFT: u8 = 0;
pub const MOUSE_BUTTON_RIGHT: u8 = 1;
pub const MOUSE_BUTTON_MIDDLE: u8 = 2;

// Status Flags (Port 0x12)
pub const STATUS_WINDOW_OPEN: u8 = 0x01;
pub const STATUS_FRAME_READY: u8 = 0x02;
pub const STATUS_KEY_AVAILABLE: u8 = 0x04;
pub const STATUS_MOUSE_DOWN: u8 = 0x08;

pub const SCALE_FACTOR: usize = 4;

/// Converts 16-bit RGBA4444 color to 32-bit ARGB8888 for framebuffer presentation.
#[inline]
pub fn rgba4444_to_argb8888(c: u16) -> u32 {
    let r4 = ((c >> 12) & 0x0F) as u32;
    let g4 = ((c >> 8) & 0x0F) as u32;
    let b4 = ((c >> 4) & 0x0F) as u32;
    let a4 = (c & 0x0F) as u32;

    let r8 = (r4 << 4) | r4;
    let g8 = (g4 << 4) | g4;
    let b8 = (b4 << 4) | b4;
    let a8 = (a4 << 4) | a4;

    (a8 << 24) | (r8 << 16) | (g8 << 8) | b8
}

pub struct CanvasDevice {
    cmd_port: u8,
    data_port: u8,
    status_port: u8,
    pub logical_width: u8,
    pub logical_height: u8,
    pub logical_buffer: Vec<u16>,
    pub physical_buffer: Vec<u32>,
    pub title: String,
    window: Option<minifb::Window>,
    pub headless: bool,
    pub window_open: bool,

    // Streaming State Machine
    current_cmd: Option<u8>,
    param_bytes: Vec<u8>,
    return_queue: VecDeque<u8>,

    // Timing & GameLoop
    pub target_fps: u8,
    last_frame_instant: Instant,
    pub delta_ms: u8,

    // Keyboard State
    pub key_down: [bool; 256],
    pub key_prev: [bool; 256],
    pub key_queue: VecDeque<u8>,

    // Mouse State
    pub mouse_down: [bool; 3],
    pub mouse_prev: [bool; 3],
    pub mouse_x: u8,
    pub mouse_y: u8,
}

impl Default for CanvasDevice {
    fn default() -> Self {
        Self::new(CANVAS_CMD_PORT, CANVAS_DATA_PORT, CANVAS_STATUS_PORT)
    }
}

impl CanvasDevice {
    pub fn new(cmd_port: u8, data_port: u8, status_port: u8) -> Self {
        Self::create(cmd_port, data_port, status_port, false)
    }

    pub fn new_headless(cmd_port: u8, data_port: u8, status_port: u8) -> Self {
        Self::create(cmd_port, data_port, status_port, true)
    }

    pub fn headless(cmd_port: u8, data_port: u8, status_port: u8) -> Self {
        Self::new_headless(cmd_port, data_port, status_port)
    }

    pub fn is_open(&self) -> bool {
        self.window_open
    }

    pub fn fps(&self) -> u8 {
        self.target_fps
    }

    pub fn logical_width(&self) -> u8 {
        self.logical_width
    }

    pub fn logical_height(&self) -> u8 {
        self.logical_height
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    fn create(cmd_port: u8, data_port: u8, status_port: u8, headless: bool) -> Self {
        CanvasDevice {
            cmd_port,
            data_port,
            status_port,
            logical_width: 160,
            logical_height: 90,
            logical_buffer: vec![0; 160 * 90],
            physical_buffer: vec![0xFF000000; (160 * SCALE_FACTOR) * (90 * SCALE_FACTOR)],
            title: "Canvas".to_string(),
            window: None,
            headless,
            window_open: false,
            current_cmd: None,
            param_bytes: Vec::new(),
            return_queue: VecDeque::new(),
            target_fps: 30,
            last_frame_instant: Instant::now(),
            delta_ms: 0,
            key_down: [false; 256],
            key_prev: [false; 256],
            key_queue: VecDeque::new(),
            mouse_down: [false; 3],
            mouse_prev: [false; 3],
            mouse_x: 0,
            mouse_y: 0,
        }
    }

    pub fn simulate_mouse_down(&mut self, button: u8) {
        if (button as usize) < 3 {
            self.mouse_down[button as usize] = true;
        }
    }

    pub fn simulate_mouse_up(&mut self, button: u8) {
        if (button as usize) < 3 {
            self.mouse_down[button as usize] = false;
        }
    }

    pub fn simulate_mouse_move(&mut self, x: u8, y: u8) {
        self.mouse_x = x;
        self.mouse_y = y;
    }

    pub fn simulate_key_down(&mut self, key_code: u8) {
        self.key_down[key_code as usize] = true;
        self.key_queue.push_back(key_code);
    }

    pub fn simulate_key_up(&mut self, key_code: u8) {
        self.key_down[key_code as usize] = false;
    }

    pub fn get_pixel_value(&self, x: u8, y: u8) -> u16 {
        let w = if self.logical_width == 0 { 256 } else { self.logical_width as usize };
        let h = if self.logical_height == 0 { 256 } else { self.logical_height as usize };
        let (x, y) = (x as usize, y as usize);
        if x < w && y < h {
            let idx = y * w + x;
            if idx < self.logical_buffer.len() {
                return self.logical_buffer[idx];
            }
        }
        0
    }

    pub fn set_pixel_value(&mut self, x: u8, y: u8, color: u16) {
        let w = if self.logical_width == 0 { 256 } else { self.logical_width as usize };
        let h = if self.logical_height == 0 { 256 } else { self.logical_height as usize };
        let (x, y) = (x as usize, y as usize);
        if x < w && y < h {
            let idx = y * w + x;
            if idx < self.logical_buffer.len() {
                self.logical_buffer[idx] = color;
            }
        }
    }

    pub fn clear_color(&mut self, color: u16) {
        self.logical_buffer.fill(color);
    }

    fn init_canvas_internal(&mut self, w: u8, h: u8, title: String) {
        let actual_w = if w == 0 { 256 } else { w as usize };
        let actual_h = if h == 0 { 256 } else { h as usize };
        self.logical_width = w;
        self.logical_height = h;
        self.title = title;
        self.logical_buffer = vec![0; actual_w * actual_h];
        let phys_w = actual_w * SCALE_FACTOR;
        let phys_h = actual_h * SCALE_FACTOR;
        self.physical_buffer = vec![0xFF000000; phys_w * phys_h];
        self.window_open = true;

        if !self.headless {
            let total_pixels = actual_w * actual_h;
            if total_pixels > 200 * 200 {
                eprintln!(
                    "CanvasDevice: opened {}x{} canvas ({} logical pixels, {} physical pixels)",
                    actual_w, actual_h, total_pixels, total_pixels * SCALE_FACTOR * SCALE_FACTOR
                );
            }
            let mut opts = minifb::WindowOptions::default();
            opts.resize = false;
            opts.scale = minifb::Scale::X1;
            match minifb::Window::new(&self.title, phys_w, phys_h, opts) {
                Ok(mut win) => {
                    win.set_target_fps(self.target_fps.max(1) as usize);
                    self.window = Some(win);
                }
                Err(e) => {
                    eprintln!("CanvasDevice warning: failed to open window: {e}");
                    self.headless = true;
                }
            }
        }
    }

    fn dispose_canvas_internal(&mut self) {
        self.window = None;
        self.window_open = false;
        self.logical_buffer.clear();
        self.physical_buffer.clear();
    }

    pub fn update_canvas_internal(&mut self) {
        let lw = if self.logical_width == 0 { 256 } else { self.logical_width as usize };
        let lh = if self.logical_height == 0 { 256 } else { self.logical_height as usize };
        let pw = lw * SCALE_FACTOR;
        let ph = lh * SCALE_FACTOR;

        if self.physical_buffer.len() != pw * ph {
            self.physical_buffer.resize(pw * ph, 0xFF000000);
        }

        // Guard against disposed/empty logical buffer
        if self.logical_buffer.len() < lw * lh {
            return;
        }

        // 4x4 Nearest-Neighbor Upscale
        for ly in 0..lh {
            for lx in 0..lw {
                let color_rgba4444 = self.logical_buffer[ly * lw + lx];
                let argb8888 = rgba4444_to_argb8888(color_rgba4444);
                let py_base = ly * SCALE_FACTOR;
                let px_base = lx * SCALE_FACTOR;

                for dy in 0..SCALE_FACTOR {
                    let row_idx = (py_base + dy) * pw;
                    for dx in 0..SCALE_FACTOR {
                        self.physical_buffer[row_idx + px_base + dx] = argb8888;
                    }
                }
            }
        }

        // Frame timing & FPS pacing
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_frame_instant);
        self.delta_ms = elapsed.as_millis().min(255) as u8;
        self.last_frame_instant = now;

        // Window blit & OS event polling
        if let Some(ref mut win) = self.window {
            if win.is_open() {
                if let Err(e) = win.update_with_buffer(&self.physical_buffer, pw, ph) {
                    eprintln!("CanvasDevice warning: update_with_buffer failed: {e}");
                }
                self.window_open = win.is_open();

                // Poll keys and mouse from minifb
                self.key_prev = self.key_down;
                self.poll_minifb_keys();

                self.mouse_prev = self.mouse_down;
                self.poll_minifb_mouse();
            } else {
                self.window_open = false;
            }
        } else {
            // Headless mode: update previous key and mouse states
            self.key_prev = self.key_down;
            self.mouse_prev = self.mouse_down;
        }
    }

    fn poll_minifb_mouse(&mut self) {
        if let Some(ref win) = self.window {
            self.mouse_down[0] = win.get_mouse_down(minifb::MouseButton::Left);
            self.mouse_down[1] = win.get_mouse_down(minifb::MouseButton::Right);
            self.mouse_down[2] = win.get_mouse_down(minifb::MouseButton::Middle);

            if let Some((mx, my)) = win.get_mouse_pos(minifb::MouseMode::Discard) {
                self.mouse_x = ((mx / SCALE_FACTOR as f32).max(0.0).min(255.0)) as u8;
                self.mouse_y = ((my / SCALE_FACTOR as f32).max(0.0).min(255.0)) as u8;
            }
        }
    }

    fn poll_minifb_keys(&mut self) {
        if let Some(ref win) = self.window {
            // Reset down states for mapped keys
            let mapped_keys = [
                (minifb::Key::A, 0x41), (minifb::Key::B, 0x42), (minifb::Key::C, 0x43),
                (minifb::Key::D, 0x44), (minifb::Key::E, 0x45), (minifb::Key::F, 0x46),
                (minifb::Key::G, 0x47), (minifb::Key::H, 0x48), (minifb::Key::I, 0x49),
                (minifb::Key::J, 0x4A), (minifb::Key::K, 0x4B), (minifb::Key::L, 0x4C),
                (minifb::Key::M, 0x4D), (minifb::Key::N, 0x4E), (minifb::Key::O, 0x4F),
                (minifb::Key::P, 0x50), (minifb::Key::Q, 0x51), (minifb::Key::R, 0x52),
                (minifb::Key::S, 0x53), (minifb::Key::T, 0x54), (minifb::Key::U, 0x55),
                (minifb::Key::V, 0x56), (minifb::Key::W, 0x57), (minifb::Key::X, 0x58),
                (minifb::Key::Y, 0x59), (minifb::Key::Z, 0x5A),
                (minifb::Key::Key0, 0x30), (minifb::Key::Key1, 0x31), (minifb::Key::Key2, 0x32),
                (minifb::Key::Key3, 0x33), (minifb::Key::Key4, 0x34), (minifb::Key::Key5, 0x35),
                (minifb::Key::Key6, 0x36), (minifb::Key::Key7, 0x37), (minifb::Key::Key8, 0x38),
                (minifb::Key::Key9, 0x39),
                (minifb::Key::Up, 0x90), (minifb::Key::Down, 0x91),
                (minifb::Key::Left, 0x92), (minifb::Key::Right, 0x93),
                (minifb::Key::Home, 0x94), (minifb::Key::End, 0x95),
                (minifb::Key::PageUp, 0x96), (minifb::Key::PageDown, 0x97),
                (minifb::Key::Space, 0x20), (minifb::Key::Enter, 0x0D),
                (minifb::Key::Escape, 0x1B), (minifb::Key::Backspace, 0x08),
                (minifb::Key::Tab, 0x09), (minifb::Key::Delete, 0x7F),
                (minifb::Key::LeftShift, 0xA0), (minifb::Key::RightShift, 0xA0),
                (minifb::Key::LeftCtrl, 0xA1), (minifb::Key::RightCtrl, 0xA1),
                (minifb::Key::LeftAlt, 0xA2), (minifb::Key::RightAlt, 0xA2),
                (minifb::Key::CapsLock, 0xA3),
                (minifb::Key::F1, 0x81), (minifb::Key::F2, 0x82), (minifb::Key::F3, 0x83),
                (minifb::Key::F4, 0x84), (minifb::Key::F5, 0x85), (minifb::Key::F6, 0x86),
                (minifb::Key::F7, 0x87), (minifb::Key::F8, 0x88), (minifb::Key::F9, 0x89),
                (minifb::Key::F10, 0x8A), (minifb::Key::F11, 0x8B), (minifb::Key::F12, 0x8C),
                (minifb::Key::Minus, 0x2D), (minifb::Key::Equal, 0x3D),
                (minifb::Key::LeftBracket, 0x5B), (minifb::Key::RightBracket, 0x5D),
                (minifb::Key::Backslash, 0x5C), (minifb::Key::Semicolon, 0x3B),
                (minifb::Key::Apostrophe, 0x27), (minifb::Key::Backquote, 0x60),
                (minifb::Key::Comma, 0x2C), (minifb::Key::Period, 0x2E),
                (minifb::Key::Slash, 0x2F),
            ];

            for (m_key, code) in mapped_keys {
                let is_down = win.is_key_down(m_key);
                self.key_down[code as usize] = is_down;
                if is_down && !self.key_prev[code as usize] {
                    self.key_queue.push_back(code);
                }
            }
        }
    }

    // Drawing Algorithms
    pub fn draw_line(&mut self, x1: u8, y1: u8, x2: u8, y2: u8, color: u16, thickness: u8) {
        let mut x0 = x1 as i32;
        let mut y0 = y1 as i32;
        let x1 = x2 as i32;
        let y1 = y2 as i32;

        let dx = (x1 - x0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let dy = -(y1 - y0).abs();
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;

        let th = thickness.max(1) as i32;
        let half = th / 2;

        loop {
            // Plot center and thickness extent
            for ty in -half..=half {
                for tx in -half..=half {
                    let px = x0 + tx;
                    let py = y0 + ty;
                    if px >= 0 && px < 256 && py >= 0 && py < 256 {
                        self.set_pixel_value(px as u8, py as u8, color);
                    }
                }
            }

            if x0 == x1 && y0 == y1 {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x0 += sx;
            }
            if e2 <= dx {
                err += dx;
                y0 += sy;
            }
        }
    }

    pub fn fill_rect(&mut self, x: u8, y: u8, w: u8, h: u8, color: u16) {
        let lw = if self.logical_width == 0 { 256 } else { self.logical_width as usize };
        let lh = if self.logical_height == 0 { 256 } else { self.logical_height as usize };
        if self.logical_buffer.len() < lw * lh {
            return;
        }
        let (x, y, w, h) = (x as usize, y as usize, w as usize, h as usize);

        for py in y..((y + h).min(lh)) {
            let row_start = py * lw;
            for px in x..((x + w).min(lw)) {
                self.logical_buffer[row_start + px] = color;
            }
        }
    }

    pub fn stroke_rect(&mut self, x: u8, y: u8, w: u8, h: u8, thickness: u8, color: u16) {
        let th = thickness.max(1);
        let (xu, yu, wu, hu, thu) = (x as usize, y as usize, w as usize, h as usize, th as usize);
        // Top and bottom horizontal borders
        self.fill_rect(x, y, w, th, color);
        if hu > thu {
            let y2 = (yu + hu - thu).min(255) as u8;
            self.fill_rect(x, y2, w, th, color);
        }
        // Left and right vertical borders
        self.fill_rect(x, y, th, h, color);
        if wu > thu {
            let x2 = (xu + wu - thu).min(255) as u8;
            self.fill_rect(x2, y, th, h, color);
        }
    }

    pub fn fill_ellipse(&mut self, cx: u8, cy: u8, rx: u8, ry: u8, color: u16) {
        if rx == 0 || ry == 0 {
            return;
        }
        let (cx, cy, rx, ry) = (cx as i64, cy as i64, rx as i64, ry as i64);
        let rx2 = rx * rx;
        let ry2 = ry * ry;

        for dy in -ry..=ry {
            let y = cy + dy;
            if y < 0 || y >= 256 {
                continue;
            }
            // dy^2 / ry^2 + dx^2 / rx^2 <= 1 => dx^2 <= rx^2 * (1 - dy^2 / ry^2)
            let remaining = rx2 - (rx2 * dy * dy) / ry2;
            if remaining >= 0 {
                let dx_max = (remaining as f64).sqrt() as i64;
                for dx in -dx_max..=dx_max {
                    let x = cx + dx;
                    if x >= 0 && x < 256 {
                        self.set_pixel_value(x as u8, y as u8, color);
                    }
                }
            }
        }
    }

    pub fn stroke_ellipse(&mut self, cx: u8, cy: u8, rx: u8, ry: u8, thickness: u8, color: u16) {
        if rx == 0 || ry == 0 {
            return;
        }
        let th = thickness.max(1) as i64;
        let (cx, cy, rx, ry) = (cx as i64, cy as i64, rx as i64, ry as i64);

        let rx_outer = rx + th / 2;
        let ry_outer = ry + th / 2;
        let rx_inner = (rx - th / 2).max(0);
        let ry_inner = (ry - th / 2).max(0);

        let rx_out2 = rx_outer * rx_outer;
        let ry_out2 = ry_outer * ry_outer;
        let rx_in2 = rx_inner * rx_inner;
        let ry_in2 = ry_inner * ry_inner;

        for dy in -ry_outer..=ry_outer {
            let y = cy + dy;
            if y < 0 || y >= 256 {
                continue;
            }
            for dx in -rx_outer..=rx_outer {
                let x = cx + dx;
                if x < 0 || x >= 256 {
                    continue;
                }
                let in_outer = (dx * dx * ry_out2 + dy * dy * rx_out2) <= (rx_out2 * ry_out2);
                let in_inner = if rx_inner > 0 && ry_inner > 0 {
                    (dx * dx * ry_in2 + dy * dy * rx_in2) < (rx_in2 * ry_in2)
                } else {
                    false
                };

                if in_outer && !in_inner {
                    self.set_pixel_value(x as u8, y as u8, color);
                }
            }
        }
    }

    pub fn paint_mask(
        &mut self,
        x: u8,
        y: u8,
        w: u8,
        h: u8,
        mask_value: u8,
        mask_invert: u8,
        color: u16,
        mask_data: &[u8],
    ) {
        if w == 0 || h == 0 {
            return;
        }
        let lw = if self.logical_width == 0 { 256 } else { self.logical_width as usize };
        let lh = if self.logical_height == 0 { 256 } else { self.logical_height as usize };
        if self.logical_buffer.len() < lw * lh {
            return;
        }
        let (x, y, w, h) = (x as usize, y as usize, w as usize, h as usize);

        for dy in 0..h {
            let py = y + dy;
            if py >= lh {
                continue;
            }
            let row_start = py * lw;
            for dx in 0..w {
                let px = x + dx;
                if px >= lw {
                    continue;
                }
                let idx = dy * w + dx;
                if idx >= mask_data.len() {
                    break;
                }
                let val = mask_data[idx];
                if val == 0x00 {
                    // 0x00 is always transparent
                    continue;
                }
                let should_paint = if mask_invert == 0 {
                    val == mask_value
                } else {
                    val != mask_value
                };
                if should_paint {
                    self.logical_buffer[row_start + px] = color;
                }
            }
        }
    }

    pub fn render_text(&mut self, text: &[u8], x: u8, y: u8, size: u8, color: u16, stroke: bool, thickness: u8) {
        let mut cur_x = x as usize;
        let cur_y = y as usize;

        if size == FONT_SM {
            for &b in text {
                if b < 32 || b > 126 {
                    cur_x += 6;
                    continue;
                }
                let glyph = &FONT_5X7[(b - 32) as usize];
                for col_idx in 0..5 {
                    let col_byte = glyph[col_idx];
                    for row_idx in 0..7 {
                        let bit = (col_byte >> row_idx) & 1;
                        if bit == 1 {
                            let px = cur_x + col_idx;
                            let py = cur_y + row_idx;
                            if !stroke {
                                if px < 256 && py < 256 {
                                    self.set_pixel_value(px as u8, py as u8, color);
                                }
                            } else {
                                let th = thickness.max(1) as usize;
                                for sy in 0..th {
                                    for sx in 0..th {
                                        let ox = px + sx;
                                        let oy = py + sy;
                                        if ox < 256 && oy < 256 {
                                            self.set_pixel_value(ox as u8, oy as u8, color);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                cur_x += 6;
            }
        } else {
            let scale = size.max(1) as usize;
            for &b in text {
                if b < 32 || b > 126 {
                    cur_x += 8 * scale;
                    continue;
                }
                let glyph = &FONT_8X8[(b - 32) as usize];

                for (row_idx, &row_byte) in glyph.iter().enumerate() {
                    for col_idx in 0..8 {
                        let bit = (row_byte >> (7 - col_idx)) & 1;
                        if bit == 1 {
                            let px = cur_x + col_idx * scale;
                            let py = cur_y + row_idx * scale;

                            if !stroke {
                                for dy in 0..scale {
                                    for dx in 0..scale {
                                        let fx = px + dx;
                                        let fy = py + dy;
                                        if fx < 256 && fy < 256 {
                                            self.set_pixel_value(fx as u8, fy as u8, color);
                                        }
                                    }
                                }
                            } else {
                                let th = thickness.max(1) as usize;
                                for dy in 0..scale {
                                    for dx in 0..scale {
                                        let fx = px + dx;
                                        let fy = py + dy;
                                        // Stroke outline
                                        for sy in 0..th {
                                            for sx in 0..th {
                                                let ox = fx + sx;
                                                let oy = fy + sy;
                                                if ox < 256 && oy < 256 {
                                                    self.set_pixel_value(ox as u8, oy as u8, color);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                cur_x += 8 * scale;
            }
        }
    }

    fn execute_command(&mut self) {
        let cmd = match self.current_cmd {
            Some(c) => c,
            None => return,
        };

        match cmd {
            CMD_INIT_CANVAS => {
                if self.param_bytes.len() >= 3 {
                    let w = self.param_bytes[0];
                    let h = self.param_bytes[1];
                    let len = self.param_bytes[2] as usize;
                    if self.param_bytes.len() >= 3 + len {
                        let title = String::from_utf8_lossy(&self.param_bytes[3..3 + len]).to_string();
                        self.init_canvas_internal(w, h, title);
                        self.current_cmd = None;
                        self.param_bytes.clear();
                    }
                }
            }
            CMD_DISPOSE_CANVAS => {
                self.dispose_canvas_internal();
                self.current_cmd = None;
                self.param_bytes.clear();
            }
            CMD_GET_CANVAS_SIZE => {
                self.return_queue.push_back(self.logical_width);
                self.return_queue.push_back(self.logical_height);
                self.current_cmd = None;
                self.param_bytes.clear();
            }
            CMD_GET_CANVAS_TITLE => {
                let bytes = self.title.as_bytes();
                let len = bytes.len().min(255);
                self.return_queue.push_back(len as u8);
                for &b in &bytes[..len] {
                    self.return_queue.push_back(b);
                }
                self.current_cmd = None;
                self.param_bytes.clear();
            }
            CMD_SET_CANVAS_TITLE => {
                if !self.param_bytes.is_empty() {
                    let len = self.param_bytes[0] as usize;
                    if self.param_bytes.len() >= 1 + len {
                        self.title = String::from_utf8_lossy(&self.param_bytes[1..1 + len]).to_string();
                        if let Some(ref mut win) = self.window {
                            win.set_title(&self.title);
                        }
                        self.current_cmd = None;
                        self.param_bytes.clear();
                    }
                }
            }
            CMD_UPDATE_CANVAS => {
                self.update_canvas_internal();
                self.current_cmd = None;
                self.param_bytes.clear();
            }
            CMD_IS_CANVAS_OPEN => {
                self.return_queue.push_back(if self.window_open { 1 } else { 0 });
                self.current_cmd = None;
                self.param_bytes.clear();
            }
            CMD_GET_FPS => {
                self.return_queue.push_back(self.target_fps);
                self.current_cmd = None;
                self.param_bytes.clear();
            }
            CMD_SET_FPS => {
                if !self.param_bytes.is_empty() {
                    self.target_fps = self.param_bytes[0].clamp(1, 120);
                    if let Some(ref mut win) = self.window {
                        win.set_target_fps(self.target_fps as usize);
                    }
                    self.current_cmd = None;
                    self.param_bytes.clear();
                }
            }
            CMD_GET_DELTA => {
                self.return_queue.push_back(self.delta_ms);
                self.current_cmd = None;
                self.param_bytes.clear();
            }
            CMD_CLEAR => {
                if self.param_bytes.len() >= 2 {
                    let hi = self.param_bytes[0] as u16;
                    let lo = self.param_bytes[1] as u16;
                    let color = (hi << 8) | lo;
                    self.clear_color(color);
                    self.current_cmd = None;
                    self.param_bytes.clear();
                }
            }
            CMD_SET_PIXEL => {
                if self.param_bytes.len() >= 4 {
                    let x = self.param_bytes[0];
                    let y = self.param_bytes[1];
                    let hi = self.param_bytes[2] as u16;
                    let lo = self.param_bytes[3] as u16;
                    let color = (hi << 8) | lo;
                    self.set_pixel_value(x, y, color);
                    self.current_cmd = None;
                    self.param_bytes.clear();
                }
            }
            CMD_GET_PIXEL => {
                if self.param_bytes.len() >= 2 {
                    let x = self.param_bytes[0];
                    let y = self.param_bytes[1];
                    let color = self.get_pixel_value(x, y);
                    self.return_queue.push_back(((color >> 8) & 0xFF) as u8);
                    self.return_queue.push_back((color & 0xFF) as u8);
                    self.current_cmd = None;
                    self.param_bytes.clear();
                }
            }
            CMD_LINE => {
                if self.param_bytes.len() >= 7 {
                    let x1 = self.param_bytes[0];
                    let y1 = self.param_bytes[1];
                    let x2 = self.param_bytes[2];
                    let y2 = self.param_bytes[3];
                    let th = self.param_bytes[4];
                    let hi = self.param_bytes[5] as u16;
                    let lo = self.param_bytes[6] as u16;
                    let color = (hi << 8) | lo;
                    self.draw_line(x1, y1, x2, y2, color, th);
                    self.current_cmd = None;
                    self.param_bytes.clear();
                }
            }
            CMD_FILL_RECT => {
                if self.param_bytes.len() >= 6 {
                    let x = self.param_bytes[0];
                    let y = self.param_bytes[1];
                    let w = self.param_bytes[2];
                    let h = self.param_bytes[3];
                    let hi = self.param_bytes[4] as u16;
                    let lo = self.param_bytes[5] as u16;
                    let color = (hi << 8) | lo;
                    self.fill_rect(x, y, w, h, color);
                    self.current_cmd = None;
                    self.param_bytes.clear();
                }
            }
            CMD_STROKE_RECT => {
                if self.param_bytes.len() >= 7 {
                    let x = self.param_bytes[0];
                    let y = self.param_bytes[1];
                    let w = self.param_bytes[2];
                    let h = self.param_bytes[3];
                    let th = self.param_bytes[4];
                    let hi = self.param_bytes[5] as u16;
                    let lo = self.param_bytes[6] as u16;
                    let color = (hi << 8) | lo;
                    self.stroke_rect(x, y, w, h, th, color);
                    self.current_cmd = None;
                    self.param_bytes.clear();
                }
            }
            CMD_FILL_ELLIPSE => {
                if self.param_bytes.len() >= 6 {
                    let cx = self.param_bytes[0];
                    let cy = self.param_bytes[1];
                    let rx = self.param_bytes[2];
                    let ry = self.param_bytes[3];
                    let hi = self.param_bytes[4] as u16;
                    let lo = self.param_bytes[5] as u16;
                    let color = (hi << 8) | lo;
                    self.fill_ellipse(cx, cy, rx, ry, color);
                    self.current_cmd = None;
                    self.param_bytes.clear();
                }
            }
            CMD_STROKE_ELLIPSE => {
                if self.param_bytes.len() >= 7 {
                    let cx = self.param_bytes[0];
                    let cy = self.param_bytes[1];
                    let rx = self.param_bytes[2];
                    let ry = self.param_bytes[3];
                    let th = self.param_bytes[4];
                    let hi = self.param_bytes[5] as u16;
                    let lo = self.param_bytes[6] as u16;
                    let color = (hi << 8) | lo;
                    self.stroke_ellipse(cx, cy, rx, ry, th, color);
                    self.current_cmd = None;
                    self.param_bytes.clear();
                }
            }
            CMD_PAINT_MASK => {
                if self.param_bytes.len() >= 8 {
                    let x = self.param_bytes[0];
                    let y = self.param_bytes[1];
                    let w = self.param_bytes[2];
                    let h = self.param_bytes[3];
                    let mask_val = self.param_bytes[4];
                    let invert = self.param_bytes[5];
                    let hi = self.param_bytes[6] as u16;
                    let lo = self.param_bytes[7] as u16;
                    let color = (hi << 8) | lo;
                    let total_bytes = w as usize * h as usize;
                    if self.param_bytes.len() >= 8 + total_bytes {
                        let mask_data = self.param_bytes[8..8 + total_bytes].to_vec();
                        self.paint_mask(x, y, w, h, mask_val, invert, color, &mask_data);
                        self.current_cmd = None;
                        self.param_bytes.clear();
                    }
                }
            }
            CMD_FILL_TEXT => {
                if self.param_bytes.len() >= 6 {
                    let x = self.param_bytes[0];
                    let y = self.param_bytes[1];
                    let sz = self.param_bytes[2];
                    let hi = self.param_bytes[3] as u16;
                    let lo = self.param_bytes[4] as u16;
                    let len = self.param_bytes[5] as usize;
                    if self.param_bytes.len() >= 6 + len {
                        let text = self.param_bytes[6..6 + len].to_vec();
                        let color = (hi << 8) | lo;
                        self.render_text(&text, x, y, sz, color, false, 1);
                        self.current_cmd = None;
                        self.param_bytes.clear();
                    }
                }
            }
            CMD_STROKE_TEXT => {
                if self.param_bytes.len() >= 7 {
                    let x = self.param_bytes[0];
                    let y = self.param_bytes[1];
                    let sz = self.param_bytes[2];
                    let th = self.param_bytes[3];
                    let hi = self.param_bytes[4] as u16;
                    let lo = self.param_bytes[5] as u16;
                    let len = self.param_bytes[6] as usize;
                    if self.param_bytes.len() >= 7 + len {
                        let text = self.param_bytes[7..7 + len].to_vec();
                        let color = (hi << 8) | lo;
                        self.render_text(&text, x, y, sz, color, true, th);
                        self.current_cmd = None;
                        self.param_bytes.clear();
                    }
                }
            }
            CMD_TEXT_WIDTH => {
                if self.param_bytes.len() >= 2 {
                    let sz = self.param_bytes[0];
                    let len = self.param_bytes[1] as usize;
                    if self.param_bytes.len() >= 2 + len {
                        let char_w = match sz {
                            FONT_SM => 6,
                            FONT_MD => 8,
                            FONT_LG => 16,
                            other => (other as usize) * 8,
                        };
                        let width = (len * char_w).min(255) as u8;
                        self.return_queue.push_back(width);
                        self.current_cmd = None;
                        self.param_bytes.clear();
                    }
                }
            }
            CMD_KEY_PRESS => {
                if !self.param_bytes.is_empty() {
                    let code = self.param_bytes[0] as usize;
                    let pressed = self.key_down[code] && !self.key_prev[code];
                    self.return_queue.push_back(if pressed { 1 } else { 0 });
                    self.current_cmd = None;
                    self.param_bytes.clear();
                }
            }
            CMD_KEY_DOWN => {
                if !self.param_bytes.is_empty() {
                    let code = self.param_bytes[0] as usize;
                    let down = self.key_down[code];
                    self.return_queue.push_back(if down { 1 } else { 0 });
                    self.current_cmd = None;
                    self.param_bytes.clear();
                }
            }
            CMD_KEY_RELEASE => {
                if !self.param_bytes.is_empty() {
                    let code = self.param_bytes[0] as usize;
                    let released = !self.key_down[code] && self.key_prev[code];
                    self.return_queue.push_back(if released { 1 } else { 0 });
                    self.current_cmd = None;
                    self.param_bytes.clear();
                }
            }
            CMD_KEY_INPUT => {
                let code = self.key_queue.pop_front().unwrap_or(0);
                self.return_queue.push_back(code);
                self.current_cmd = None;
                self.param_bytes.clear();
            }
            CMD_MOUSE_PRESS => {
                if !self.param_bytes.is_empty() {
                    let btn = self.param_bytes[0] as usize;
                    let pressed = if btn < 3 {
                        self.mouse_down[btn] && !self.mouse_prev[btn]
                    } else {
                        false
                    };
                    self.return_queue.push_back(if pressed { 1 } else { 0 });
                    self.current_cmd = None;
                    self.param_bytes.clear();
                }
            }
            CMD_MOUSE_DOWN => {
                if !self.param_bytes.is_empty() {
                    let btn = self.param_bytes[0] as usize;
                    let down = if btn < 3 {
                        self.mouse_down[btn]
                    } else {
                        false
                    };
                    self.return_queue.push_back(if down { 1 } else { 0 });
                    self.current_cmd = None;
                    self.param_bytes.clear();
                }
            }
            CMD_MOUSE_RELEASE => {
                if !self.param_bytes.is_empty() {
                    let btn = self.param_bytes[0] as usize;
                    let released = if btn < 3 {
                        !self.mouse_down[btn] && self.mouse_prev[btn]
                    } else {
                        false
                    };
                    self.return_queue.push_back(if released { 1 } else { 0 });
                    self.current_cmd = None;
                    self.param_bytes.clear();
                }
            }
            CMD_MOUSE_POS => {
                self.return_queue.push_back(self.mouse_x);
                self.return_queue.push_back(self.mouse_y);
                self.current_cmd = None;
                self.param_bytes.clear();
            }
            _ => {
                self.current_cmd = None;
                self.param_bytes.clear();
            }
        }
    }
}

// Safety: CanvasDevice encapsulates minifb window handle which contains raw pointers
// on macOS, but it is exclusively owned and operated by one thread at a time.
unsafe impl Send for CanvasDevice {}

impl Device for CanvasDevice {
    fn name(&self) -> &str {
        "CanvasDevice"
    }

    fn port_read(&mut self, port: u8) -> u8 {
        if port == self.data_port {
            self.return_queue.pop_front().unwrap_or(0)
        } else if port == self.status_port {
            let mut st = 0u8;
            if self.window_open {
                st |= STATUS_WINDOW_OPEN;
            }
            if !self.key_queue.is_empty() {
                st |= STATUS_KEY_AVAILABLE;
            }
            if self.mouse_down.iter().any(|&b| b) {
                st |= STATUS_MOUSE_DOWN;
            }
            st
        } else {
            0xFF
        }
    }

    fn port_write(&mut self, port: u8, data: u8) {
        if port == self.cmd_port {
            self.current_cmd = Some(data);
            self.param_bytes.clear();
            self.execute_command();
        } else if port == self.data_port {
            if self.current_cmd.is_none() {
                // Ignore stray data port write with no active command
                return;
            }
            const MAX_PARAM_BYTES: usize = 65536 + 16;
            if self.param_bytes.len() >= MAX_PARAM_BYTES {
                eprintln!(
                    "CanvasDevice error: param_bytes overflow (cmd={:02X}, len={}). Dropping command to prevent hang.",
                    self.current_cmd.unwrap_or(0),
                    self.param_bytes.len()
                );
                self.current_cmd = None;
                self.param_bytes.clear();
                return;
            }
            self.param_bytes.push(data);
            self.execute_command();
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

// Embedded 8x8 ASCII Font Table (ASCII 32 ' ' to 126 '~')
#[rustfmt::skip]
static FONT_8X8: [[u8; 8]; 95] = [
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00], // ' ' (32)
    [0x18, 0x3C, 0x3C, 0x18, 0x18, 0x00, 0x18, 0x00], // '!'
    [0x66, 0x66, 0x24, 0x00, 0x00, 0x00, 0x00, 0x00], // '"'
    [0x6C, 0x6C, 0xFE, 0x6C, 0xFE, 0x6C, 0x6C, 0x00], // '#'
    [0x18, 0x3E, 0x60, 0x3C, 0x06, 0x7C, 0x18, 0x00], // '$'
    [0x00, 0x63, 0x66, 0x0C, 0x18, 0x33, 0x63, 0x00], // '%'
    [0x38, 0x6C, 0x38, 0x76, 0xDC, 0xCC, 0x76, 0x00], // '&'
    [0x30, 0x30, 0x60, 0x00, 0x00, 0x00, 0x00, 0x00], // '''
    [0x0C, 0x18, 0x30, 0x30, 0x30, 0x18, 0x0C, 0x00], // '('
    [0x30, 0x18, 0x0C, 0x0C, 0x0C, 0x18, 0x30, 0x00], // ')'
    [0x00, 0x66, 0x3C, 0xFF, 0x3C, 0x66, 0x00, 0x00], // '*'
    [0x00, 0x18, 0x18, 0x7E, 0x18, 0x18, 0x00, 0x00], // '+'
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x18, 0x18, 0x30], // ','
    [0x00, 0x00, 0x00, 0x7E, 0x00, 0x00, 0x00, 0x00], // '-'
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x18, 0x18, 0x00], // '.'
    [0x06, 0x0C, 0x18, 0x30, 0x60, 0xC0, 0x80, 0x00], // '/'
    [0x3C, 0x66, 0x6E, 0x76, 0x66, 0x66, 0x3C, 0x00], // '0' (48)
    [0x18, 0x38, 0x18, 0x18, 0x18, 0x18, 0x7E, 0x00], // '1'
    [0x3C, 0x66, 0x06, 0x0C, 0x18, 0x30, 0x7E, 0x00], // '2'
    [0x3C, 0x66, 0x06, 0x1C, 0x06, 0x66, 0x3C, 0x00], // '3'
    [0x0C, 0x1C, 0x34, 0x64, 0xFE, 0x04, 0x04, 0x00], // '4'
    [0x7E, 0x60, 0x7C, 0x06, 0x06, 0x66, 0x3C, 0x00], // '5'
    [0x1C, 0x30, 0x60, 0x7C, 0x66, 0x66, 0x3C, 0x00], // '6'
    [0x7E, 0x66, 0x0C, 0x18, 0x18, 0x18, 0x18, 0x00], // '7'
    [0x3C, 0x66, 0x66, 0x3C, 0x66, 0x66, 0x3C, 0x00], // '8'
    [0x3C, 0x66, 0x66, 0x3E, 0x06, 0x0C, 0x38, 0x00], // '9'
    [0x00, 0x18, 0x18, 0x00, 0x00, 0x18, 0x18, 0x00], // ':'
    [0x00, 0x18, 0x18, 0x00, 0x00, 0x18, 0x18, 0x30], // ';'
    [0x0E, 0x1C, 0x38, 0x70, 0x38, 0x1C, 0x0E, 0x00], // '<'
    [0x00, 0x00, 0x7E, 0x00, 0x7E, 0x00, 0x00, 0x00], // '='
    [0x70, 0x38, 0x1C, 0x0E, 0x1C, 0x38, 0x70, 0x00], // '>'
    [0x3C, 0x66, 0x06, 0x0C, 0x18, 0x00, 0x18, 0x00], // '?'
    [0x3C, 0x66, 0x6E, 0x6E, 0x60, 0x62, 0x3C, 0x00], // '@'
    [0x18, 0x3C, 0x66, 0x66, 0x7E, 0x66, 0x66, 0x00], // 'A' (65)
    [0x7C, 0x66, 0x66, 0x7C, 0x66, 0x66, 0x7C, 0x00], // 'B'
    [0x3C, 0x66, 0x60, 0x60, 0x60, 0x66, 0x3C, 0x00], // 'C'
    [0x78, 0x6C, 0x66, 0x66, 0x66, 0x6C, 0x78, 0x00], // 'D'
    [0x7E, 0x60, 0x60, 0x7C, 0x60, 0x60, 0x7E, 0x00], // 'E'
    [0x7E, 0x60, 0x60, 0x7C, 0x60, 0x60, 0x60, 0x00], // 'F'
    [0x3C, 0x66, 0x60, 0x6E, 0x66, 0x66, 0x3A, 0x00], // 'G'
    [0x66, 0x66, 0x66, 0x7E, 0x66, 0x66, 0x66, 0x00], // 'H'
    [0x3C, 0x18, 0x18, 0x18, 0x18, 0x18, 0x3C, 0x00], // 'I'
    [0x1E, 0x0C, 0x0C, 0x0C, 0x0C, 0x6C, 0x38, 0x00], // 'J'
    [0x66, 0x6C, 0x78, 0x70, 0x78, 0x6C, 0x66, 0x00], // 'K'
    [0x60, 0x60, 0x60, 0x60, 0x60, 0x60, 0x7E, 0x00], // 'L'
    [0x63, 0x77, 0x7F, 0x6B, 0x63, 0x63, 0x63, 0x00], // 'M'
    [0x66, 0x76, 0x7E, 0x7E, 0x6E, 0x66, 0x66, 0x00], // 'N'
    [0x3C, 0x66, 0x66, 0x66, 0x66, 0x66, 0x3C, 0x00], // 'O'
    [0x7C, 0x66, 0x66, 0x7C, 0x60, 0x60, 0x60, 0x00], // 'P'
    [0x3C, 0x66, 0x66, 0x66, 0x6A, 0x6C, 0x36, 0x00], // 'Q'
    [0x7C, 0x66, 0x66, 0x7C, 0x6C, 0x66, 0x66, 0x00], // 'R'
    [0x3C, 0x66, 0x60, 0x3C, 0x06, 0x66, 0x3C, 0x00], // 'S'
    [0x7E, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x00], // 'T'
    [0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x3C, 0x00], // 'U'
    [0x66, 0x66, 0x66, 0x66, 0x66, 0x3C, 0x18, 0x00], // 'V'
    [0x63, 0x63, 0x63, 0x6B, 0x7F, 0x77, 0x63, 0x00], // 'W'
    [0x66, 0x66, 0x3C, 0x18, 0x3C, 0x66, 0x66, 0x00], // 'X'
    [0x66, 0x66, 0x66, 0x3C, 0x18, 0x18, 0x18, 0x00], // 'Y'
    [0x7E, 0x06, 0x0C, 0x18, 0x30, 0x60, 0x7E, 0x00], // 'Z'
    [0x3C, 0x30, 0x30, 0x30, 0x30, 0x30, 0x3C, 0x00], // '['
    [0xC0, 0x60, 0x30, 0x18, 0x0C, 0x06, 0x02, 0x00], // '\'
    [0x3C, 0x0C, 0x0C, 0x0C, 0x0C, 0x0C, 0x3C, 0x00], // ']'
    [0x18, 0x3C, 0x66, 0x00, 0x00, 0x00, 0x00, 0x00], // '^'
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xFF], // '_'
    [0x30, 0x18, 0x0C, 0x00, 0x00, 0x00, 0x00, 0x00], // '`'
    [0x00, 0x00, 0x3C, 0x06, 0x3E, 0x66, 0x3E, 0x00], // 'a' (97)
    [0x60, 0x60, 0x7C, 0x66, 0x66, 0x66, 0x7C, 0x00], // 'b'
    [0x00, 0x00, 0x3C, 0x66, 0x60, 0x66, 0x3C, 0x00], // 'c'
    [0x06, 0x06, 0x3E, 0x66, 0x66, 0x66, 0x3E, 0x00], // 'd'
    [0x00, 0x00, 0x3C, 0x66, 0x7E, 0x60, 0x3C, 0x00], // 'e'
    [0x0E, 0x18, 0x7E, 0x18, 0x18, 0x18, 0x18, 0x00], // 'f'
    [0x00, 0x00, 0x3E, 0x66, 0x66, 0x3E, 0x06, 0x7C], // 'g'
    [0x60, 0x60, 0x7C, 0x66, 0x66, 0x66, 0x66, 0x00], // 'h'
    [0x18, 0x00, 0x38, 0x18, 0x18, 0x18, 0x3C, 0x00], // 'i'
    [0x06, 0x00, 0x06, 0x06, 0x06, 0x66, 0x3C, 0x00], // 'j'
    [0x60, 0x60, 0x66, 0x6C, 0x78, 0x6C, 0x66, 0x00], // 'k'
    [0x38, 0x18, 0x18, 0x18, 0x18, 0x18, 0x3C, 0x00], // 'l'
    [0x00, 0x00, 0x66, 0x7F, 0x7F, 0x6B, 0x63, 0x00], // 'm'
    [0x00, 0x00, 0x7C, 0x66, 0x66, 0x66, 0x66, 0x00], // 'n'
    [0x00, 0x00, 0x3C, 0x66, 0x66, 0x66, 0x3C, 0x00], // 'o'
    [0x00, 0x00, 0x7C, 0x66, 0x66, 0x7C, 0x60, 0x60], // 'p'
    [0x00, 0x00, 0x3E, 0x66, 0x66, 0x3E, 0x06, 0x06], // 'q'
    [0x00, 0x00, 0x7C, 0x66, 0x60, 0x60, 0x60, 0x00], // 'r'
    [0x00, 0x00, 0x3E, 0x60, 0x3C, 0x06, 0x7C, 0x00], // 's'
    [0x18, 0x18, 0x7E, 0x18, 0x18, 0x18, 0x0E, 0x00], // 't'
    [0x00, 0x00, 0x66, 0x66, 0x66, 0x66, 0x3E, 0x00], // 'u'
    [0x00, 0x00, 0x66, 0x66, 0x66, 0x3C, 0x18, 0x00], // 'v'
    [0x00, 0x00, 0x63, 0x6B, 0x7F, 0x3E, 0x36, 0x00], // 'w'
    [0x00, 0x00, 0x66, 0x3C, 0x18, 0x3C, 0x66, 0x00], // 'x'
    [0x00, 0x00, 0x66, 0x66, 0x66, 0x3E, 0x06, 0x7C], // 'y'
    [0x00, 0x00, 0x7E, 0x0C, 0x18, 0x30, 0x7E, 0x00], // 'z'
    [0x0E, 0x18, 0x18, 0x70, 0x18, 0x18, 0x0E, 0x00], // '{'
    [0x18, 0x18, 0x18, 0x00, 0x18, 0x18, 0x18, 0x00], // '|'
    [0x70, 0x18, 0x18, 0x0E, 0x18, 0x18, 0x70, 0x00], // '}'
    [0x76, 0xDC, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00], // '~' (126)
];

// Embedded 5x7 ASCII Font Table (ASCII 32 ' ' to 126 '~')
#[rustfmt::skip]
static FONT_5X7: [[u8; 5]; 95] = [
    [0x00, 0x00, 0x00, 0x00, 0x00], // 32: ' '
    [0x00, 0x00, 0x5F, 0x00, 0x00], // 33: '!'
    [0x00, 0x07, 0x00, 0x07, 0x00], // 34: '"'
    [0x14, 0x7F, 0x14, 0x7F, 0x14], // 35: '#'
    [0x24, 0x2A, 0x7F, 0x2A, 0x12], // 36: '$'
    [0x23, 0x13, 0x08, 0x64, 0x62], // 37: '%'
    [0x36, 0x49, 0x55, 0x22, 0x50], // 38: '&'
    [0x00, 0x05, 0x03, 0x00, 0x00], // 39: "'"
    [0x00, 0x1C, 0x22, 0x41, 0x00], // 40: '('
    [0x00, 0x41, 0x22, 0x1C, 0x00], // 41: ')'
    [0x14, 0x08, 0x3E, 0x08, 0x14], // 42: '*'
    [0x08, 0x08, 0x3E, 0x08, 0x08], // 43: '+'
    [0x00, 0x50, 0x30, 0x00, 0x00], // 44: ','
    [0x08, 0x08, 0x08, 0x08, 0x08], // 45: '-'
    [0x00, 0x60, 0x60, 0x00, 0x00], // 46: '.'
    [0x20, 0x10, 0x08, 0x04, 0x02], // 47: '/'
    [0x3E, 0x51, 0x49, 0x45, 0x3E], // 48: '0'
    [0x00, 0x42, 0x7F, 0x40, 0x00], // 49: '1'
    [0x42, 0x61, 0x51, 0x49, 0x46], // 50: '2'
    [0x21, 0x41, 0x45, 0x4B, 0x31], // 51: '3'
    [0x18, 0x14, 0x12, 0x7F, 0x10], // 52: '4'
    [0x27, 0x45, 0x45, 0x45, 0x39], // 53: '5'
    [0x3C, 0x4A, 0x49, 0x49, 0x30], // 54: '6'
    [0x01, 0x71, 0x09, 0x05, 0x03], // 55: '7'
    [0x36, 0x49, 0x49, 0x49, 0x36], // 56: '8'
    [0x06, 0x49, 0x49, 0x29, 0x1E], // 57: '9'
    [0x00, 0x36, 0x36, 0x00, 0x00], // 58: ':'
    [0x00, 0x56, 0x36, 0x00, 0x00], // 59: ';'
    [0x08, 0x14, 0x22, 0x41, 0x00], // 60: '<'
    [0x14, 0x14, 0x14, 0x14, 0x14], // 61: '='
    [0x00, 0x41, 0x22, 0x14, 0x08], // 62: '>'
    [0x02, 0x01, 0x51, 0x09, 0x06], // 63: '?'
    [0x32, 0x49, 0x79, 0x41, 0x3E], // 64: '@'
    [0x7E, 0x11, 0x11, 0x11, 0x7E], // 65: 'A'
    [0x7F, 0x49, 0x49, 0x49, 0x36], // 66: 'B'
    [0x3E, 0x41, 0x41, 0x41, 0x22], // 67: 'C'
    [0x7F, 0x41, 0x41, 0x22, 0x1C], // 68: 'D'
    [0x7F, 0x49, 0x49, 0x49, 0x41], // 69: 'E'
    [0x7F, 0x09, 0x09, 0x09, 0x01], // 70: 'F'
    [0x3E, 0x41, 0x49, 0x49, 0x7A], // 71: 'G'
    [0x7F, 0x08, 0x08, 0x08, 0x7F], // 72: 'H'
    [0x00, 0x41, 0x7F, 0x41, 0x00], // 73: 'I'
    [0x20, 0x40, 0x41, 0x3F, 0x01], // 74: 'J'
    [0x7F, 0x08, 0x14, 0x22, 0x41], // 75: 'K'
    [0x7F, 0x40, 0x40, 0x40, 0x40], // 76: 'L'
    [0x7F, 0x02, 0x0C, 0x02, 0x7F], // 77: 'M'
    [0x7F, 0x04, 0x08, 0x10, 0x7F], // 78: 'N'
    [0x3E, 0x41, 0x41, 0x41, 0x3E], // 79: 'O'
    [0x7F, 0x09, 0x09, 0x09, 0x06], // 80: 'P'
    [0x3E, 0x41, 0x51, 0x21, 0x5E], // 81: 'Q'
    [0x7F, 0x09, 0x19, 0x29, 0x46], // 82: 'R'
    [0x46, 0x49, 0x49, 0x49, 0x31], // 83: 'S'
    [0x01, 0x01, 0x7F, 0x01, 0x01], // 84: 'T'
    [0x3F, 0x40, 0x40, 0x40, 0x3F], // 85: 'U'
    [0x1F, 0x20, 0x40, 0x20, 0x1F], // 86: 'V'
    [0x3F, 0x40, 0x38, 0x40, 0x3F], // 87: 'W'
    [0x63, 0x14, 0x08, 0x14, 0x63], // 88: 'X'
    [0x07, 0x08, 0x70, 0x08, 0x07], // 89: 'Y'
    [0x61, 0x51, 0x49, 0x45, 0x43], // 90: 'Z'
    [0x00, 0x7F, 0x41, 0x41, 0x00], // 91: '['
    [0x02, 0x04, 0x08, 0x10, 0x20], // 92: '\\'
    [0x00, 0x41, 0x41, 0x7F, 0x00], // 93: ']'
    [0x04, 0x02, 0x01, 0x02, 0x04], // 94: '^'
    [0x40, 0x40, 0x40, 0x40, 0x40], // 95: '_'
    [0x00, 0x01, 0x02, 0x04, 0x00], // 96: '`'
    [0x20, 0x54, 0x54, 0x54, 0x78], // 97: 'a'
    [0x7F, 0x48, 0x44, 0x44, 0x38], // 98: 'b'
    [0x38, 0x44, 0x44, 0x44, 0x20], // 99: 'c'
    [0x38, 0x44, 0x44, 0x48, 0x7F], // 100: 'd'
    [0x38, 0x54, 0x54, 0x54, 0x18], // 101: 'e'
    [0x08, 0x7E, 0x09, 0x01, 0x02], // 102: 'f'
    [0x0C, 0x52, 0x52, 0x52, 0x3E], // 103: 'g'
    [0x7F, 0x08, 0x04, 0x04, 0x78], // 104: 'h'
    [0x00, 0x44, 0x7D, 0x40, 0x00], // 105: 'i'
    [0x20, 0x40, 0x44, 0x3D, 0x00], // 106: 'j'
    [0x7F, 0x10, 0x28, 0x44, 0x00], // 107: 'k'
    [0x00, 0x41, 0x7F, 0x40, 0x00], // 108: 'l'
    [0x7C, 0x04, 0x18, 0x04, 0x78], // 109: 'm'
    [0x7C, 0x08, 0x04, 0x04, 0x78], // 110: 'n'
    [0x38, 0x44, 0x44, 0x44, 0x38], // 111: 'o'
    [0x7C, 0x14, 0x14, 0x14, 0x08], // 112: 'p'
    [0x08, 0x14, 0x14, 0x18, 0x7C], // 113: 'q'
    [0x7C, 0x08, 0x04, 0x04, 0x08], // 114: 'r'
    [0x48, 0x54, 0x54, 0x54, 0x20], // 115: 's'
    [0x04, 0x3F, 0x44, 0x40, 0x20], // 116: 't'
    [0x3C, 0x40, 0x40, 0x20, 0x7C], // 117: 'u'
    [0x1C, 0x20, 0x40, 0x20, 0x1C], // 118: 'v'
    [0x3C, 0x40, 0x30, 0x40, 0x3C], // 119: 'w'
    [0x44, 0x28, 0x10, 0x28, 0x44], // 120: 'x'
    [0x0C, 0x50, 0x50, 0x50, 0x3C], // 121: 'y'
    [0x44, 0x64, 0x54, 0x4C, 0x44], // 122: 'z'
    [0x00, 0x08, 0x36, 0x41, 0x00], // 123: '{'
    [0x00, 0x00, 0x7F, 0x00, 0x00], // 124: '|'
    [0x00, 0x41, 0x36, 0x08, 0x00], // 125: '}'
    [0x10, 0x08, 0x08, 0x10, 0x08], // 126: '~'
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_canvas_lifecycle_headless() {
        let mut dev = CanvasDevice::new_headless(
            DEFAULT_CANVAS_CMD_PORT,
            DEFAULT_CANVAS_DATA_PORT,
            DEFAULT_CANVAS_STATUS_PORT,
        );

        // 1. Init Canvas: 160x90, title "Test"
        dev.port_write(DEFAULT_CANVAS_CMD_PORT, CMD_INIT_CANVAS);
        dev.port_write(DEFAULT_CANVAS_DATA_PORT, 160);
        dev.port_write(DEFAULT_CANVAS_DATA_PORT, 90);
        dev.port_write(DEFAULT_CANVAS_DATA_PORT, 4); // title len
        for b in b"Test" {
            dev.port_write(DEFAULT_CANVAS_DATA_PORT, *b);
        }

        assert_eq!(dev.logical_width, 160);
        assert_eq!(dev.logical_height, 90);
        assert_eq!(dev.title, "Test");
        assert!(dev.window_open);

        // 2. Query Size
        dev.port_write(DEFAULT_CANVAS_CMD_PORT, CMD_GET_CANVAS_SIZE);
        let w = dev.port_read(DEFAULT_CANVAS_DATA_PORT);
        let h = dev.port_read(DEFAULT_CANVAS_DATA_PORT);
        assert_eq!(w, 160);
        assert_eq!(h, 90);

        // 3. Clear Canvas with Red (0xF00F)
        dev.port_write(DEFAULT_CANVAS_CMD_PORT, CMD_CLEAR);
        dev.port_write(DEFAULT_CANVAS_DATA_PORT, 0xF0);
        dev.port_write(DEFAULT_CANVAS_DATA_PORT, 0x0F);
        assert_eq!(dev.get_pixel_value(10, 10), 0xF00F);

        // 4. SetPixel (20, 30) with Green (0x0F0F)
        dev.port_write(DEFAULT_CANVAS_CMD_PORT, CMD_SET_PIXEL);
        dev.port_write(DEFAULT_CANVAS_DATA_PORT, 20);
        dev.port_write(DEFAULT_CANVAS_DATA_PORT, 30);
        dev.port_write(DEFAULT_CANVAS_DATA_PORT, 0x0F);
        dev.port_write(DEFAULT_CANVAS_DATA_PORT, 0x0F);
        assert_eq!(dev.get_pixel_value(20, 30), 0x0F0F);

        // 5. GetPixel (20, 30)
        dev.port_write(DEFAULT_CANVAS_CMD_PORT, CMD_GET_PIXEL);
        dev.port_write(DEFAULT_CANVAS_DATA_PORT, 20);
        dev.port_write(DEFAULT_CANVAS_DATA_PORT, 30);
        let hi = dev.port_read(DEFAULT_CANVAS_DATA_PORT);
        let lo = dev.port_read(DEFAULT_CANVAS_DATA_PORT);
        assert_eq!(((hi as u16) << 8) | (lo as u16), 0x0F0F);

        // 6. Dispose Canvas
        dev.port_write(DEFAULT_CANVAS_CMD_PORT, CMD_DISPOSE_CANVAS);
        assert!(!dev.window_open);
    }

    #[test]
    fn test_canvas_param_bytes_overflow_protection() {
        let mut dev = CanvasDevice::new_headless(
            DEFAULT_CANVAS_CMD_PORT,
            DEFAULT_CANVAS_DATA_PORT,
            DEFAULT_CANVAS_STATUS_PORT,
        );

        // 1. Stray write when current_cmd is None should be ignored
        dev.port_write(DEFAULT_CANVAS_DATA_PORT, 0x99);
        assert!(dev.param_bytes.is_empty());

        // 2. Start CMD_PAINT_MASK with 255x255 expected size (needs 65033 bytes)
        dev.port_write(DEFAULT_CANVAS_CMD_PORT, CMD_PAINT_MASK);
        dev.port_write(DEFAULT_CANVAS_DATA_PORT, 0); // x
        dev.port_write(DEFAULT_CANVAS_DATA_PORT, 0); // y
        dev.port_write(DEFAULT_CANVAS_DATA_PORT, 255); // w
        dev.port_write(DEFAULT_CANVAS_DATA_PORT, 255); // h
        dev.port_write(DEFAULT_CANVAS_DATA_PORT, 1); // mask_val
        dev.port_write(DEFAULT_CANVAS_DATA_PORT, 0); // invert
        dev.port_write(DEFAULT_CANVAS_DATA_PORT, 0xFF); // hi
        dev.port_write(DEFAULT_CANVAS_DATA_PORT, 0xFF); // lo

        // Flood data port past MAX_PARAM_BYTES (65536 + 16 = 65552)
        for _ in 0..65550 {
            dev.port_write(DEFAULT_CANVAS_DATA_PORT, 0x01);
        }

        // Should have safely reset current_cmd and cleared param_bytes
        assert_eq!(dev.current_cmd, None);
        assert!(dev.param_bytes.is_empty());
    }
}
