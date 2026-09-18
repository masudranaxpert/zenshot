use crate::clipboard::copy_to_clipboard;
use crate::config::Config;
use crate::icons::{IconPair, ToolbarIcons};
use eframe::egui::{
    self, Color32, CursorIcon, Key, Pos2, Rect, Stroke, Vec2,
};
use image::RgbaImage;

/// Handle position for resizing the selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Handle {
    TopLeft,
    Top,
    TopRight,
    Right,
    BottomRight,
    Bottom,
    BottomLeft,
    Left,
}

/// Supported annotation types.
#[derive(Debug, Clone)]
pub enum Annotation {
    Rectangle {
        rect: Rect,
        color: Color32,
        thickness: f32,
    },
    Arrow {
        start: Pos2,
        end: Pos2,
        color: Color32,
        thickness: f32,
    },
    Line {
        start: Pos2,
        end: Pos2,
        color: Color32,
        thickness: f32,
    },
    Pen {
        points: Vec<Pos2>,
        color: Color32,
        thickness: f32,
    },
    Marker {
        points: Vec<Pos2>,
        color: Color32,
        thickness: f32,
    },
    Text {
        pos: Pos2,
        text: String,
        color: Color32,
        size: f32,
    },
}

/// Active editing tool.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    Select,
    Pen,
    Line,
    Arrow,
    Rectangle,
    Marker,
    Text,
}

/// Preset color palette.
pub const PALETTE: [Color32; 6] = [
    Color32::from_rgb(239, 68, 68),  // Red
    Color32::from_rgb(59, 130, 246), // Blue
    Color32::from_rgb(16, 185, 129), // Green
    Color32::from_rgb(245, 158, 11), // Yellow
    Color32::from_rgb(168, 85, 247), // Purple
    Color32::from_rgb(255, 255, 255), // White
];

// Standard toolbar metrics: action bar holds 4 buttons of 24x20, and the tool bar
// holds 8 buttons of 20x20. Those two solve for the padding/gap/margin below.
const H_ICON: Vec2 = Vec2::new(24.0, 20.0);
const V_ICON: Vec2 = Vec2::new(20.0, 20.0);
const BAR_THICKNESS: f32 = 29.0;
const BTN_PAD: f32 = 2.0;
const BTN_GAP: f32 = 1.0;
const BAR_MARGIN: f32 = 2.0;

/// Buttons on the action bar: Print, Copy, Save, Close.
const H_ACTION_COUNT: f32 = 4.0;
/// Buttons on the tool bar: Pen, Line, Arrow, Rect, Marker, Text, Color, Undo.
const V_TOOL_COUNT: f32 = 8.0;

/// Length of a toolbar along its main axis for `n` buttons of `icon` extent.
fn bar_length(n: f32, icon: f32) -> f32 {
    n * (icon + 2.0 * BTN_PAD) + (n - 1.0) * BTN_GAP + 2.0 * BAR_MARGIN
}

const TOOLBAR_GAP: f32 = 4.0;

/// Screen region the floating bars may occupy, clipped to the monitor work area.
fn toolbar_safe_bounds(sel: Rect, screen: Rect) -> Rect {
    let inset = screen.shrink(TOOLBAR_GAP);
    #[cfg(windows)]
    {
        if let Some(work) = monitor_work_area_points(sel.right_bottom()) {
            let clipped = work.intersect(inset);
            if clipped.width() >= BAR_THICKNESS && clipped.height() >= BAR_THICKNESS {
                return clipped;
            }
        }
    }
    let _ = sel;
    inset
}

#[cfg(windows)]
fn monitor_work_area_points(anchor_pt: Pos2) -> Option<Rect> {
    use windows_sys::Win32::Foundation::POINT;
    use windows_sys::Win32::Graphics::Gdi::{
        GetMonitorInfoW, MonitorFromPoint, MONITORINFO, MONITOR_DEFAULTTONEAREST,
    };

    let scale = crate::display_scale_factor().max(1.0);
    let (vx, vy, _, _) = crate::capture::virtual_screen_bounds();

    let phys_pt = POINT {
        x: (anchor_pt.x * scale).round() as i32 + vx,
        y: (anchor_pt.y * scale).round() as i32 + vy,
    };

    unsafe {
        let hmon = MonitorFromPoint(phys_pt, MONITOR_DEFAULTTONEAREST);
        if hmon.is_null() {
            return None;
        }

        let mut mi: MONITORINFO = std::mem::zeroed();
        mi.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
        if GetMonitorInfoW(hmon, &mut mi) == 0 {
            return None;
        }

        let rc = mi.rcWork;
        let left = (rc.left - vx) as f32 / scale;
        let top = (rc.top - vy) as f32 / scale;
        let right = (rc.right - vx) as f32 / scale;
        let bottom = (rc.bottom - vy) as f32 / scale;

        Some(Rect::from_min_max(Pos2::new(left, top), Pos2::new(right, bottom)))
    }
}

fn clamp_pos(v: f32, lo: f32, hi: f32) -> f32 {
    if lo <= hi {
        v.clamp(lo, hi)
    } else {
        lo
    }
}

/// Prefer `outside_a`, then `outside_b`, then `inside`, all clamped to `[lo, hi]`.
fn place_axis(outside_a: f32, outside_b: f32, inside: f32, lo: f32, hi: f32) -> f32 {
    if outside_a >= lo && outside_a <= hi {
        outside_a
    } else if outside_b >= lo && outside_b <= hi {
        outside_b
    } else {
        clamp_pos(inside, lo, hi)
    }
}

/// Places the action (horizontal) and tool (vertical) bars around `sel`, staying
/// inside `safe`. Full-screen grabs have no outside room, so the bars sit in the
/// bottom-right as an L: tools on the right, actions to their left.
fn layout_toolbars(sel: Rect, safe: Rect) -> (Rect, Rect) {
    let h_size = Vec2::new(bar_length(H_ACTION_COUNT, H_ICON.x), BAR_THICKNESS);
    let v_size = Vec2::new(BAR_THICKNESS, bar_length(V_TOOL_COUNT, V_ICON.y));

    let v_x = place_axis(
        sel.right() + TOOLBAR_GAP,
        sel.left() - TOOLBAR_GAP - v_size.x,
        sel.right() - TOOLBAR_GAP - v_size.x,
        safe.left(),
        safe.right() - v_size.x,
    );
    let v_y = clamp_pos(sel.bottom() - v_size.y, safe.top(), safe.bottom() - v_size.y);
    let v_rect = Rect::from_min_size(Pos2::new(v_x, v_y), v_size);

    let h_y = place_axis(
        sel.bottom() + TOOLBAR_GAP,
        sel.top() - TOOLBAR_GAP - h_size.y,
        sel.bottom() - TOOLBAR_GAP - h_size.y,
        safe.top(),
        safe.bottom() - h_size.y,
    );
    let h_x = clamp_pos(sel.right() - h_size.x, safe.left(), safe.right() - h_size.x);
    let mut h_rect = Rect::from_min_size(Pos2::new(h_x, h_y), h_size);

    if h_rect.intersects(v_rect) {
        let left_of_v = v_rect.left() - TOOLBAR_GAP - h_size.x;
        if left_of_v >= safe.left() {
            h_rect = Rect::from_min_size(Pos2::new(left_of_v, h_y), h_size);
        } else {
            let above = v_rect.top() - TOOLBAR_GAP - h_size.y;
            let y = if above >= safe.top() {
                above
            } else {
                clamp_pos(
                    v_rect.bottom() + TOOLBAR_GAP,
                    safe.top(),
                    safe.bottom() - h_size.y,
                )
            };
            h_rect = Rect::from_min_size(
                Pos2::new(
                    clamp_pos(v_rect.right() - h_size.x, safe.left(), safe.right() - h_size.x),
                    y,
                ),
                h_size,
            );
        }
    }

    (h_rect, v_rect)
}

/// Selection chrome: 1px marching-ants pattern of three black then three white pixels,
/// eight 6x6 square grips, and a dimension badge above top-left.
const ANTS_DASH: f32 = 3.0;
const HANDLE_SIZE: f32 = 6.0;
const BADGE_FONT_SIZE: f32 = 11.0;
const BADGE_BACKDROP: Color32 = Color32::from_black_alpha(201);

/// The eight resize grips, clockwise from the top-left corner.
fn handle_positions(sel: Rect) -> [Pos2; 8] {
    [
        sel.left_top(),
        Pos2::new(sel.center().x, sel.top()),
        sel.right_top(),
        Pos2::new(sel.right(), sel.center().y),
        sel.right_bottom(),
        Pos2::new(sel.center().x, sel.bottom()),
        sel.left_bottom(),
        Pos2::new(sel.left(), sel.center().y),
    ]
}

/// Draws the alternating black/white 1px selection outline.
fn paint_marching_ants(painter: &egui::Painter, rect: Rect) {
    let edges = [
        (rect.left_top(), rect.right_top()),
        (rect.right_top(), rect.right_bottom()),
        (rect.right_bottom(), rect.left_bottom()),
        (rect.left_bottom(), rect.left_top()),
    ];

    for (from, to) in edges {
        let span = to - from;
        let len = span.length();
        if len <= 0.0 {
            continue;
        }
        let dir = span / len;

        let mut travelled = 0.0;
        let mut ink_black = true;
        while travelled < len {
            let step = ANTS_DASH.min(len - travelled);
            let color = if ink_black { Color32::BLACK } else { Color32::WHITE };
            painter.line_segment(
                [from + dir * travelled, from + dir * (travelled + step)],
                Stroke::new(1.0_f32, color),
            );
            travelled += step;
            ink_black = !ink_black;
        }
    }
}

