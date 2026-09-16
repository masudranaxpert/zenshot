use crate::clipboard::copy_to_clipboard;
use crate::config::Config;
use crate::icons::ToolbarIcons;
use chrono::Local;
use eframe::egui::{
    self, Color32, CursorIcon, ImageButton, Key, Pos2, Rect, Stroke, Vec2,
};
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
}

impl ZenShotApp {
    pub fn new(config: Config, screen_image: RgbaImage) -> Self {
        Self {
            config,
            screen_image,
            texture: None,
            icons: None,
            selection: None,
            drag_state: DragState::None,
            current_tool: Tool::Select,
            color_index: 0,
            annotations: Vec::new(),
            text_input: String::new(),
            active_text_pos: None,
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
        const HANDLE_RADIUS: f32 = 8.0;
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

    /// Calculates exact screen bounds of the Horizontal and Vertical toolbars.
    fn get_toolbar_rects(&self, sel: Rect, screen_rect: Rect) -> (Rect, Rect) {
        let btn_size = 28.0;

        // Horizontal toolbar: 4 core actions (Print, Copy, Save, Close) - 100% local, zero upload
        let h_width = 4.0 * btn_size + 8.0;
        let h_height = btn_size + 6.0;

        let mut h_x = sel.right() - h_width;
        if h_x < screen_rect.left() + 4.0 {
            h_x = screen_rect.left() + 4.0;
        }

        let mut h_y = sel.bottom() + 6.0;
        if h_y + h_height > screen_rect.bottom() - 4.0 {
            h_y = sel.bottom() - h_height - 6.0;
        }

        let h_rect = Rect::from_min_size(Pos2::new(h_x, h_y), Vec2::new(h_width, h_height));

        // Vertical toolbar: 8 buttons (Pen, Line, Arrow, Rect, Marker, Text, Color, Undo)
        let v_width = btn_size + 6.0;
        let v_height = 8.0 * btn_size + 8.0;

        let mut v_x = sel.right() + 6.0;
        if v_x + v_width > screen_rect.right() - 4.0 {
            v_x = sel.right() - v_width - 6.0;
        }

        let mut v_y = sel.bottom() - v_height;
        if v_y < screen_rect.top() + 4.0 {
            v_y = screen_rect.top() + 4.0;
        }

        let v_rect = Rect::from_min_size(Pos2::new(v_x, v_y), Vec2::new(v_width, v_height));

        (h_rect, v_rect)
    }
}

impl eframe::App for ZenShotApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 1. Initialize desktop texture and icons once
        if self.texture.is_none() {
            let size = [self.screen_image.width() as usize, self.screen_image.height() as usize];
            let pixels = self.screen_image.as_flat_samples();
            let color_image = egui::ColorImage::from_rgba_unmultiplied(size, pixels.as_slice());
            self.texture = Some(ctx.load_texture("desktop", color_image, egui::TextureOptions::LINEAR));
        }
        if self.icons.is_none() {
            self.icons = Some(ToolbarIcons::load(ctx));
        }

        let screen_rect = ctx.screen_rect();

        // 2. Global Hotkeys
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

        // 3. Pointer and toolbar hover detection (CRITICAL: prevents toolbar clicks from resetting selection)
        let pointer = ctx.input(|i| i.pointer.clone());
        let current_pos = pointer.hover_pos().unwrap_or(Pos2::ZERO);

        let mut mouse_on_toolbar = false;
        if let Some(sel) = self.selection {
            let (h_bar, v_bar) = self.get_toolbar_rects(sel, screen_rect);
            if h_bar.contains(current_pos) || v_bar.contains(current_pos) {
                mouse_on_toolbar = true;
            }
        }

        let mut desired_cursor = CursorIcon::Crosshair;
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
                    // Darkened desktop background
                    painter.image(
                        tex.id(),
                        screen_rect,
                        Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                        Color32::from_rgba_unmultiplied(80, 80, 85, 255),
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

                        // Selection border: 1.5px light blue
                        painter.rect_stroke(sel, 0.0_f32, Stroke::new(1.5_f32, Color32::from_rgb(0, 174, 239)));

                        // 8 Sizing Handles
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
                            let handle_rect = Rect::from_center_size(pos, Vec2::splat(6.0));
                            painter.rect_filled(handle_rect, 0.0_f32, Color32::WHITE);
                            painter.rect_stroke(handle_rect, 0.0_f32, Stroke::new(1.0_f32, Color32::from_rgb(0, 120, 215)));
                        }

                        // Dimension Badge: W x H
                        let dim_text = format!("{} x {}", sel.width().round() as i32, sel.height().round() as i32);
                        let badge_pos = Pos2::new(sel.left(), (sel.top() - 22.0).max(4.0));
                        let font_id = egui::FontId::monospace(11.0);
                        let galley = painter.layout_no_wrap(dim_text, font_id, Color32::WHITE);
                        let badge_rect = Rect::from_min_size(badge_pos, galley.size() + Vec2::new(8.0, 4.0));
                        painter.rect_filled(badge_rect, 2.0_f32, Color32::from_black_alpha(220));
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

                // Render Lightshot Dual Toolbars when selection is active
                if let Some(sel) = self.selection {
                    if matches!(self.drag_state, DragState::None) {
                        self.render_lightshot_toolbars(ui, ctx, sel, screen_rect);
                    }
                }
            });
    }
}

