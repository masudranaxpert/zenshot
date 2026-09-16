use crate::clipboard::copy_to_clipboard;
use crate::config::Config;
use chrono::Local;
use eframe::egui::{self, Color32, CursorIcon, Key, Pos2, Rect, Stroke, Vec2};
use image::{Rgba, RgbaImage};
use std::fs;

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
    Pen {
        points: Vec<Pos2>,
        color: Color32,
        thickness: f32,
    },
}

/// Active editing tool.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    Select,
    Rectangle,
    Arrow,
    Pen,
}

/// Preset color palette.
pub const PRESET_COLORS: [Color32; 5] = [
    Color32::from_rgb(239, 68, 68),  // Red
    Color32::from_rgb(59, 130, 246), // Blue
    Color32::from_rgb(16, 185, 129), // Green
    Color32::from_rgb(245, 158, 11), // Yellow
    Color32::from_rgb(168, 85, 247), // Purple
];

/// Current drag interaction state.
#[derive(Debug, Clone)]
enum DragState {
    None,
    CreatingSelection(Pos2),
    MovingSelection { start_mouse: Pos2, orig_rect: Rect },
    ResizingSelection { handle: Handle, orig_rect: Rect },
    DrawingRect(Pos2),
    DrawingArrow(Pos2),
    DrawingPen(Vec<Pos2>),
}

/// Main application state for ZenShot.
pub struct ZenShotApp {
    config: Config,
    screen_image: RgbaImage,
    texture: Option<egui::TextureHandle>,
    selection: Option<Rect>,
    drag_state: DragState,
    current_tool: Tool,
    active_color: Color32,
    annotations: Vec<Annotation>,
    current_mouse: Option<Pos2>,
}

impl ZenShotApp {
    pub fn new(config: Config, screen_image: RgbaImage) -> Self {
        let active_color = Color32::from_rgb(
            config.stroke_color[0],
            config.stroke_color[1],
            config.stroke_color[2],
        );

        Self {
            config,
            screen_image,
            texture: None,
            selection: None,
            drag_state: DragState::None,
            current_tool: Tool::Select,
            active_color,
            annotations: Vec::new(),
            current_mouse: None,
        }
    }

    /// Performs in-memory crop with burned annotations.
    fn crop_current_selection(&self, screen_rect: Rect) -> Option<RgbaImage> {
        let sel = self.selection?;
        Some(burn_and_crop(&self.screen_image, screen_rect, sel, &self.annotations))
    }

    /// Saves cropped image to configured path and exits immediately.
    fn action_save(&mut self, ctx: &egui::Context, screen_rect: Rect) {
        if let Some(img) = self.crop_current_selection(screen_rect) {
            let save_dir = self.config.resolve_save_dir();
            let _ = fs::create_dir_all(&save_dir);
            let filename = Local::now().format(&self.config.filename_format).to_string();
            let dest_path = save_dir.join(filename);
            let _ = img.save(&dest_path);
        }
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }

    /// Copies cropped image directly to clipboard in RAM and exits immediately.
    fn action_copy(&mut self, ctx: &egui::Context, screen_rect: Rect) {
        if let Some(img) = self.crop_current_selection(screen_rect) {
            let _ = copy_to_clipboard(&img);
        }
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }

    /// Checks if mouse point hits any of the 8 selection handles.
    fn hit_test_handles(&self, sel: Rect, point: Pos2) -> Option<Handle> {
        const HANDLE_RADIUS: f32 = 9.0;
        let handles = [
            (Handle::TopLeft, sel.left_top()),
            (Handle::Top, Pos2::new(sel.center().x, sel.top())),
            (Handle::TopRight, sel.right_top()),
            (Handle::Right, Pos2::new(sel.right(), sel.center().y)),
            (Handle::BottomRight, sel.right_bottom()),
            (Handle::Bottom, Pos2::new(sel.center().x, sel.bottom())),
            (Handle::BottomLeft, sel.left_bottom()),
            (Handle::Left, Pos2::new(sel.left(), sel.center().y)),
        ];

        for (handle, pos) in handles {
            if (point - pos).length() <= HANDLE_RADIUS {
                return Some(handle);
            }
        }
        None
    }
}