/// Draws one resize grip.
fn paint_handle(painter: &egui::Painter, center: Pos2) {
    let outer = Rect::from_center_size(center, Vec2::splat(HANDLE_SIZE));
    painter.rect_filled(outer, 0.0_f32, Color32::WHITE);
    painter.rect_filled(outer.shrink(1.0), 0.0_f32, Color32::BLACK);
}

/// Dragging interaction state.
#[derive(Debug, Clone)]
enum DragState {
    None,
    CreatingSelection(Pos2),
    MovingSelection { start_mouse: Pos2, orig_rect: Rect },
    ResizingSelection { handle: Handle, orig_rect: Rect },
    DrawingRect(Pos2),
    DrawingArrow(Pos2),
    DrawingLine(Pos2),
    DrawingPath(Vec<Pos2>),
}

/// Main application state for ZenShot.
pub struct ZenShotApp {
    config: Config,
    screen_image: RgbaImage,
    texture: Option<egui::TextureHandle>,
    icons: Option<ToolbarIcons>,
    selection: Option<Rect>,
    drag_state: DragState,
    current_tool: Tool,
    color_index: usize,
    annotations: Vec<Annotation>,
    text_input: String,
    active_text_pos: Option<Pos2>,
    last_pointer: Pos2,
    export_error: Option<String>,
    vanished: bool,
    frame_count: u32,
    t0: std::time::Instant,
    is_warm: bool,
    is_active: bool,
    warmed_up: bool,
    trigger_flag: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
    quit_flag: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
    #[cfg(windows)]
    hwnd: isize,
    #[cfg(windows)]
    prev_foreground: isize,
}

impl ZenShotApp {
    #[cfg(windows)]
    pub fn new(
        config: Config,
        screen_image: Option<RgbaImage>,
        ctx: &egui::Context,
        hwnd: isize,
        prev_foreground: isize,
        t0: std::time::Instant,
    ) -> Self {
        let selection = if config.keep_selection {
            config.last_selection.and_then(|[x, y, w, h]| {
                if w > 6.0 && h > 6.0 {
                    Some(Rect::from_min_size(Pos2::new(x, y), Vec2::new(w, h)))
                } else {
                    None
                }
            })
        } else {
            None
        };

        let (screen_image, texture) = if let Some(img) = screen_image {
            let size = [img.width() as usize, img.height() as usize];
            let pixels: Vec<egui::Color32> =
                bytemuck::cast_slice::<u8, egui::Color32>(img.as_raw()).to_vec();
            let color_image = egui::ColorImage { size, pixels };
            let tex = ctx.load_texture("desktop", color_image, egui::TextureOptions::NEAREST);
            (img, Some(tex))
        } else {
            (RgbaImage::new(0, 0), None)
        };

        Self {
            config,
            screen_image,
            texture,
            icons: None,
            selection,
            drag_state: DragState::None,
            current_tool: Tool::Select,
            color_index: 0,
            annotations: Vec::new(),
            text_input: String::new(),
            active_text_pos: None,
            last_pointer: Pos2::ZERO,
            export_error: None,
            vanished: false,
            frame_count: 0,
            t0,
            is_warm: false,
            is_active: true,
            warmed_up: true,
            trigger_flag: None,
            quit_flag: None,
            hwnd,
            prev_foreground,
        }
    }

    #[cfg(windows)]
    pub fn new_warm(
        config: Config,
        ctx: &egui::Context,
        hwnd: isize,
        trigger_flag: std::sync::Arc<std::sync::atomic::AtomicBool>,
        quit_flag: std::sync::Arc<std::sync::atomic::AtomicBool>,
    ) -> Self {
        let dummy = egui::ColorImage::example();
        let texture = ctx.load_texture("desktop", dummy, egui::TextureOptions::NEAREST);
        let icons = Some(ToolbarIcons::load(ctx));

        Self {
            config,
            screen_image: RgbaImage::new(0, 0),
            texture: Some(texture),
            icons,
            selection: None,
            drag_state: DragState::None,
            current_tool: Tool::Select,
            color_index: 0,
            annotations: Vec::new(),
            text_input: String::new(),
            active_text_pos: None,
            last_pointer: Pos2::ZERO,
            export_error: None,
            vanished: false,
            frame_count: 0,
            t0: std::time::Instant::now(),
            is_warm: true,
            is_active: false,
            warmed_up: false,
            trigger_flag: Some(trigger_flag),
            quit_flag: Some(quit_flag),
            hwnd,
            prev_foreground: 0,
        }
    }

    #[cfg(not(windows))]
    pub fn new(
        config: Config,
        screen_image: Option<RgbaImage>,
        ctx: &egui::Context,
        t0: std::time::Instant,
    ) -> Self {
        let selection = if config.keep_selection {
            config.last_selection.and_then(|[x, y, w, h]| {
                if w > 6.0 && h > 6.0 {
                    Some(Rect::from_min_size(Pos2::new(x, y), Vec2::new(w, h)))
                } else {
                    None
                }
            })
        } else {
            None
        };

        let (screen_image, texture) = if let Some(img) = screen_image {
            let size = [img.width() as usize, img.height() as usize];
            let pixels: Vec<egui::Color32> =
                bytemuck::cast_slice::<u8, egui::Color32>(img.as_raw()).to_vec();
            let color_image = egui::ColorImage { size, pixels };
            let tex = ctx.load_texture("desktop", color_image, egui::TextureOptions::NEAREST);
            (img, Some(tex))
        } else {
            (RgbaImage::new(0, 0), None)
        };

        Self {
            config,
            screen_image,
            texture,
            icons: None,
            selection,
            drag_state: DragState::None,
            current_tool: Tool::Select,
            color_index: 0,
            annotations: Vec::new(),
            text_input: String::new(),
            active_text_pos: None,
            last_pointer: Pos2::ZERO,
            export_error: None,
            vanished: false,
            frame_count: 0,
            t0,
            is_warm: false,
            is_active: true,
            warmed_up: true,
            trigger_flag: None,
            quit_flag: None,
        }
    }

    #[cfg(not(windows))]
    pub fn new_warm(
        config: Config,
        ctx: &egui::Context,
        trigger_flag: std::sync::Arc<std::sync::atomic::AtomicBool>,
        quit_flag: std::sync::Arc<std::sync::atomic::AtomicBool>,
    ) -> Self {
        let dummy = egui::ColorImage::example();
        let texture = ctx.load_texture("desktop", dummy, egui::TextureOptions::NEAREST);
        let icons = Some(ToolbarIcons::load(ctx));

        Self {
            config,
            screen_image: RgbaImage::new(0, 0),
            texture: Some(texture),
            icons,
            selection: None,
            drag_state: DragState::None,
            current_tool: Tool::Select,
            color_index: 0,
            annotations: Vec::new(),
            text_input: String::new(),
            active_text_pos: None,
            last_pointer: Pos2::ZERO,
            export_error: None,
            vanished: false,
            frame_count: 0,
            t0: std::time::Instant::now(),
            is_warm: true,
            is_active: false,
            warmed_up: false,
            trigger_flag: Some(trigger_flag),
            quit_flag: Some(quit_flag),
        }
    }

    /// Hide the overlay immediately so Copy/Esc gives immediate visual feedback before background export.
    fn vanish(&mut self, ctx: &egui::Context) {
        let _ = ctx;
        if self.vanished {
            return;
        }
        self.vanished = true;

        #[cfg(windows)]
        unsafe {
            use windows_sys::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWA_CLOAK};
            use windows_sys::Win32::UI::WindowsAndMessaging::{
                SetForegroundWindow, SetWindowPos, SWP_NOACTIVATE, SWP_NOSIZE, SWP_NOZORDER,
            };
            if self.hwnd != 0 {
                let on: i32 = 1;
                let _ = DwmSetWindowAttribute(
                    self.hwnd as _,
                    DWMWA_CLOAK as u32,
                    &on as *const _ as *const _,
                    std::mem::size_of::<i32>() as u32,
                );
                // Park off-screen instead of SW_HIDE so winit/User32 keeps processing request_repaint().
                SetWindowPos(
                    self.hwnd as _,
                    std::ptr::null_mut(),
                    -32000,
                    -32000,
                    0,
                    0,
                    SWP_NOSIZE | SWP_NOACTIVATE | SWP_NOZORDER,
                );
                if self.prev_foreground != 0 {
                    SetForegroundWindow(self.prev_foreground as _);
                }
            }
        }