impl ZenShotApp {
    /// Renders Lightshot's iconic Dual Floating Toolbars (Horizontal & Vertical) with extracted icons.
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
        let h_builder = egui::UiBuilder::new().max_rect(h_rect);
        ui.allocate_new_ui(h_builder, |ui| {
            egui::Frame::popup(ui.style())
                .fill(Color32::from_rgb(238, 238, 242)) // Lightshot signature toolbar silver/grey
                .stroke(Stroke::new(1.0_f32, Color32::from_rgb(180, 180, 190)))
                .rounding(3.0)
                .inner_margin(2.0)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing = Vec2::new(2.0, 0.0);

                        if let Some(icons) = &self.icons {
                            if ui.add(ImageButton::new(&icons.print)).on_hover_text("Print (Ctrl+P)").clicked() {
                                action = ToolbarAction::Print;
                            }
                            if ui.add(ImageButton::new(&icons.copy)).on_hover_text("Copy to Clipboard (Ctrl+C)").clicked() {
                                action = ToolbarAction::Copy;
                            }
                            if ui.add(ImageButton::new(&icons.save)).on_hover_text("Save to disk (Ctrl+S)").clicked() {
                                action = ToolbarAction::Save;
                            }
                            if ui.add(ImageButton::new(&icons.close)).on_hover_text("Cancel (Esc)").clicked() {
                                action = ToolbarAction::Close;
                            }
                        }
                    });
                });
        });

        // --- 2. VERTICAL DRAWING TOOLBAR (Right) ---
        let v_builder = egui::UiBuilder::new().max_rect(v_rect);
        ui.allocate_new_ui(v_builder, |ui| {
            egui::Frame::popup(ui.style())
                .fill(Color32::from_rgb(238, 238, 242)) // Lightshot signature toolbar silver/grey
                .stroke(Stroke::new(1.0_f32, Color32::from_rgb(180, 180, 190)))
                .rounding(3.0)
                .inner_margin(2.0)
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing = Vec2::new(0.0, 2.0);

                        if let Some(icons) = &self.icons {
                            let btn = ImageButton::new(&icons.pen).selected(current_tool == Tool::Pen);
                            if ui.add(btn).on_hover_text("Pen Tool").clicked() {
                                action = ToolbarAction::SelectTool(if current_tool == Tool::Pen { Tool::Select } else { Tool::Pen });
                            }

                            let btn = ImageButton::new(&icons.line).selected(current_tool == Tool::Line);
                            if ui.add(btn).on_hover_text("Line Tool").clicked() {
                                action = ToolbarAction::SelectTool(if current_tool == Tool::Line { Tool::Select } else { Tool::Line });
                            }

                            let btn = ImageButton::new(&icons.arrow).selected(current_tool == Tool::Arrow);
                            if ui.add(btn).on_hover_text("Arrow Tool").clicked() {
                                action = ToolbarAction::SelectTool(if current_tool == Tool::Arrow { Tool::Select } else { Tool::Arrow });
                            }

                            let btn = ImageButton::new(&icons.rect).selected(current_tool == Tool::Rectangle);
                            if ui.add(btn).on_hover_text("Rectangle Tool").clicked() {
                                action = ToolbarAction::SelectTool(if current_tool == Tool::Rectangle { Tool::Select } else { Tool::Rectangle });
                            }

                            let btn = ImageButton::new(&icons.marker).selected(current_tool == Tool::Marker);
                            if ui.add(btn).on_hover_text("Marker / Highlighter Tool").clicked() {
                                action = ToolbarAction::SelectTool(if current_tool == Tool::Marker { Tool::Select } else { Tool::Marker });
                            }

                            let btn = ImageButton::new(&icons.text).selected(current_tool == Tool::Text);
                            if ui.add(btn).on_hover_text("Text Tool").clicked() {
                                action = ToolbarAction::SelectTool(if current_tool == Tool::Text { Tool::Select } else { Tool::Text });
                            }

                            let (rect, resp) = ui.allocate_exact_size(Vec2::splat(22.0), egui::Sense::click());
                            ui.painter().rect_filled(rect, 2.0_f32, current_color);
                            ui.painter().rect_stroke(rect, 2.0_f32, Stroke::new(1.0_f32, Color32::from_rgb(120, 120, 130)));
                            if resp.on_hover_text("Click to cycle color").clicked() {
                                action = ToolbarAction::CycleColor;
                            }

                            let undo_btn = ImageButton::new(&icons.undo);
                            if ui.add(undo_btn).on_hover_text("Undo (Ctrl+Z)").clicked() {
                                action = ToolbarAction::Undo;
                            }
                        }
                    });
                });
        });

        // --- 3. APPLY ACTIONS SAFELY AFTER UI RENDERING ---
        match action {
            ToolbarAction::None => {}
            ToolbarAction::Copy => self.action_copy(ctx, screen_rect),
            ToolbarAction::Save => self.action_save(ctx, screen_rect),
            ToolbarAction::Close => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
            ToolbarAction::Print => self.action_save(ctx, screen_rect),
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

                let dx = (ex - sx) as f32;
                let dy = (ey - sy) as f32;
                let len = (dx * dx + dy * dy).sqrt();
                if len > 6.0 {
                    let norm_x = dx / len;
                    let norm_y = dy / len;
                    let head_size = (t_val as f32 * 4.0).clamp(10.0, 22.0);
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
            Annotation::Line { start, end, color, thickness } => {
                let sx = ((start.x - selection.min.x) * scale_x).round() as i32;
                let sy = ((start.y - selection.min.y) * scale_y).round() as i32;
                let ex = ((end.x - selection.min.x) * scale_x).round() as i32;
                let ey = ((end.y - selection.min.y) * scale_y).round() as i32;
                let t_val = (*thickness * scale_x).round().max(1.0) as i32;
                let rgba = Rgba([color.r(), color.g(), color.b(), 255]);
                rasterize_line(&mut cropped, sx, sy, ex, ey, t_val, rgba);
            }
            Annotation::Pen { points, color, thickness } | Annotation::Marker { points, color, thickness } => {
                let t_val = (*thickness * scale_x).round().max(1.0) as i32;
                let rgba = Rgba([color.r(), color.g(), color.b(), color.a()]);

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
            Annotation::Text { .. } => {}
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
                    if color.0[3] < 255 {
                        let existing = img.get_pixel(px as u32, py as u32);
                        let a_f = color.0[3] as f32 / 255.0;
                        let inv_a = 1.0 - a_f;
                        let r = (color.0[0] as f32 * a_f + existing.0[0] as f32 * inv_a) as u8;
                        let g = (color.0[1] as f32 * a_f + existing.0[1] as f32 * inv_a) as u8;
                        let b = (color.0[2] as f32 * a_f + existing.0[2] as f32 * inv_a) as u8;
                        img.put_pixel(px as u32, py as u32, Rgba([r, g, b, 255]));
                    } else {
                        img.put_pixel(px as u32, py as u32, color);
                    }
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

        let cropped = burn_and_crop(&orig, screen_rect, selection, &[arrow]);
        let line_pixel = cropped.get_pixel(30, 10);
        assert_eq!(*line_pixel, Rgba([0, 255, 0, 255]));
    }
}