impl eframe::App for ZenShotApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Load captured screen into GPU texture on first frame
        if self.texture.is_none() {
            let size = [self.screen_image.width() as _, self.screen_image.height() as _];
            let pixels = self.screen_image.as_flat_samples();
            let color_image = egui::ColorImage::from_rgba_unmultiplied(size, pixels.as_slice());
            self.texture = Some(ctx.load_texture("desktop", color_image, egui::TextureOptions::LINEAR));
        }

        let screen_rect = ctx.screen_rect();

        // Keyboard shortcuts
        if ctx.input(|i| i.key_pressed(Key::Escape)) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }
        if ctx.input(|i| (i.modifiers.command || i.modifiers.ctrl) && i.key_pressed(Key::C)) {
            self.action_copy(ctx, screen_rect);
            return;
        }
        if ctx.input(|i| (i.modifiers.command || i.modifiers.ctrl) && i.key_pressed(Key::S)) {
            self.action_save(ctx, screen_rect);
            return;
        }
        if ctx.input(|i| (i.modifiers.command || i.modifiers.ctrl) && i.key_pressed(Key::Z)) {
            self.annotations.pop();
        }
        if ctx.input(|i| i.key_pressed(Key::R)) {
            self.current_tool = if self.current_tool == Tool::Rectangle { Tool::Select } else { Tool::Rectangle };
        }
        if ctx.input(|i| i.key_pressed(Key::A)) {
            self.current_tool = if self.current_tool == Tool::Arrow { Tool::Select } else { Tool::Arrow };
        }
        if ctx.input(|i| i.key_pressed(Key::P)) {
            self.current_tool = if self.current_tool == Tool::Pen { Tool::Select } else { Tool::Pen };
        }

        // Pointer state
        let pointer = ctx.input(|i| i.pointer.clone());
        let current_pos = pointer.hover_pos().unwrap_or(Pos2::ZERO);
        self.current_mouse = Some(current_pos);

        let mut desired_cursor = CursorIcon::Crosshair;

        // Interaction state machine
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
                            self.drag_state = DragState::ResizingSelection {
                                handle,
                                orig_rect: sel,
                            };
                        }
                    } else if sel.contains(current_pos) {
                        match self.current_tool {
                            Tool::Select => {
                                desired_cursor = CursorIcon::Move;
                                if pointer.primary_pressed() {
                                    self.drag_state = DragState::MovingSelection {
                                        start_mouse: current_pos,
                                        orig_rect: sel,
                                    };
                                }
                            }
                            Tool::Rectangle => {
                                desired_cursor = CursorIcon::Crosshair;
                                if pointer.primary_pressed() {
                                    self.drag_state = DragState::DrawingRect(current_pos);
                                }
                            }
                            Tool::Arrow => {
                                desired_cursor = CursorIcon::Crosshair;
                                if pointer.primary_pressed() {
                                    self.drag_state = DragState::DrawingArrow(current_pos);
                                }
                            }
                            Tool::Pen => {
                                desired_cursor = CursorIcon::Crosshair;
                                if pointer.primary_pressed() {
                                    self.drag_state = DragState::DrawingPen(vec![current_pos]);
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

                let clamped_x = new_rect.min.x.clamp(screen_rect.min.x, screen_rect.max.x - new_rect.width());
                let clamped_y = new_rect.min.y.clamp(screen_rect.min.y, screen_rect.max.y - new_rect.height());
                new_rect = Rect::from_min_size(Pos2::new(clamped_x, clamped_y), new_rect.size());

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
                            color: self.active_color,
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
                            color: self.active_color,
                            thickness: self.config.stroke_thickness,
                        });
                    }
                    self.drag_state = DragState::None;
                }
            }
            DragState::DrawingPen(points) => {
                desired_cursor = CursorIcon::Crosshair;
                if let Some(last) = points.last() {
                    if (current_pos - *last).length() >= 2.0 {
                        points.push(current_pos);
                    }
                }
                if pointer.primary_released() {
                    if points.len() >= 2 {
                        self.annotations.push(Annotation::Pen {
                            points: points.clone(),
                            color: self.active_color,
                            thickness: self.config.stroke_thickness,
                        });
                    }
                    self.drag_state = DragState::None;
                }
            }
        }

        ctx.set_cursor_icon(desired_cursor);

        // Rendering panel
        egui::CentralPanel::default()
            .frame(egui::Frame::none())
            .show(ctx, |ui| {
                let painter = ui.painter();

                if let Some(tex) = &self.texture {
                    // Step 1: Dimmed backdrop of the whole desktop
                    painter.image(
                        tex.id(),
                        screen_rect,
                        Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                        Color32::from_rgba_unmultiplied(90, 90, 95, 255),
                    );

                    // Step 2: Clear, un-dimmed view inside the selection
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

                        // Crisp selection outline
                        painter.rect_stroke(sel, 0.0_f32, Stroke::new(1.5_f32, Color32::from_rgb(56, 189, 248)));

                        // 8 Handles: circular with subtle shadow
                        let handle_positions = [
                            sel.left_top(),
                            Pos2::new(sel.center().x, sel.top()),
                            sel.right_top(),
                            Pos2::new(sel.right(), sel.center().y),
                            sel.right_bottom(),
                            Pos2::new(sel.center().x, sel.bottom()),
                            sel.left_bottom(),
                            Pos2::new(sel.left(), sel.center().y),
                        ];
                        for pos in handle_positions {
                            painter.circle_filled(pos, 5.0_f32, Color32::WHITE);
                            painter.circle_stroke(pos, 5.0_f32, Stroke::new(1.5_f32, Color32::from_rgb(2, 132, 199)));
                        }

                        // Dimension Pill Badge: W × H px
                        let dim_text = format!("{} × {} px", sel.width().round() as i32, sel.height().round() as i32);
                        let badge_pos = Pos2::new(sel.left(), (sel.top() - 26.0).max(6.0));
                        let font_id = egui::FontId::monospace(12.0);
                        let galley = painter.layout_no_wrap(dim_text, font_id, Color32::from_rgb(241, 245, 249));
                        let badge_rect = Rect::from_min_size(badge_pos, galley.size() + Vec2::new(14.0, 6.0));
                        painter.rect_filled(badge_rect, 4.0_f32, Color32::from_black_alpha(220));
                        painter.rect_stroke(badge_rect, 4.0_f32, Stroke::new(1.0_f32, Color32::from_rgb(51, 65, 85)));
                        painter.galley(badge_pos + Vec2::new(7.0, 3.0), galley, Color32::WHITE);
                    }
                }

                // Step 3: Draw existing annotations
                for ann in &self.annotations {
                    draw_annotation(painter, ann);
                }

                // Step 4: Draw active drawing previews
                match &self.drag_state {
                    DragState::DrawingRect(start) => {
                        let preview = normalize_rect(Rect::from_two_pos(*start, current_pos));
                        painter.rect_stroke(preview, 0.0_f32, Stroke::new(self.config.stroke_thickness, self.active_color));
                    }
                    DragState::DrawingArrow(start) => {
                        draw_arrow(painter, *start, current_pos, self.active_color, self.config.stroke_thickness);
                    }
                    DragState::DrawingPen(points) => {
                        for window in points.windows(2) {
                            painter.line_segment([window[0], window[1]], Stroke::new(self.config.stroke_thickness, self.active_color));
                        }
                    }
                    _ => {}
                }

                // Step 5: Floating Designer Toolbar
                if let Some(sel) = self.selection {
                    if matches!(self.drag_state, DragState::None) {
                        self.render_toolbar(ui, ctx, sel, screen_rect);
                    }
                }
            });
    }
}