        #[cfg(not(windows))]
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
    }

    fn restore(&mut self, ctx: &egui::Context) {
        self.vanished = false;

        #[cfg(windows)]
        unsafe {
            use windows_sys::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWA_CLOAK};
            use windows_sys::Win32::UI::WindowsAndMessaging::{
                SetForegroundWindow, SetWindowPos, HWND_TOPMOST, SWP_NOSIZE,
            };
            if self.hwnd != 0 {
                let (vx, vy, _, _) = crate::capture::virtual_screen_bounds();
                SetWindowPos(
                    self.hwnd as _,
                    HWND_TOPMOST,
                    vx,
                    vy,
                    0,
                    0,
                    SWP_NOSIZE,
                );
                let off: i32 = 0;
                let _ = DwmSetWindowAttribute(
                    self.hwnd as _,
                    DWMWA_CLOAK as u32,
                    &off as *const _ as *const _,
                    std::mem::size_of::<i32>() as u32,
                );
                SetForegroundWindow(self.hwnd as _);
            }
        }

        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
    }

    fn fail_export(&mut self, ctx: &egui::Context, err: String) {
        self.export_error = Some(err);
        self.restore(ctx);
    }

    /// Text is only pushed into `annotations` on Enter. Export from the
    /// toolbar must commit whatever is still in the inline editor first.
    fn commit_pending_text(&mut self) {
        let Some(pos) = self.active_text_pos.take() else {
            return;
        };
        let text = std::mem::take(&mut self.text_input);
        if !text.trim().is_empty() {
            self.annotations.push(Annotation::Text {
                pos,
                text,
                color: self.current_color(),
                size: 16.0,
            });
        }
    }

    /// In one-shot mode, exits the process. In warm mode, hides the window,
    /// restores foreground, and resets state to await the next IPC capture trigger.
    fn quit(&mut self, ctx: &egui::Context) {
        self.vanish(ctx);
        if self.is_warm {
            if let Some(trigger_flag) = &self.trigger_flag {
                trigger_flag.store(false, std::sync::atomic::Ordering::SeqCst);
            }
            self.is_active = false;
            self.frame_count = 0;
            self.selection = None;
            self.annotations.clear();
            self.drag_state = DragState::None;
            self.active_text_pos = None;
            self.text_input.clear();
            self.vanished = false;
        } else {
            std::process::exit(0);
        }
    }

    /// Active drawing color from palette.
    pub fn current_color(&self) -> Color32 {
        PALETTE[self.color_index % PALETTE.len()]
    }

    /// Cycle to next color in palette.
    pub fn cycle_color(&mut self) {
        self.color_index = (self.color_index + 1) % PALETTE.len();
    }

    /// Performs in-memory crop with burned annotations.
    fn crop_current_selection(&self, ctx: &egui::Context, screen_rect: Rect) -> Option<RgbaImage> {
        let sel = self.selection?;
        Some(burn_and_crop(&self.screen_image, screen_rect, sel, &self.annotations, ctx))
    }

    fn remember_selection(&mut self) {
        if !self.config.keep_selection {
            return;
        }
        if let Some(sel) = self.selection {
            let last = [sel.min.x, sel.min.y, sel.width(), sel.height()];
            self.config.last_selection = Some(last);
            let _ = Config::persist_last_selection(last);
        }
    }

    /// Saves cropped image to configured path and exits immediately.
    /// With nothing selected this is a no-op, matching Lightshot.
    fn action_save(&mut self, ctx: &egui::Context, screen_rect: Rect) {
        if self.selection.is_none() {
            return;
        }
        self.commit_pending_text();
        self.vanish(ctx);
        let Some(img) = self.crop_current_selection(ctx, screen_rect) else {
            self.fail_export(ctx, "Nothing to save".into());
            return;
        };
        match crate::export::save_image(&img, &self.config) {
            Ok(path) => {
                self.remember_selection();
                if self.config.show_notifications {
                    crate::notify::saved(&path);
                }
                self.quit(ctx);
            }
            Err(err) => self.fail_export(ctx, err),
        }
    }

    /// Copies cropped image directly to clipboard in RAM and exits immediately.
    fn action_copy(&mut self, ctx: &egui::Context, screen_rect: Rect) {
        if self.selection.is_none() {
            return;
        }
        self.commit_pending_text();
        self.vanish(ctx);
        let Some(img) = self.crop_current_selection(ctx, screen_rect) else {
            self.fail_export(ctx, "Nothing to copy".into());
            return;
        };
        match copy_to_clipboard(&img) {
            Ok(()) => {
                self.remember_selection();
                if self.config.show_notifications {
                    crate::notify::copied();
                }
                self.quit(ctx);
            }
            Err(err) => self.fail_export(ctx, err),
        }
    }

    /// Hands the cropped image to the system printer and exits.
    /// Printing is the one path that has to touch disk, since both print
    /// backends take a file rather than a stream.
    fn action_print(&mut self, ctx: &egui::Context, screen_rect: Rect) {
        if self.selection.is_none() {
            return;
        }
        self.commit_pending_text();
        self.vanish(ctx);
        let Some(img) = self.crop_current_selection(ctx, screen_rect) else {
            self.fail_export(ctx, "Nothing to print".into());
            return;
        };
        let path = std::env::temp_dir().join(format!("zenshot_print_{}.png", std::process::id()));
        if let Err(err) = img.save(&path) {
            self.fail_export(ctx, format!("Print failed: {err}"));
            return;
        }
        match spawn_print_job(&path) {
            Ok(()) => self.quit(ctx),
            Err(err) => self.fail_export(ctx, format!("Print failed: {err}")),
        }
    }

    /// Checks if mouse point hits any of the 8 selection handles.
    fn hit_test_handles(&self, sel: Rect, point: Pos2) -> Option<Handle> {
        const HANDLE_ORDER: [Handle; 8] = [
            Handle::TopLeft,
            Handle::Top,
            Handle::TopRight,
            Handle::Right,
            Handle::BottomRight,
            Handle::Bottom,
            Handle::BottomLeft,
            Handle::Left,
        ];

        // A couple of pixels of slack past the drawn grip, so they stay easy to
        // grab without swallowing clicks well inside the selection.
        let reach = HANDLE_SIZE / 2.0 + 2.0;
        HANDLE_ORDER
            .into_iter()
            .zip(handle_positions(sel))
            .find(|(_, pos)| (point - *pos).length() <= reach)
            .map(|(handle, _)| handle)
    }

    /// Calculates exact screen bounds of the Horizontal and Vertical toolbars.
    fn get_toolbar_rects(&self, sel: Rect, screen_rect: Rect) -> (Rect, Rect) {
        layout_toolbars(sel, toolbar_safe_bounds(sel, screen_rect))
    }
}

/// Draws the toolbar chrome scanline by scanline: a 1px translucent black frame, a
/// (250,251,251) highlight row, a linear body fade to (211,214,217), then a
/// two-row drop shadow. `vertical` runs the fade left-to-right instead.
pub(crate) fn paint_lightshot_toolbar(painter: &egui::Painter, rect: Rect, vertical: bool) {
    let extent = if vertical { rect.width() } else { rect.height() };
    let rows = extent.round() as i32;

    for row in 0..rows {
        let color = toolbar_scanline(row, rows);
        let offset = row as f32;
        let strip = if vertical {
            Rect::from_min_max(
                Pos2::new(rect.min.x + offset, rect.min.y),
                Pos2::new((rect.min.x + offset + 1.0).min(rect.max.x), rect.max.y),
            )
        } else {
            Rect::from_min_max(
                Pos2::new(rect.min.x, rect.min.y + offset),
                Pos2::new(rect.max.x, (rect.min.y + offset + 1.0).min(rect.max.y)),
            )
        };
        painter.rect_filled(strip, 0.0_f32, color);
    }

    // Frame along the two edges the scanlines do not cover.
    let (start, end) = if vertical {
        (
            [rect.left_top(), rect.right_top()],
            [rect.left_bottom(), rect.right_bottom()],
        )
    } else {
        (
            [rect.left_top(), rect.left_bottom()],
            [rect.right_top(), rect.right_bottom()],
        )
    };
    painter.line_segment(start, Stroke::new(1.0_f32, TOOLBAR_FRAME));
    painter.line_segment(end, Stroke::new(1.0_f32, TOOLBAR_FRAME));
}

const TOOLBAR_FRAME: Color32 = Color32::from_black_alpha(19);

/// Colour of scanline `row` of a toolbar `rows` thick.
///
/// The body is not a single ramp: the strip falls from (245,247,248) to
/// (232,236,239) over the first 8% and then eases to (211,214,217), which
/// gives the bars a glossy rather than flat finish.
fn toolbar_scanline(row: i32, rows: i32) -> Color32 {
    const HIGHLIGHT: Color32 = Color32::from_rgb(250, 251, 251);
    const BODY_TOP: (u8, u8, u8) = (245, 247, 248);
    const BODY_KNEE: (u8, u8, u8) = (232, 236, 239);
    const BODY_BOTTOM: (u8, u8, u8) = (211, 214, 217);
    const KNEE_AT: f32 = 0.087;

    match rows - row {
        1 => return Color32::from_black_alpha(26),
        2 => return Color32::from_black_alpha(91),
        3 => return Color32::from_rgb(203, 206, 208),
        _ => {}
    }
    if row == 0 {
        return TOOLBAR_FRAME;
    }
    if row == 1 {
        return HIGHLIGHT;
    }

    let body_rows = (rows - 5).max(1) as f32;
    let t = ((row - 2) as f32 / body_rows).clamp(0.0, 1.0);

    let (from, to, local) = if t <= KNEE_AT {
        (BODY_TOP, BODY_KNEE, t / KNEE_AT)
    } else {
        (BODY_KNEE, BODY_BOTTOM, (t - KNEE_AT) / (1.0 - KNEE_AT))
    };

    Color32::from_rgb(
        lerp_u8(from.0, to.0, local),
        lerp_u8(from.1, to.1, local),
        lerp_u8(from.2, to.2, local),
    )
}

#[inline]
fn lerp_u8(a: u8, b: u8, t: f32) -> u8 {
    (a as f32 + (b as f32 - a as f32) * t).round() as u8
}

/// One toolbar cell: dark glyph while idle, colored glyph on hover or when
/// selected, plus soft hover highlight.
fn icon_button(ui: &mut egui::Ui, icon: &IconPair, size: Vec2, active: bool, tip: &str) -> egui::Response {
    let cell = size + Vec2::splat(2.0 * BTN_PAD);
    let (rect, resp) = ui.allocate_exact_size(cell, egui::Sense::click());
    let hovered = resp.hovered() || resp.is_pointer_button_down_on();

    let border = Stroke::new(1.0_f32, Color32::from_rgb(166, 178, 190));
    if active {
        ui.painter().rect_filled(rect, 2.0, Color32::from_black_alpha(30));
        ui.painter().rect_stroke(rect, 2.0, border);
    } else if hovered {
        ui.painter().rect_filled(rect, 2.0, Color32::from_white_alpha(150));
        ui.painter().rect_stroke(rect, 2.0, border);
    }

    // Downsample icon through a mipmapped linear texture for smooth edges at any DPI.
    egui::Image::new(icon.get(active || hovered))
        .fit_to_exact_size(size)
        .rounding(2.0)
        .paint_at(ui, rect);

    resp.on_hover_text(tip)
}