impl ZenShotApp {
    /// Renders floating toolbar anchored to the selection rectangle.
    fn render_toolbar(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, sel: Rect, screen_rect: Rect) {
        let bar_width = 460.0;
        let bar_height = 42.0;

        // Smart placement: below selection, or flips above/inside if close to screen border
        let mut x = sel.right() - bar_width;
        if x < screen_rect.left() + 10.0 {
            x = screen_rect.left() + 10.0;
        }

        let mut y = sel.bottom() + 10.0;
        if y + bar_height > screen_rect.bottom() - 10.0 {
            y = sel.bottom() - bar_height - 10.0;
        }

        let toolbar_rect = Rect::from_min_size(Pos2::new(x, y), Vec2::new(bar_width, bar_height));

        let builder = egui::UiBuilder::new().max_rect(toolbar_rect);
        ui.allocate_new_ui(builder, |ui| {
            egui::Frame::popup(ui.style())
                .fill(Color32::from_rgb(24, 24, 27))
                .stroke(Stroke::new(1.0_f32, Color32::from_rgb(63, 63, 70)))
                .rounding(8.0)
                .inner_margin(6.0)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        // Tool 1: Rectangle Box
                        let is_rect = self.current_tool == Tool::Rectangle;
                        if ui.selectable_label(is_rect, "▢ Box").on_hover_text("Rectangle Box (R)").clicked() {
                            self.current_tool = if is_rect { Tool::Select } else { Tool::Rectangle };
                        }

                        // Tool 2: Arrow
                        let is_arrow = self.current_tool == Tool::Arrow;
                        if ui.selectable_label(is_arrow, "➔ Arrow").on_hover_text("Arrow Tool (A)").clicked() {
                            self.current_tool = if is_arrow { Tool::Select } else { Tool::Arrow };
                        }

                        // Tool 3: Pen
                        let is_pen = self.current_tool == Tool::Pen;
                        if ui.selectable_label(is_pen, "✎ Pen").on_hover_text("Freehand Pen (P)").clicked() {
                            self.current_tool = if is_pen { Tool::Select } else { Tool::Pen };
                        }

                        ui.separator();

                        // Color Palette Dots
                        for color in PRESET_COLORS {
                            let (rect, resp) = ui.allocate_exact_size(Vec2::splat(16.0), egui::Sense::click());
                            let is_active = self.active_color == color;
                            ui.painter().circle_filled(rect.center(), 7.0_f32, color);
                            if is_active {
                                ui.painter().circle_stroke(rect.center(), 9.0_f32, Stroke::new(2.0_f32, Color32::WHITE));
                            }
                            if resp.clicked() {
                                self.active_color = color;
                            }
                        }

                        ui.separator();

                        // Undo Button
                        let undo_enabled = !self.annotations.is_empty();
                        if ui.add_enabled(undo_enabled, egui::Button::new("↶").min_size(Vec2::new(24.0, 24.0)))
                            .on_hover_text("Undo (Ctrl+Z)")
                            .clicked()
                        {
                            self.annotations.pop();
                        }

                        ui.separator();

                        // Copy Button: In-memory copy & exit (< 2ms)
                        if ui.button(egui::RichText::new("📋 Copy").strong().color(Color32::from_rgb(16, 185, 129)))
                            .on_hover_text("Instant In-Memory Copy to Clipboard (Ctrl+C)")
                            .clicked()
                        {
                            self.action_copy(ctx, screen_rect);
                        }

                        // Save Button: Save to disk & exit
                        if ui.button(egui::RichText::new("💾 Save").strong().color(Color32::from_rgb(14, 165, 233)))
                            .on_hover_text("Save Image to Disk (Ctrl+S)")
                            .clicked()
                        {
                            self.action_save(ctx, screen_rect);
                        }

                        // Close Button: Cancel & exit
                        if ui.button(egui::RichText::new("✕").color(Color32::from_rgb(244, 63, 94)))
                            .on_hover_text("Cancel and Exit (Esc)")
                            .clicked()
                        {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    });
                });
        });
    }
}