impl eframe::App for ZenShotApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // In warm mode, handle IPC quit and capture triggers.
        if let Some(quit_flag) = &self.quit_flag {
            if quit_flag.load(std::sync::atomic::Ordering::SeqCst) {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                return;
            }
        }

        if self.is_warm {
            let triggered = self
                .trigger_flag
                .as_ref()
                .map(|f| f.swap(false, std::sync::atomic::Ordering::SeqCst))
                .unwrap_or(false);

            if !self.is_active {
                if !self.warmed_up {
                    self.warmed_up = true;
                    #[cfg(windows)]
                    unsafe {
                        if self.hwnd != 0 {
                            use windows_sys::Win32::UI::WindowsAndMessaging::{
                                SetWindowPos, SWP_NOACTIVATE, SWP_NOSIZE, SWP_NOZORDER,
                            };
                            // Park off-screen so eframe's first-frame set_visible(true) cannot intercept desktop clicks.
                            SetWindowPos(
                                self.hwnd as _,
                                std::ptr::null_mut(),
                                -32000,
                                -32000,
                                0,
                                0,
                                SWP_NOSIZE | SWP_NOACTIVATE | SWP_NOZORDER,
                            );
                        }
                    }
                    return;
                }

                if !triggered {
                    return;
                }
            } else if !triggered {
                // Currently active and no new trigger received; proceed with normal rendering.
            } else {
                // Re-triggered while already active: reset and take a fresh capture.
                self.vanish(ctx);
            }

            if triggered {
                self.t0 = std::time::Instant::now();
                self.config = Config::load_or_default();
                #[cfg(windows)]
                {
                    self.prev_foreground = unsafe {
                        windows_sys::Win32::UI::WindowsAndMessaging::GetForegroundWindow()
                    } as isize;
                }

                let t_cap = std::time::Instant::now();
                match crate::capture::capture_screen(self.config.capture_cursor) {
                    Ok(img) => {
                        let size = [img.width() as usize, img.height() as usize];
                        let pixels: Vec<egui::Color32> =
                            bytemuck::cast_slice::<u8, egui::Color32>(img.as_raw()).to_vec();
                        let color_image = egui::ColorImage { size, pixels };
                        if let Some(tex) = &mut self.texture {
                            tex.set(color_image, egui::TextureOptions::NEAREST);
                        } else {
                            self.texture = Some(ctx.load_texture(
                                "desktop",
                                color_image,
                                egui::TextureOptions::NEAREST,
                            ));
                        }
                        self.screen_image = img;
                        eprintln!("[ZenShot PERF] Warm capture + texture set: {:?}", t_cap.elapsed());
                    }
                    Err(err) => {
                        eprintln!("Error capturing screen: {err}");
                        self.quit(ctx);
                        return;
                    }
                }

                self.is_active = true;
                self.frame_count = 0;

                #[cfg(windows)]
                unsafe {
                    if self.hwnd != 0 {
                        use windows_sys::Win32::UI::WindowsAndMessaging::{
                            SetWindowPos, HWND_TOPMOST, SWP_NOACTIVATE, SWP_NOSIZE,
                        };
                        let (vx, vy, _, _) = crate::capture::virtual_screen_bounds();
                        SetWindowPos(
                            self.hwnd as _,
                            HWND_TOPMOST,
                            vx,
                            vy,
                            0,
                            0,
                            SWP_NOSIZE | SWP_NOACTIVATE,
                        );
                    }
                }
                self.selection = if self.config.keep_selection {
                    self.config.last_selection.and_then(|[x, y, w, h]| {
                        if w > 6.0 && h > 6.0 {
                            Some(Rect::from_min_size(Pos2::new(x, y), Vec2::new(w, h)))
                        } else {
                            None
                        }
                    })
                } else {
                    None
                };
                self.drag_state = DragState::None;
                self.current_tool = Tool::Select;
                self.annotations.clear();
                self.active_text_pos = None;
                self.text_input.clear();
                self.export_error = None;
                self.vanished = false;
                ctx.request_repaint();
            }
        }

        // Capture live screen in Frame 1 while window is still DWM-cloaked (cold-start path).
        // This ensures the screenshot is ~16ms fresh when uncloaked on Frame 2,
        // eliminating the 300ms stale-frame caret blink/tooltip disparity.
        if self.texture.is_none() {
            let t_cap = std::time::Instant::now();
            match crate::capture::capture_screen(self.config.capture_cursor) {
                Ok(img) => {
                    let size = [img.width() as usize, img.height() as usize];
                    let pixels: Vec<egui::Color32> =
                        bytemuck::cast_slice::<u8, egui::Color32>(img.as_raw()).to_vec();
                    let color_image = egui::ColorImage { size, pixels };
                    self.texture = Some(ctx.load_texture(
                        "desktop",
                        color_image,
                        egui::TextureOptions::NEAREST,
                    ));
                    self.screen_image = img;
                    eprintln!("[ZenShot PERF] Frame 1 capture: {:?}", t_cap.elapsed());
                }
                Err(err) => {
                    eprintln!("Error capturing screen: {err}");
                    self.quit(ctx);
                }
            }
        }

        let screen_rect = ctx.screen_rect();
        if matches!(self.drag_state, DragState::None) {
            if let Some(sel) = self.selection {
                self.selection = Some(constrain_to_screen(sel, screen_rect));
            }
        }

        // 2. Global overlay hotkeys: Esc/Ctrl+X close, Ctrl+A full screen,
        // Ctrl+C copy, Ctrl+S save, Ctrl+P print and Ctrl+Z undo.
        if self.active_text_pos.is_some() {
            if ctx.input(|i| i.key_pressed(Key::Escape)) {
                self.active_text_pos = None;
                self.text_input.clear();
            }
        } else {
            let hit = |key: Key| {
                ctx.input(|i| (i.modifiers.command || i.modifiers.ctrl) && i.key_pressed(key))
            };

            if ctx.input(|i| i.key_pressed(Key::Escape)) || hit(Key::X) {
                self.quit(ctx);
                return;
            }
            if hit(Key::A) {
                self.selection = Some(screen_rect);
            }
            if hit(Key::C) {
                self.action_copy(ctx, screen_rect);
                return;
            }
            if hit(Key::S) {
                self.action_save(ctx, screen_rect);
                return;
            }
            if hit(Key::P) {
                self.action_print(ctx, screen_rect);
                return;
            }
            if hit(Key::Z) {
                self.annotations.pop();
            }
        }

        // 3. Pointer and toolbar hover detection
        let pointer = ctx.input(|i| i.pointer.clone());
        let current_pos = pointer.latest_pos().unwrap_or(self.last_pointer);
        self.last_pointer = current_pos;

        // Right-click clears the selection.
        if pointer.secondary_clicked() {
            self.selection = None;
            self.annotations.clear();
            self.drag_state = DragState::None;
        }

        let on_toolbar = self.selection.is_some_and(|sel| {
            let (h_bar, v_bar) = self.get_toolbar_rects(sel, screen_rect);
            h_bar.contains(current_pos) || v_bar.contains(current_pos)
        });

        let mouse_on_toolbar = on_toolbar && matches!(self.drag_state, DragState::None);

        let mut desired_cursor = if mouse_on_toolbar {
            CursorIcon::Default
        } else {
            CursorIcon::Crosshair
        };
        let active_color = self.current_color();

        // 4. Mouse Drag State Machine (ONLY processes if mouse is NOT over toolbar)
        if !mouse_on_toolbar {
            match &mut self.drag_state {
                DragState::None => {
                    if let Some(sel) = self.selection {
                        if let Some(handle) = self.hit_test_handles(sel, current_pos) {
                            desired_cursor = match handle {
                                Handle::TopLeft | Handle::BottomRight => CursorIcon::ResizeNwSe,
                                Handle::TopRight | Handle::BottomLeft => CursorIcon::ResizeNeSw,
                                Handle::Top | Handle::Bottom => CursorIcon::ResizeVertical,
                                Handle::Left | Handle::Right => CursorIcon::ResizeHorizontal,
                            };

                            if pointer.primary_pressed() {
                                self.drag_state = DragState::ResizingSelection { handle, orig_rect: sel };
                            }
                        } else if sel.contains(current_pos) {
                            match self.current_tool {
                                Tool::Select => {
                                    desired_cursor = CursorIcon::Move;
                                    if pointer.primary_pressed() {
                                        self.drag_state = DragState::MovingSelection { start_mouse: current_pos, orig_rect: sel };
                                    }
                                }
                                Tool::Pen => {
                                    desired_cursor = CursorIcon::Crosshair;
                                    if pointer.primary_pressed() {
                                        self.drag_state = DragState::DrawingPath(vec![current_pos]);
                                    }
                                }
                                Tool::Line => {
                                    desired_cursor = CursorIcon::Crosshair;
                                    if pointer.primary_pressed() {
                                        self.drag_state = DragState::DrawingLine(current_pos);
                                    }
                                }
                                Tool::Arrow => {
                                    desired_cursor = CursorIcon::Crosshair;
                                    if pointer.primary_pressed() {
                                        self.drag_state = DragState::DrawingArrow(current_pos);
                                    }
                                }
                                Tool::Rectangle => {
                                    desired_cursor = CursorIcon::Crosshair;
                                    if pointer.primary_pressed() {
                                        self.drag_state = DragState::DrawingRect(current_pos);
                                    }
                                }
                                Tool::Marker => {
                                    desired_cursor = CursorIcon::Crosshair;
                                    if pointer.primary_pressed() {
                                        self.drag_state = DragState::DrawingPath(vec![current_pos]);
                                    }
                                }
                                Tool::Text => {
                                    desired_cursor = CursorIcon::Text;
                                    if pointer.primary_pressed() {
                                        self.active_text_pos = Some(current_pos);
                                        self.text_input.clear();
                                    }
                                }
                            }
                        } else if pointer.primary_pressed() {
                            self.selection = None;
                            self.annotations.clear();
                            self.drag_state = DragState::CreatingSelection(current_pos);
                        }
                    } else if pointer.primary_pressed() {
                        self.drag_state = DragState::CreatingSelection(current_pos);
                    }
                }
                DragState::CreatingSelection(start) => {
                    desired_cursor = CursorIcon::Crosshair;
                    self.selection = Some(Rect::from_two_pos(*start, current_pos));

                    if pointer.primary_released() {
                        if let Some(sel) = self.selection {
                            let normalized = normalize_rect(sel);
                            if normalized.width() > 6.0 && normalized.height() > 6.0 {
                                self.selection = Some(normalized);
                            } else {
                                self.selection = None;
                            }
                        }
                        self.drag_state = DragState::None;
                    }
                }
                DragState::MovingSelection { start_mouse, orig_rect } => {
                    desired_cursor = CursorIcon::Move;
                    let delta = current_pos - *start_mouse;
                    let mut new_rect = orig_rect.translate(delta);
                    new_rect = constrain_to_screen(new_rect, screen_rect);
                    self.selection = Some(new_rect);

                    if pointer.primary_released() {
                        self.drag_state = DragState::None;
                    }
                }
                DragState::ResizingSelection { handle, orig_rect } => {
                    let mut min = orig_rect.min;
                    let mut max = orig_rect.max;

                    match handle {
                        Handle::TopLeft => { min.x = current_pos.x; min.y = current_pos.y; }
                        Handle::Top => { min.y = current_pos.y; }
                        Handle::TopRight => { max.x = current_pos.x; min.y = current_pos.y; }
                        Handle::Right => { max.x = current_pos.x; }
                        Handle::BottomRight => { max.x = current_pos.x; max.y = current_pos.y; }
                        Handle::Bottom => { max.y = current_pos.y; }
                        Handle::BottomLeft => { min.x = current_pos.x; max.y = current_pos.y; }
                        Handle::Left => { min.x = current_pos.x; }
                    }

                    self.selection = Some(Rect::from_two_pos(min, max));

                    if pointer.primary_released() {
                        if let Some(s) = self.selection {
                            self.selection = Some(normalize_rect(s));
                        }
                        self.drag_state = DragState::None;
                    }
                }
                DragState::DrawingRect(start) => {
                    desired_cursor = CursorIcon::Crosshair;
                    if pointer.primary_released() {
                        let box_rect = normalize_rect(Rect::from_two_pos(*start, current_pos));
                        if box_rect.width() > 3.0 && box_rect.height() > 3.0 {
                            self.annotations.push(Annotation::Rectangle {
                                rect: box_rect,
                                color: active_color,
                                thickness: self.config.stroke_thickness,
                            });
                        }
                        self.drag_state = DragState::None;
                    }
                }
                DragState::DrawingArrow(start) => {
                    desired_cursor = CursorIcon::Crosshair;
                    if pointer.primary_released() {
                        if (current_pos - *start).length() > 6.0 {
                            self.annotations.push(Annotation::Arrow {
                                start: *start,
                                end: current_pos,
                                color: active_color,
                                thickness: self.config.stroke_thickness,
                            });
                        }
                        self.drag_state = DragState::None;
                    }
                }
                DragState::DrawingLine(start) => {
                    desired_cursor = CursorIcon::Crosshair;
                    if pointer.primary_released() {
                        if (current_pos - *start).length() > 3.0 {
                            self.annotations.push(Annotation::Line {
                                start: *start,
                                end: current_pos,
                                color: active_color,
                                thickness: self.config.stroke_thickness,
                            });
                        }
                        self.drag_state = DragState::None;
                    }
                }
                DragState::DrawingPath(points) => {
                    desired_cursor = CursorIcon::Crosshair;
                    if let Some(last) = points.last() {
                        if (current_pos - *last).length() >= 2.0 {
                            points.push(current_pos);
                        }
                    }
                    if pointer.primary_released() {
                        if points.len() >= 2 {
                            if self.current_tool == Tool::Marker {
                                let marker_color = Color32::from_rgba_unmultiplied(active_color.r(), active_color.g(), active_color.b(), 90);
                                self.annotations.push(Annotation::Marker {
                                    points: points.clone(),
                                    color: marker_color,
                                    thickness: self.config.stroke_thickness * 3.5,
                                });
                            } else {
                                self.annotations.push(Annotation::Pen {
                                    points: points.clone(),
                                    color: active_color,
                                    thickness: self.config.stroke_thickness,
                                });
                            }
                        }
                        self.drag_state = DragState::None;
                    }
                }
            }
        }

        ctx.set_cursor_icon(desired_cursor);

        // 5. Render Central Canvas
        egui::CentralPanel::default()
            .frame(egui::Frame::none())
            .show(ctx, |ui| {
                let painter = ui.painter();

                if let Some(tex) = &self.texture {
                    // Darkened desktop background using 50% multiply.
                    painter.image(
                        tex.id(),
                        screen_rect,
                        Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                        Color32::from_rgb(128, 128, 128),
                    );

                    // Un-dimmed crystal-clear selection window
                    if let Some(sel) = self.selection {
                        let uv_min = Pos2::new(
                            (sel.min.x - screen_rect.min.x) / screen_rect.width(),
                            (sel.min.y - screen_rect.min.y) / screen_rect.height(),
                        );
                        let uv_max = Pos2::new(
                            (sel.max.x - screen_rect.min.x) / screen_rect.width(),
                            (sel.max.y - screen_rect.min.y) / screen_rect.height(),
                        );

                        painter.image(tex.id(), sel, Rect::from_min_max(uv_min, uv_max), Color32::WHITE);

                        paint_marching_ants(painter, sel);

                        for pos in handle_positions(sel) {
                            paint_handle(painter, pos);
                        }

                        // Dimension badge, sitting just above the top-left corner.
                        let dim_text = format!(
                            "{}x{}",
                            sel.width().round() as i32,
                            sel.height().round() as i32
                        );
                        let galley = painter.layout_no_wrap(
                            dim_text,
                            egui::FontId::proportional(BADGE_FONT_SIZE),
                            Color32::WHITE,
                        );
                        let badge_size = galley.size() + Vec2::new(8.0, 4.0);
                        let badge_pos = Pos2::new(
                            sel.left(),
                            (sel.top() - badge_size.y - 3.0).max(0.0),
                        );
                        let badge_rect = Rect::from_min_size(badge_pos, badge_size);
                        painter.rect_filled(badge_rect, 0.0_f32, BADGE_BACKDROP);
                        painter.galley(badge_pos + Vec2::new(4.0, 2.0), galley, Color32::WHITE);
                    }
                }

                // Render all drawn annotations
                for ann in &self.annotations {
                    draw_annotation(painter, ann);
                }

                // Render live drawing preview
                match &self.drag_state {
                    DragState::DrawingRect(start) => {
                        let preview = normalize_rect(Rect::from_two_pos(*start, current_pos));
                        painter.rect_stroke(preview, 0.0_f32, Stroke::new(self.config.stroke_thickness, self.current_color()));
                    }
                    DragState::DrawingArrow(start) => {
                        draw_arrow(painter, *start, current_pos, self.current_color(), self.config.stroke_thickness);
                    }
                    DragState::DrawingLine(start) => {
                        painter.line_segment([*start, current_pos], Stroke::new(self.config.stroke_thickness, self.current_color()));
                    }
                    DragState::DrawingPath(points) => {
                        for window in points.windows(2) {
                            painter.line_segment([window[0], window[1]], Stroke::new(self.config.stroke_thickness, self.current_color()));
                        }
                    }
                    _ => {}
                }

                // Active inline text box
                if let Some(pos) = self.active_text_pos {
                    let mut text_active = true;
                    let text_rect = Rect::from_min_size(pos, Vec2::new(200.0, 26.0));
                    let builder = egui::UiBuilder::new().max_rect(text_rect);
                    ui.allocate_new_ui(builder, |ui| {
                        let res = ui.text_edit_singleline(&mut self.text_input);
                        res.request_focus();
                        if ui.input(|i| i.key_pressed(Key::Enter)) {
                            if !self.text_input.trim().is_empty() {
                                self.annotations.push(Annotation::Text {
                                    pos,
                                    text: self.text_input.clone(),
                                    color: self.current_color(),
                                    size: 16.0,
                                });
                            }
                            text_active = false;
                        }
                    });
                    if !text_active {
                        self.active_text_pos = None;
                    }
                }

                // Render toolbars when selection is active
                if let Some(sel) = self.selection {
                    if matches!(self.drag_state, DragState::None) {
                        if self.icons.is_none() {
                            self.icons = Some(ToolbarIcons::load(ctx));
                        }
                        self.render_lightshot_toolbars(ui, ctx, sel, screen_rect);
                    }
                }
            });

        if let Some(err) = &self.export_error {
            egui::Area::new(egui::Id::new("export_error"))
                .anchor(egui::Align2::CENTER_TOP, [0.0, 16.0])
                .show(ctx, |ui| {
                    egui::Frame::none()
                        .fill(Color32::from_rgba_unmultiplied(28, 28, 28, 220))
                        .inner_margin(egui::Margin::symmetric(12.0, 8.0))
                        .rounding(4.0)
                        .show(ui, |ui| {
                            ui.label(
                                egui::RichText::new(err)
                                    .color(Color32::from_rgb(255, 210, 210))
                                    .size(13.0),
                            );
                        });
                });
        }

        self.frame_count += 1;
        if self.frame_count == 1 && self.icons.is_none() {
            self.icons = Some(ToolbarIcons::load(ctx));
        }

        // Uncloak window on frame 2: frame 1 completes swap_buffers and GPU presentation.
        // DWM uncloaking reveals the presented frame instantly with zero flash or uninitialized buffer.
        if self.frame_count < 2 {
            ctx.request_repaint();
        } else if self.frame_count == 2 {
            #[cfg(windows)]
            unsafe {
                use windows_sys::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWA_CLOAK};
                use windows_sys::Win32::UI::WindowsAndMessaging::{
                    SetForegroundWindow, ShowWindow, SW_SHOW,
                };
                if self.hwnd != 0 {
                    let off: i32 = 0;
                    let _ = DwmSetWindowAttribute(
                        self.hwnd as _,
                        DWMWA_CLOAK as u32,
                        &off as *const _ as *const _,
                        std::mem::size_of::<i32>() as u32,
                    );
                    ShowWindow(self.hwnd as _, SW_SHOW);
                    SetForegroundWindow(self.hwnd as _);
                }
            }
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
            ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            eprintln!("[ZenShot PERF] hotkey→visible: {:?}", self.t0.elapsed());
        }
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 1.0]
    }
}

impl ZenShotApp {
    /// Renders floating toolbars (Horizontal & Vertical) for actions and annotation tools.
    fn render_lightshot_toolbars(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, sel: Rect, screen_rect: Rect) {
        enum ToolbarAction {
            None,
            Copy,
            Save,
            Print,
            Close,
            SelectTool(Tool),
            CycleColor,
            Undo,
        }

        let (h_rect, v_rect) = self.get_toolbar_rects(sel, screen_rect);
        let mut action = ToolbarAction::None;
        let current_tool = self.current_tool;
        let current_color = self.current_color();

        // --- 1. HORIZONTAL ACTION TOOLBAR (Bottom) ---
        // Draw with painter for exact Lightshot gradient (250,251,251) -> (211,214,217) + shadow
        let h_btn_size = H_ICON;
        paint_lightshot_toolbar(ui.painter(), h_rect, false);
        let h_builder = egui::UiBuilder::new().max_rect(h_rect.shrink(BAR_MARGIN));
        ui.allocate_new_ui(h_builder, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing = Vec2::new(BTN_GAP, 0.0);

                if let Some(icons) = &self.icons {
                    if icon_button(ui, &icons.print, h_btn_size, false, "Print (Ctrl+P)").clicked() {
                        action = ToolbarAction::Print;
                    }
                    if icon_button(ui, &icons.copy, h_btn_size, false, "Copy (Ctrl+C)").clicked() {
                        action = ToolbarAction::Copy;
                    }
                    if icon_button(ui, &icons.save, h_btn_size, false, "Save (Ctrl+S)").clicked() {
                        action = ToolbarAction::Save;
                    }
                    if icon_button(ui, &icons.close, h_btn_size, false, "Close (Ctrl+X)").clicked() {
                        action = ToolbarAction::Close;
                    }
                }
            });
        });

        // --- 2. VERTICAL DRAWING TOOLBAR (Right) ---
        let v_btn_size = V_ICON;
        paint_lightshot_toolbar(ui.painter(), v_rect, true);
        let v_builder = egui::UiBuilder::new().max_rect(v_rect.shrink(BAR_MARGIN));
        ui.allocate_new_ui(v_builder, |ui| {
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing = Vec2::new(0.0, BTN_GAP);

                if let Some(icons) = &self.icons {
                    if icon_button(ui, &icons.pen, v_btn_size, current_tool == Tool::Pen, "Pen").clicked() {
                        action = ToolbarAction::SelectTool(if current_tool == Tool::Pen { Tool::Select } else { Tool::Pen });
                    }
                    if icon_button(ui, &icons.line, v_btn_size, current_tool == Tool::Line, "Line").clicked() {
                        action = ToolbarAction::SelectTool(if current_tool == Tool::Line { Tool::Select } else { Tool::Line });
                    }
                    if icon_button(ui, &icons.arrow, v_btn_size, current_tool == Tool::Arrow, "Arrow").clicked() {
                        action = ToolbarAction::SelectTool(if current_tool == Tool::Arrow { Tool::Select } else { Tool::Arrow });
                    }
                    if icon_button(ui, &icons.rect, v_btn_size, current_tool == Tool::Rectangle, "Rectangle").clicked() {
                        action = ToolbarAction::SelectTool(if current_tool == Tool::Rectangle { Tool::Select } else { Tool::Rectangle });
                    }
                    if icon_button(ui, &icons.marker, v_btn_size, current_tool == Tool::Marker, "Marker").clicked() {
                        action = ToolbarAction::SelectTool(if current_tool == Tool::Marker { Tool::Select } else { Tool::Marker });
                    }
                    if icon_button(ui, &icons.text, v_btn_size, current_tool == Tool::Text, "Text").clicked() {
                        action = ToolbarAction::SelectTool(if current_tool == Tool::Text { Tool::Select } else { Tool::Text });
                    }

                    // Colour swatch: swatch frame tinted with
                    // the active colour, sized to line up with the icons.
                    let cell = v_btn_size + Vec2::splat(2.0 * BTN_PAD);
                    let (rect, resp) = ui.allocate_exact_size(cell, egui::Sense::click());
                    let border = Stroke::new(1.0_f32, Color32::from_rgb(166, 178, 190));
                    if resp.hovered() || resp.is_pointer_button_down_on() {
                        ui.painter().rect_filled(rect, 2.0, Color32::from_white_alpha(150));
                        ui.painter().rect_stroke(rect, 2.0, border);
                    }
                    // Fill the chip underneath the frame.
                    ui.painter().rect_filled(rect.shrink(2.0 * BTN_PAD), 2.0, current_color);
                    egui::Image::new(&icons.color)
                        .fit_to_exact_size(v_btn_size)
                        .rounding(2.0)
                        .paint_at(ui, rect);
                    if resp.on_hover_text("Color (click to cycle)").clicked() {
                        action = ToolbarAction::CycleColor;
                    }

                    if icon_button(ui, &icons.undo, v_btn_size, false, "Undo (Ctrl+Z)").clicked() {
                        action = ToolbarAction::Undo;
                    }
                }
            });
        });

        // --- 3. APPLY ACTIONS SAFELY AFTER UI RENDERING ---
        match action {
            ToolbarAction::None => {}
            ToolbarAction::Copy => self.action_copy(ctx, screen_rect),
            ToolbarAction::Save => self.action_save(ctx, screen_rect),
            ToolbarAction::Close => self.quit(ctx),
            ToolbarAction::Print => self.action_print(ctx, screen_rect),
            ToolbarAction::SelectTool(tool) => {
                self.current_tool = tool;
            }
            ToolbarAction::CycleColor => {
                self.cycle_color();
            }
            ToolbarAction::Undo => {
                self.annotations.pop();
            }
        }
    }
}

/// Sends a PNG on disk to the default printer via the platform's print helper.
fn spawn_print_job(path: &std::path::Path) -> std::io::Result<()> {
    #[cfg(target_os = "windows")]
    let mut cmd = {
        let mut c = std::process::Command::new("mspaint");
        c.arg("/p").arg(path);
        c
    };

    #[cfg(not(target_os = "windows"))]
    let mut cmd = {
        let mut c = std::process::Command::new("lp");
        c.arg(path);
        c
    };

    cmd.stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map(|_| ())
}