/// Draws an annotation on screen.
fn draw_annotation(painter: &egui::Painter, ann: &Annotation) {
    match ann {
        Annotation::Rectangle { rect, color, thickness } => {
            painter.rect_stroke(*rect, 0.0_f32, Stroke::new(*thickness, *color));
        }
        Annotation::Arrow { start, end, color, thickness } => {
            draw_arrow(painter, *start, *end, *color, *thickness);
        }
        Annotation::Pen { points, color, thickness } => {
            for window in points.windows(2) {
                painter.line_segment([window[0], window[1]], Stroke::new(*thickness, *color));
            }
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
        let head_size = (thickness * 4.0).clamp(10.0, 20.0);
        let perp = Vec2::new(-norm.y, norm.x) * (head_size * 0.45);
        let arrow_left = end - norm * head_size + perp;
        let arrow_right = end - norm * head_size - perp;

        painter.line_segment([end, arrow_left], Stroke::new(thickness, color));
        painter.line_segment([end, arrow_right], Stroke::new(thickness, color));
    }
}

/// Normalizes a rectangle ensuring min <= max.
pub fn normalize_rect(r: Rect) -> Rect {
    let min_x = r.min.x.min(r.max.x);
    let max_x = r.min.x.max(r.max.x);
    let min_y = r.min.y.min(r.max.y);
    let max_y = r.min.y.max(r.max.y);
    Rect::from_min_max(Pos2::new(min_x, min_y), Pos2::new(max_x, max_y))
}

/// Burns all annotations onto cropped image pixels directly in RAM.
pub fn burn_and_crop(
    orig: &RgbaImage,
    screen_rect: Rect,
    selection: Rect,
    annotations: &[Annotation],
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

    for ann in annotations {
        match ann {
            Annotation::Rectangle { rect, color, thickness } => {
                let box_min_x = (((rect.min.x - selection.min.x) * scale_x).round() as i32).max(0);
                let box_min_y = (((rect.min.y - selection.min.y) * scale_y).round() as i32).max(0);
                let box_max_x = (((rect.max.x - selection.min.x) * scale_x).round() as i32).min(width as i32 - 1);
                let box_max_y = (((rect.max.y - selection.min.y) * scale_y).round() as i32).min(height as i32 - 1);

                if box_max_x <= box_min_x || box_max_y <= box_min_y {
                    continue;
                }

                let t_val = (*thickness * scale_x).round().max(1.0) as i32;
                let rgba = Rgba([color.r(), color.g(), color.b(), 255]);

                // Draw top/bottom borders
                for t in 0..t_val {
                    let y_top = box_min_y + t;
                    let y_bot = box_max_y - t;
                    for x in box_min_x..=box_max_x {
                        if y_top >= 0 && y_top < height as i32 && x >= 0 && x < width as i32 {
                            cropped.put_pixel(x as u32, y_top as u32, rgba);
                        }
                        if y_bot >= 0 && y_bot < height as i32 && x >= 0 && x < width as i32 {
                            cropped.put_pixel(x as u32, y_bot as u32, rgba);
                        }
                    }
                }

                // Draw left/right borders
                for t in 0..t_val {
                    let x_left = box_min_x + t;
                    let x_right = box_max_x - t;
                    for y in box_min_y..=box_max_y {
                        if x_left >= 0 && x_left < width as i32 && y >= 0 && y < height as i32 {
                            cropped.put_pixel(x_left as u32, y as u32, rgba);
                        }
                        if x_right >= 0 && x_right < width as i32 && y >= 0 && y < height as i32 {
                            cropped.put_pixel(x_right as u32, y as u32, rgba);
                        }
                    }
                }
            }
            Annotation::Arrow { start, end, color, thickness } => {
                let sx = ((start.x - selection.min.x) * scale_x).round() as i32;
                let sy = ((start.y - selection.min.y) * scale_y).round() as i32;
                let ex = ((end.x - selection.min.x) * scale_x).round() as i32;
                let ey = ((end.y - selection.min.y) * scale_y).round() as i32;

                let t_val = (*thickness * scale_x).round().max(1.0) as i32;
                let rgba = Rgba([color.r(), color.g(), color.b(), 255]);

                rasterize_line(&mut cropped, sx, sy, ex, ey, t_val, rgba);

                // Arrowhead rasterization
                let dx = (ex - sx) as f32;
                let dy = (ey - sy) as f32;
                let len = (dx * dx + dy * dy).sqrt();
                if len > 6.0 {
                    let norm_x = dx / len;
                    let norm_y = dy / len;
                    let head_size = (t_val as f32 * 4.0).clamp(10.0, 24.0);
                    let perp_x = -norm_y * (head_size * 0.45);
                    let perp_y = norm_x * (head_size * 0.45);

                    let lx = (ex as f32 - norm_x * head_size + perp_x).round() as i32;
                    let ly = (ey as f32 - norm_y * head_size + perp_y).round() as i32;
                    let rx = (ex as f32 - norm_x * head_size - perp_x).round() as i32;
                    let ry = (ey as f32 - norm_y * head_size - perp_y).round() as i32;

                    rasterize_line(&mut cropped, ex, ey, lx, ly, t_val, rgba);
                    rasterize_line(&mut cropped, ex, ey, rx, ry, t_val, rgba);
                }
            }
            Annotation::Pen { points, color, thickness } => {
                let t_val = (*thickness * scale_x).round().max(1.0) as i32;
                let rgba = Rgba([color.r(), color.g(), color.b(), 255]);

                for window in points.windows(2) {
                    let p1 = window[0];
                    let p2 = window[1];
                    let x1 = ((p1.x - selection.min.x) * scale_x).round() as i32;
                    let y1 = ((p1.y - selection.min.y) * scale_y).round() as i32;
                    let x2 = ((p2.x - selection.min.x) * scale_x).round() as i32;
                    let y2 = ((p2.y - selection.min.y) * scale_y).round() as i32;

                    rasterize_line(&mut cropped, x1, y1, x2, y2, t_val, rgba);
                }
            }
        }
    }

    cropped
}

/// Robust Bresenham line rasterizer with configurable thickness.
fn rasterize_line(img: &mut RgbaImage, x0: i32, y0: i32, x1: i32, y1: i32, thickness: i32, color: Rgba<u8>) {
    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;

    let mut curr_x = x0;
    let mut curr_y = y0;
    let radius = (thickness / 2).max(0);

    loop {
        for oy in -radius..=radius {
            for ox in -radius..=radius {
                let px = curr_x + ox;
                let py = curr_y + oy;
                if px >= 0 && px < img.width() as i32 && py >= 0 && py < img.height() as i32 {
                    img.put_pixel(px as u32, py as u32, color);
                }
            }
        }

        if curr_x == x1 && curr_y == y1 {
            break;
        }

        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            curr_x += sx;
        }
        if e2 <= dx {
            err += dx;
            curr_y += sy;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

        let cropped = burn_and_crop(&orig, screen_rect, selection, &[box_annotation]);

        assert_eq!(cropped.width(), 100);
        assert_eq!(cropped.height(), 100);

        // Border pixel (x = 20 - 10 = 10, y = 20 - 10 = 10) must be red
        let border_pixel = cropped.get_pixel(10, 10);
        assert_eq!(*border_pixel, Rgba([255, 0, 0, 255]));

        // Outside pixel (x = 0, y = 0) must remain black
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

        let cropped = burn_and_crop(&orig, screen_rect, selection, &[arrow]);

        // Point along line (x = 30, y = 10) must be green
        let line_pixel = cropped.get_pixel(30, 10);
        assert_eq!(*line_pixel, Rgba([0, 255, 0, 255]));
    }
}