/// Fits `rect` inside `screen`. Oversized selections (restored after a
/// resolution change) are shrunk; inverted clamp ranges never reach `f32::clamp`.
pub fn constrain_to_screen(rect: Rect, screen: Rect) -> Rect {
    let rect = normalize_rect(rect);
    let w = rect.width().min(screen.width()).max(1.0);
    let h = rect.height().min(screen.height()).max(1.0);
    let max_x = (screen.max.x - w).max(screen.min.x);
    let max_y = (screen.max.y - h).max(screen.min.y);
    let x = if screen.min.x <= max_x {
        rect.min.x.clamp(screen.min.x, max_x)
    } else {
        screen.min.x
    };
    let y = if screen.min.y <= max_y {
        rect.min.y.clamp(screen.min.y, max_y)
    } else {
        screen.min.y
    };
    Rect::from_min_size(Pos2::new(x, y), Vec2::new(w, h))
}

/// Normalizes a rectangle ensuring min <= max.
pub fn normalize_rect(r: Rect) -> Rect {
    let min_x = r.min.x.min(r.max.x);
    let max_x = r.min.x.max(r.max.x);
    let min_y = r.min.y.min(r.max.y);
    let max_y = r.min.y.max(r.max.y);
    Rect::from_min_max(Pos2::new(min_x, min_y), Pos2::new(max_x, max_y))
}

/// Draws an annotation on the egui canvas.
fn draw_annotation(painter: &egui::Painter, ann: &Annotation) {
    match ann {
        Annotation::Rectangle { rect, color, thickness } => {
            painter.rect_stroke(*rect, 0.0_f32, Stroke::new(*thickness, *color));
        }
        Annotation::Arrow { start, end, color, thickness } => {
            draw_arrow(painter, *start, *end, *color, *thickness);
        }
        Annotation::Line { start, end, color, thickness } => {
            painter.line_segment([*start, *end], Stroke::new(*thickness, *color));
        }
        Annotation::Pen { points, color, thickness } => {
            for window in points.windows(2) {
                painter.line_segment([window[0], window[1]], Stroke::new(*thickness, *color));
            }
        }
        Annotation::Marker { points, color, thickness } => {
            for window in points.windows(2) {
                painter.line_segment([window[0], window[1]], Stroke::new(*thickness, *color));
            }
        }
        Annotation::Text { pos, text, color, size } => {
            painter.text(*pos, egui::Align2::LEFT_TOP, text, egui::FontId::proportional(*size), *color);
        }
    }
}

/// Draws an arrow with calculated geometric head.
fn draw_arrow(painter: &egui::Painter, start: Pos2, end: Pos2, color: Color32, thickness: f32) {
    painter.line_segment([start, end], Stroke::new(thickness, color));

    let dir = end - start;
    let len = dir.length();
    if len > 8.0 {
        let norm = dir / len;
        let head_size = (thickness * 4.0).clamp(10.0, 22.0);
        let perp = Vec2::new(-norm.y, norm.x) * (head_size * 0.45);
        let arrow_left = end - norm * head_size + perp;
        let arrow_right = end - norm * head_size - perp;

        painter.line_segment([end, arrow_left], Stroke::new(thickness, color));
        painter.line_segment([end, arrow_right], Stroke::new(thickness, color));
    }
}

/// Burns all annotations onto cropped image pixels directly in RAM.
pub fn burn_and_crop(
    orig: &RgbaImage,
    screen_rect: Rect,
    selection: Rect,
    annotations: &[Annotation],
    ctx: &egui::Context,
) -> RgbaImage {
    let scale_x = orig.width() as f32 / screen_rect.width();
    let scale_y = orig.height() as f32 / screen_rect.height();

    let sel_min_x = ((selection.min.x - screen_rect.min.x) * scale_x).round().max(0.0) as u32;
    let sel_min_y = ((selection.min.y - screen_rect.min.y) * scale_y).round().max(0.0) as u32;
    let sel_max_x = ((selection.max.x - screen_rect.min.x) * scale_x).round().min(orig.width() as f32) as u32;
    let sel_max_y = ((selection.max.y - screen_rect.min.y) * scale_y).round().min(orig.height() as f32) as u32;

    let width = sel_max_x.saturating_sub(sel_min_x).max(1);
    let height = sel_max_y.saturating_sub(sel_min_y).max(1);

    let mut cropped = image::imageops::crop_imm(orig, sel_min_x, sel_min_y, width, height).to_image();

    // Each annotation is accumulated into a coverage mask and blended in one
    // pass. Blending per segment instead would darken every stroke overlap and
    // every joint, which is very visible with the translucent marker.
    let mut mask = vec![false; (width * height) as usize];
    let size = (width, height);
    let to_local = |p: Pos2| {
        (
            ((p.x - selection.min.x) * scale_x).round() as i32,
            ((p.y - selection.min.y) * scale_y).round() as i32,
        )
    };

    for ann in annotations {
        mask.fill(false);

        match ann {
            Annotation::Rectangle { rect, color, thickness } => {
                let (min_x, min_y) = to_local(rect.min);
                let (max_x, max_y) = to_local(rect.max);
                if max_x <= min_x || max_y <= min_y {
                    continue;
                }
                let t = stroke_px(*thickness, scale_x);
                mask_line(&mut mask, size, (min_x, min_y), (max_x, min_y), t);
                mask_line(&mut mask, size, (max_x, min_y), (max_x, max_y), t);
                mask_line(&mut mask, size, (max_x, max_y), (min_x, max_y), t);
                mask_line(&mut mask, size, (min_x, max_y), (min_x, min_y), t);
                blend_mask(&mut cropped, &mask, *color);
            }
            Annotation::Arrow { start, end, color, thickness } => {
                let (sx, sy) = to_local(*start);
                let (ex, ey) = to_local(*end);
                let t = stroke_px(*thickness, scale_x);
                mask_line(&mut mask, size, (sx, sy), (ex, ey), t);

                let dx = (ex - sx) as f32;
                let dy = (ey - sy) as f32;
                let len = (dx * dx + dy * dy).sqrt();
                if len > 6.0 {
                    let (nx, ny) = (dx / len, dy / len);
                    let head = (t as f32 * 4.0).clamp(10.0, 22.0);
                    let (px, py) = (-ny * head * 0.45, nx * head * 0.45);

                    let lx = (ex as f32 - nx * head + px).round() as i32;
                    let ly = (ey as f32 - ny * head + py).round() as i32;
                    let rx = (ex as f32 - nx * head - px).round() as i32;
                    let ry = (ey as f32 - ny * head - py).round() as i32;

                    mask_line(&mut mask, size, (ex, ey), (lx, ly), t);
                    mask_line(&mut mask, size, (ex, ey), (rx, ry), t);
                }
                blend_mask(&mut cropped, &mask, *color);
            }
            Annotation::Line { start, end, color, thickness } => {
                let t = stroke_px(*thickness, scale_x);
                mask_line(&mut mask, size, to_local(*start), to_local(*end), t);
                blend_mask(&mut cropped, &mask, *color);
            }
            Annotation::Pen { points, color, thickness }
            | Annotation::Marker { points, color, thickness } => {
                let t = stroke_px(*thickness, scale_x);
                for window in points.windows(2) {
                    mask_line(&mut mask, size, to_local(window[0]), to_local(window[1]), t);
                }
                blend_mask(&mut cropped, &mask, *color);
            }
            Annotation::Text { pos, text, color, size } => {
                let origin = *pos - selection.min.to_vec2();
                let scale = Vec2::new(scale_x, scale_y);
                burn_text(&mut cropped, ctx, origin, (text, *color, *size), scale);
            }
        }
    }

    cropped
}

/// Stroke width in image pixels, never thinner than one pixel.
fn stroke_px(thickness: f32, scale: f32) -> i32 {
    (thickness * scale).round().max(1.0) as i32
}

/// Marks a thick Bresenham line into a `w` x `h` coverage mask.
fn mask_line(
    mask: &mut [bool],
    (w, h): (u32, u32),
    (x0, y0): (i32, i32),
    (x1, y1): (i32, i32),
    thickness: i32,
) {
    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let step_x = if x0 < x1 { 1 } else { -1 };
    let step_y = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;

    let mut curr_x = x0;
    let mut curr_y = y0;
    let radius = (thickness / 2).max(0);

    loop {
        for oy in -radius..=radius {
            for ox in -radius..=radius {
                let px = curr_x + ox;
                let py = curr_y + oy;
                if px >= 0 && px < w as i32 && py >= 0 && py < h as i32 {
                    mask[py as usize * w as usize + px as usize] = true;
                }
            }
        }

        if curr_x == x1 && curr_y == y1 {
            break;
        }

        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            curr_x += step_x;
        }
        if e2 <= dx {
            err += dx;
            curr_y += step_y;
        }
    }
}

/// Composites a single colour over every pixel the mask covers.
fn blend_mask(img: &mut RgbaImage, mask: &[bool], color: Color32) {
    let w = img.width();
    for (idx, covered) in mask.iter().enumerate() {
        if !covered {
            continue;
        }
        let idx = idx as u32;
        blend_pixel(img, idx % w, idx / w, color, 1.0);
    }
}

/// Source-over blend of `color` at `coverage` onto one opaque pixel.
fn blend_pixel(img: &mut RgbaImage, x: u32, y: u32, color: Color32, coverage: f32) {
    let alpha = coverage * (color.a() as f32 / 255.0);
    if alpha <= 0.0 {
        return;
    }
    let inv = 1.0 - alpha;
    let dst = img.get_pixel_mut(x, y);
    dst.0[0] = (color.r() as f32 * alpha + dst.0[0] as f32 * inv).round() as u8;
    dst.0[1] = (color.g() as f32 * alpha + dst.0[1] as f32 * inv).round() as u8;
    dst.0[2] = (color.b() as f32 * alpha + dst.0[2] as f32 * inv).round() as u8;
    dst.0[3] = 255;
}

/// Burns laid-out text by sampling coverage straight out of egui's font atlas,
/// so the saved PNG uses the same glyphs the overlay previewed.
fn burn_text(
    img: &mut RgbaImage,
    ctx: &egui::Context,
    origin: Pos2,
    (text, color, size): (&str, Color32, f32),
    scale: Vec2,
) {
    let (scale_x, scale_y) = (scale.x, scale.y);
    let galley =
        ctx.fonts(|f| f.layout_no_wrap(text.to_owned(), egui::FontId::proportional(size), color));
    let atlas = ctx.fonts(|f| f.image());
    let atlas_w = atlas.size[0];

    for row in &galley.rows {
        for glyph in &row.glyphs {
            let uv = glyph.uv_rect;
            if uv.is_nothing() {
                continue;
            }
            let src_w = u32::from(uv.max[0] - uv.min[0]);
            let src_h = u32::from(uv.max[1] - uv.min[1]);
            if src_w == 0 || src_h == 0 {
                continue;
            }

            let top_left = origin + glyph.pos.to_vec2() + uv.offset;
            let dst_x = (top_left.x * scale_x).round() as i32;
            let dst_y = (top_left.y * scale_y).round() as i32;
            let dst_w = (uv.size.x * scale_x).round().max(1.0) as u32;
            let dst_h = (uv.size.y * scale_y).round().max(1.0) as u32;

            for row_px in 0..dst_h {
                let y = dst_y + row_px as i32;
                if y < 0 || y >= img.height() as i32 {
                    continue;
                }
                let src_y = u32::from(uv.min[1]) + row_px * src_h / dst_h;

                for col_px in 0..dst_w {
                    let x = dst_x + col_px as i32;
                    if x < 0 || x >= img.width() as i32 {
                        continue;
                    }
                    let src_x = u32::from(uv.min[0]) + col_px * src_w / dst_w;
                    let coverage = atlas.pixels[src_y as usize * atlas_w + src_x as usize];
                    blend_pixel(img, x as u32, y as u32, color, coverage.min(1.0));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;

    #[test]
    fn test_normalize_rect_handles_inverted_drag() {
        let inverted = Rect::from_min_max(Pos2::new(500.0, 400.0), Pos2::new(100.0, 50.0));
        let norm = normalize_rect(inverted);
        assert_eq!(norm.min, Pos2::new(100.0, 50.0));
        assert_eq!(norm.max, Pos2::new(500.0, 400.0));
        assert_eq!(norm.width(), 400.0);
        assert_eq!(norm.height(), 350.0);
    }

    #[test]
    fn oversized_selection_is_clamped_without_panic() {
        let screen = Rect::from_min_size(Pos2::ZERO, Vec2::new(100.0, 80.0));
        let huge = Rect::from_min_size(Pos2::new(-20.0, -10.0), Vec2::new(400.0, 300.0));
        let fit = constrain_to_screen(huge, screen);
        assert!(fit.width() <= screen.width() + f32::EPSILON);
        assert!(fit.height() <= screen.height() + f32::EPSILON);
        assert!(fit.min.x >= screen.min.x);
        assert!(fit.min.y >= screen.min.y);
        assert!(fit.max.x <= screen.max.x + 0.01);
        assert!(fit.max.y <= screen.max.y + 0.01);
    }

    fn assert_bar_ok(bar: Rect, safe: Rect) {
        assert!(bar.width() > 0.0 && bar.height() > 0.0);
        assert!(bar.min.x + 0.01 >= safe.min.x);
        assert!(bar.min.y + 0.01 >= safe.min.y);
        assert!(bar.max.x <= safe.max.x + 0.01);
        assert!(bar.max.y <= safe.max.y + 0.01);
    }

    #[test]
    fn fullscreen_toolbars_stay_above_taskbar_and_do_not_overlap() {
        let screen = Rect::from_min_size(Pos2::ZERO, Vec2::new(1920.0, 1080.0));
        let safe = toolbar_safe_bounds(screen, screen);
        let (h, v) = layout_toolbars(screen, safe);
        assert_bar_ok(h, safe);
        assert_bar_ok(v, safe);
        assert!(!h.intersects(v), "h={h:?} overlaps v={v:?}");
    }

    #[test]
    fn mid_screen_selection_keeps_toolbars_outside() {
        let screen = Rect::from_min_size(Pos2::ZERO, Vec2::new(1920.0, 1080.0));
        let sel = Rect::from_min_size(Pos2::new(400.0, 300.0), Vec2::new(240.0, 180.0));
        let safe = toolbar_safe_bounds(sel, screen);
        let (h, v) = layout_toolbars(sel, safe);
        assert_bar_ok(h, safe);
        assert_bar_ok(v, safe);
        assert!(!h.intersects(v));
        assert!(h.min.y >= sel.max.y);
        assert!(v.min.x >= sel.max.x);
    }

    #[test]
    fn test_burn_and_crop_draws_box_onto_pixels() {
        let mut orig = RgbaImage::new(200, 200);
        for pixel in orig.pixels_mut() {
            *pixel = Rgba([0, 0, 0, 255]);
        }

        let screen_rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(200.0, 200.0));
        let selection = Rect::from_min_size(Pos2::new(10.0, 10.0), Vec2::new(100.0, 100.0));

        let box_annotation = Annotation::Rectangle {
            rect: Rect::from_min_size(Pos2::new(20.0, 20.0), Vec2::new(40.0, 40.0)),
            color: Color32::from_rgb(255, 0, 0),
            thickness: 2.0,
        };

        let cropped = burn_and_crop(&orig, screen_rect, selection, &[box_annotation], &test_ctx());

        assert_eq!(cropped.width(), 100);
        assert_eq!(cropped.height(), 100);

        let border_pixel = cropped.get_pixel(10, 10);
        assert_eq!(*border_pixel, Rgba([255, 0, 0, 255]));

        let bg_pixel = cropped.get_pixel(0, 0);
        assert_eq!(*bg_pixel, Rgba([0, 0, 0, 255]));
    }

    #[test]
    fn test_burn_and_crop_draws_arrow_onto_pixels() {
        let mut orig = RgbaImage::new(100, 100);
        for pixel in orig.pixels_mut() {
            *pixel = Rgba([0, 0, 0, 255]);
        }

        let screen_rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(100.0, 100.0));
        let selection = Rect::from_min_size(Pos2::ZERO, Vec2::new(100.0, 100.0));

        let arrow = Annotation::Arrow {
            start: Pos2::new(10.0, 10.0),
            end: Pos2::new(50.0, 10.0),
            color: Color32::from_rgb(0, 255, 0),
            thickness: 2.0,
        };

        let cropped = burn_and_crop(&orig, screen_rect, selection, &[arrow], &test_ctx());
        let line_pixel = cropped.get_pixel(30, 10);
        assert_eq!(*line_pixel, Rgba([0, 255, 0, 255]));
    }

    #[test]
    fn test_burn_and_crop_burns_text_into_pixels() {
        let mut orig = RgbaImage::new(200, 100);
        for pixel in orig.pixels_mut() {
            *pixel = Rgba([0, 0, 0, 255]);
        }

        let screen_rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(200.0, 100.0));
        let selection = Rect::from_min_size(Pos2::ZERO, Vec2::new(200.0, 100.0));

        let text = Annotation::Text {
            pos: Pos2::new(10.0, 10.0),
            text: "ZenShot".to_string(),
            color: Color32::from_rgb(255, 0, 0),
            size: 24.0,
        };

        let cropped = burn_and_crop(&orig, screen_rect, selection, &[text], &test_ctx());

        let painted = cropped.pixels().filter(|p| p.0[0] > 0).count();
        assert!(painted > 0, "text annotation was not burned into the image");
    }

    /// Scanline colours sampled from the toolbar strip to verify gradient consistency.
    #[test]
    fn test_toolbar_gradient_matches_expected() {
        let expected: [(i32, [u8; 3]); 9] = [
            (1, [250, 251, 251]),
            (2, [245, 247, 248]),
            (3, [237, 240, 243]),
            (4, [232, 236, 239]),
            (10, [226, 230, 233]),
            (15, [220, 224, 227]),
            (18, [217, 221, 223]),
            (25, [211, 214, 217]),
            (26, [203, 206, 208]),
        ];

        for (row, want) in expected {
            let got = toolbar_scanline(row, 29);
            let got = [got.r(), got.g(), got.b()];
            for channel in 0..3 {
                let delta = got[channel].abs_diff(want[channel]);
                assert!(
                    delta <= 2,
                    "row {row} channel {channel}: got {got:?}, want {want:?}"
                );
            }
        }
    }

    /// Fonts are only available after the context has run a frame.
    fn test_ctx() -> egui::Context {
        let ctx = egui::Context::default();
        let _ = ctx.run(egui::RawInput::default(), |_| {});
        ctx
    }
}
