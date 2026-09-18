use crate::config::{Config, OutputFormat};
use crate::hotkey::{self, Hotkey};
use chrono::Local;
use eframe::egui::{
    self, Color32, FontId, Frame, Key, Pos2, Rect, RichText, Rounding, Sense, Stroke, Vec2,
    ViewportBuilder,
};

/// The Options surface inherits the capture overlay's committed world:
/// Lightshot's glossy light chrome with one cyan-blue accent. Every colour
/// here traces back to something measured on that chrome.
const ACCENT: Color32 = Color32::from_rgb(14, 108, 185); // sampled from the active icon glyphs
const ACCENT_SOFT: Color32 = Color32::from_rgba_premultiplied(2, 12, 20, 28); // accent @ 11%
const GROUND: Color32 = Color32::from_rgb(245, 247, 248); // toolbar BODY_TOP
const CARD: Color32 = Color32::from_rgb(253, 254, 254);
const HAIRLINE: Color32 = Color32::from_rgba_premultiplied(72, 77, 82, 110); // border @ 43%
const INK: Color32 = Color32::from_rgb(45, 52, 60);
const INK_MUTED: Color32 = Color32::from_rgb(108, 117, 128);
const DANGER: Color32 = Color32::from_rgb(176, 58, 46);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Tab {
    General,
    Hotkeys,
    Formats,
}

const TABS: [(Tab, &str); 3] = [
    (Tab::General, "General"),
    (Tab::Hotkeys, "Hotkeys"),
    (Tab::Formats, "Formats"),
];

pub struct OptionsApp {
    config: Config,
    tab: Tab,
    listening: Option<ListenTarget>,
    status: Option<String>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ListenTarget {
    Capture,
    SaveFullscreen,
}

impl OptionsApp {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            tab: Tab::General,
            listening: None,
            status: None,
        }
    }
}

pub fn run() -> eframe::Result<()> {
    let config = Config::load_or_default();
    let native_options = eframe::NativeOptions {
        persist_window: false,
        viewport: ViewportBuilder::default()
            .with_title("ZenShot Options")
            .with_app_id("ZenShot")
            .with_icon(crate::icons::window_icon())
            .with_inner_size([560.0, 440.0])
            .with_min_inner_size([520.0, 400.0])
            .with_resizable(true),
        ..Default::default()
    };
    eframe::run_native(
        "Options",
        native_options,
        Box::new(|cc| {
            apply_theme(&cc.egui_ctx);
            Ok(Box::new(OptionsApp::new(config)))
        }),
    )
}

/// One visual system, applied once: light glossy chrome, hairline borders,
/// accent carried only by the active element.
fn apply_theme(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    let v = &mut style.visuals;
    v.override_text_color = Some(INK);
    v.window_fill = GROUND;
    v.panel_fill = GROUND;
    v.extreme_bg_color = Color32::WHITE; // text-field interior
    v.selection.bg_fill = ACCENT_SOFT;
    v.selection.stroke = Stroke::new(1.0_f32, ACCENT);
    v.slider_trailing_fill = true;

    for state in [
        &mut v.widgets.inactive,
        &mut v.widgets.hovered,
        &mut v.widgets.active,
    ] {
        state.rounding = Rounding::same(4.0);
        state.fg_stroke = Stroke::new(1.6_f32, ACCENT); // checkbox tick / slider handle ring
    }
    v.widgets.inactive.bg_fill = Color32::WHITE;
    v.widgets.inactive.weak_bg_fill = Color32::WHITE;
    v.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, HAIRLINE);
    v.widgets.hovered.bg_fill = Color32::from_rgb(240, 244, 247);
    v.widgets.hovered.weak_bg_fill = Color32::from_rgb(240, 244, 247);
    v.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(120, 138, 155));
    v.widgets.active.bg_fill = ACCENT_SOFT;
    v.widgets.active.weak_bg_fill = ACCENT_SOFT;
    v.widgets.active.bg_stroke = Stroke::new(1.0_f32, ACCENT);
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, HAIRLINE); // separators stay hairline

    style.spacing.icon_width = 16.0;
    style.spacing.icon_spacing = 8.0;
    style.spacing.button_padding = Vec2::new(12.0, 4.0);
    style.text_styles.insert(
        egui::TextStyle::Body,
        egui::FontId::proportional(13.0),
    );
    style.text_styles.insert(
        egui::TextStyle::Button,
        egui::FontId::proportional(13.0),
    );
    ctx.set_style(style);
}

impl eframe::App for OptionsApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Some(target) = self.listening {
            // the armed keycap breathes — keep the animation ticking
            ctx.request_repaint_after(std::time::Duration::from_millis(50));
            if ctx.input(|i| i.key_pressed(Key::Escape)) {
                self.listening = None;
            } else if let Some(hotkey) = read_binding(ctx) {
                match target {
                    ListenTarget::Capture => self.config.hotkey_capture = hotkey,
                    ListenTarget::SaveFullscreen => self.config.hotkey_save_fullscreen = hotkey,
                }
                self.listening = None;
            }
        } else {
            // Dialog keys yield to hotkey capture; Enter applies only when no
            // text field holds focus, matching Windows dialog conventions.
            if ctx.input(|i| i.key_pressed(Key::Escape)) {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            } else if !ctx.memory(|m| m.focused().is_some())
                && ctx.input(|i| i.key_pressed(Key::Enter))
            {
                self.apply(ctx);
            }
        }


        // Viewport-level panels, not nested ones: the tab strip, page and
        // footer each span the full window so they cannot drift apart.
        self.ui_header(ctx);
        self.ui_footer(ctx);
        egui::CentralPanel::default()
            .frame(
                Frame::none()
                    .fill(GROUND)
                    .inner_margin(egui::Margin::symmetric(16.0, 12.0)),
            )
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| match self.tab {
                        Tab::General => self.ui_general(ui),
                        Tab::Hotkeys => self.ui_hotkeys(ui),
                        Tab::Formats => self.ui_formats(ui),
                    });
            });
    }

    /// Unpainted seams between panels take the ground colour instead of the
    /// renderer's black clear — the surface reads as one continuous sheet.
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.961, 0.969, 0.973, 1.0] // GROUND 245,247,248
    }
}

/// Lightshot's header gloss, drawn as two interpolated colour bands — the
/// same stops as the capture overlay's toolbar chrome, but resolution
/// independent so display scaling can never band it into stripes.
fn paint_glossy_header(painter: &egui::Painter, rect: Rect) {
    let top = Color32::from_rgb(250, 251, 251);
    let knee = Color32::from_rgb(232, 236, 239);
    let bottom = Color32::from_rgb(211, 214, 217);
    let knee_y = rect.top() + rect.height() * 0.087;
    let shadow_top = rect.bottom() - 1.0;

    let mut mesh = egui::Mesh::default();
    for (y0, y1, c0, c1) in [
        (rect.top(), knee_y, top, knee),
        (knee_y, shadow_top, knee, bottom),
    ] {
        let base = mesh.vertices.len() as u32;
        for (pos, color) in [
            (Pos2::new(rect.left(), y0), c0),
            (Pos2::new(rect.right(), y0), c0),
            (Pos2::new(rect.left(), y1), c1),
            (Pos2::new(rect.right(), y1), c1),
        ] {
            mesh.colored_vertex(pos, color);
        }
        mesh.add_triangle(base, base + 1, base + 2);
        mesh.add_triangle(base + 2, base + 1, base + 3);
    }
    painter.add(egui::Shape::mesh(mesh));

    // the chrome's crisp edges: hairline frame on top, two-row drop shadow
    painter.hline(
        rect.left()..=rect.right(),
        rect.top(),
        Stroke::new(1.0_f32, Color32::from_black_alpha(19)),
    );
    // one quiet seam under the strip; heavier stacking reads as a black band
    painter.hline(
        rect.left()..=rect.right(),
        rect.bottom() - 0.5,
        Stroke::new(1.0_f32, Color32::from_black_alpha(45)),
    );
}

impl OptionsApp {
    /// Full-width window chrome. The strip is a TopBottomPanel on the
    /// viewport, so it always matches the title-bar width.
    fn ui_header(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("options_header")
            .frame(Frame::none().fill(GROUND))
            .exact_height(40.0)
            .show_separator_line(false)
            .show(ctx, |ui| {
                let rect = ui.max_rect();
                paint_glossy_header(ui.painter(), rect);
                ui.allocate_ui_with_layout(
                    rect.size(),
                    egui::Layout::left_to_right(egui::Align::Center),
                    |ui| {
                        ui.add_space(18.0);
                        for (tab, name) in TABS {
                            if cfg!(not(windows)) && tab == Tab::Hotkeys {
                                continue;
                            }
                            self.tab_button(ui, tab, name);
                            ui.add_space(4.0);
                        }
                    },
                );
            });
    }

    fn tab_button(&mut self, ui: &mut egui::Ui, tab: Tab, name: &str) {
        let active = self.tab == tab;
        let font = FontId::proportional(13.5);
        let measured = ui
            .painter()
            .layout_no_wrap(name.to_owned(), font.clone(), Color32::WHITE);
        let size = measured.size();
        let (rect, resp) = ui.allocate_exact_size(Vec2::new(size.x + 20.0, 40.0), Sense::click());
        let text_color = if active {
            Color32::from_rgb(10, 66, 115)
        } else if resp.hovered() {
            INK
        } else {
            Color32::from_rgb(96, 106, 117)
        };
        let galley = ui.painter().layout_no_wrap(name.to_owned(), font, text_color);
        let text_pos = Pos2::new(
            rect.center().x - size.x / 2.0,
            rect.center().y - size.y / 2.0 - 1.0,
        );
        ui.painter().galley(text_pos, galley, text_color);

        if active {
            ui.painter().hline(
                rect.left() + 6.0..=rect.right() - 6.0,
                rect.bottom() - 2.0,
                Stroke::new(2.0_f32, ACCENT),
            );
        }
        if resp.clicked() {
            self.tab = tab;
        }
    }

    fn ui_footer(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("options_footer")
            .frame(
                Frame::none()
                    .fill(GROUND)
                    .inner_margin(egui::Margin::symmetric(16.0, 10.0)),
            )
            .show_separator_line(false)
            .show(ctx, |ui| {
                let rect = ui.max_rect();
                ui.painter().hline(
                    rect.left()..=rect.right(),
                    rect.top(),
                    Stroke::new(1.0_f32, HAIRLINE),
                );
                ui.horizontal(|ui| {
                    if let Some(status) = &self.status {
                        ui.label(RichText::new(status).size(12.0).color(DANGER));
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add(
                                egui::Button::new(RichText::new("OK").color(Color32::WHITE).strong())
                                    .fill(ACCENT)
                                    .min_size(Vec2::new(84.0, 26.0)),
                            )
                            .clicked()
                        {
                            self.apply(ctx);
                        }
                        ui.add_space(8.0);
                        if ui
                            .add(
                                egui::Button::new(RichText::new("Cancel").color(INK))
                                    .fill(Color32::WHITE)
                                    .stroke(Stroke::new(1.0_f32, HAIRLINE))
                                    .min_size(Vec2::new(84.0, 26.0)),
                            )
                            .clicked()
                        {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    });
                });
            });
    }
}

/// White content card with the chrome's hairline border — the one elevation
/// this surface declares.
fn card(ui: &mut egui::Ui, title: &str, add: impl FnOnce(&mut egui::Ui)) {
    ui.add_space(2.0);
    Frame::none()
        .fill(CARD)
        .stroke(Stroke::new(1.0_f32, HAIRLINE))
        .rounding(Rounding::same(6.0))
        .inner_margin(egui::Margin::symmetric(14.0, 11.0))
        .outer_margin(egui::Margin::symmetric(0.0, 5.0))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            if !title.is_empty() {
                ui.label(RichText::new(title).size(12.5).color(INK_MUTED));
                ui.add_space(7.0);
            }
            add(ui);
        });
}

fn field_label(ui: &mut egui::Ui, text: &str) {
    ui.label(RichText::new(text).size(12.0).color(INK_MUTED));
    ui.add_space(3.0);
}

impl OptionsApp {
    fn ui_general(&mut self, ui: &mut egui::Ui) {
        card(ui, "Behaviour", |ui| {
            #[cfg(windows)]
            {
                ui.checkbox(&mut self.config.autostart, "Start ZenShot with Windows");
            }
            ui.checkbox(
                &mut self.config.show_notifications,
                "Show notifications about copying and saving",
            );
            ui.checkbox(
                &mut self.config.capture_cursor,
                "Capture a cursor on a screenshot",
            );
            ui.checkbox(
                &mut self.config.keep_selection,
                "Keep the selected area position",
            );
        });

        card(ui, "Saving", |ui| {
            field_label(ui, "Save directory");
            ui.add(
                egui::TextEdit::singleline(&mut self.config.save_dir)
                    .desired_width(ui.available_width() - 2.0)
                    .font(egui::TextStyle::Monospace),
            );
            ui.add_space(9.0);
            field_label(ui, "Filename pattern (strftime)");
            ui.add(
                egui::TextEdit::singleline(&mut self.config.filename_format)
                    .desired_width(ui.available_width() - 2.0)
                    .font(egui::TextStyle::Monospace),
            );
            ui.add_space(5.0);
            filename_preview(ui, &self.config.filename_format);
        });

        #[cfg(not(windows))]
        card(ui, "Linux", |ui| {
            ui.label(
                RichText::new(
                    "Linux does not need a tray process. Bind Ctrl+Shift+S in your desktop \
                     settings to the command `zenshot`.",
                )
                .size(12.5)
                .color(INK_MUTED),
            );
        });

        ui.add_space(6.0);
    }

    fn ui_hotkeys(&mut self, ui: &mut egui::Ui) {
        ui.add_space(4.0);
        ui.label(
            RichText::new("Click a shortcut, then press the new key combination. Esc cancels.")
                .size(12.0)
                .color(INK_MUTED),
        );
        ui.add_space(2.0);

        card(ui, "Shortcuts", |ui| {
            hotkey_row(
                ui,
                "General hotkey",
                &mut self.config.hotkey_capture_enabled,
                &self.config.hotkey_capture,
                self.listening,
                ListenTarget::Capture,
                &mut self.listening,
            );
            ui.add_space(8.0);
            hotkey_row(
                ui,
                "Instant save of the fullscreen",
                &mut self.config.hotkey_save_fullscreen_enabled,
                &self.config.hotkey_save_fullscreen,
                self.listening,
                ListenTarget::SaveFullscreen,
                &mut self.listening,
            );
        });

        ui.add_space(6.0);
        if ui
            .add(
                egui::Button::new(RichText::new("Reset to defaults").color(INK))
                    .fill(Color32::WHITE)
                    .stroke(Stroke::new(1.0_f32, HAIRLINE)),
            )
            .clicked()
        {
            self.config.hotkey_capture = Hotkey::ctrl_shift_s();
            self.config.hotkey_capture_enabled = true;
            self.config.hotkey_save_fullscreen = Hotkey::ctrl_alt_s();
            self.config.hotkey_save_fullscreen_enabled = false;
            self.listening = None;
        }
    }

    fn ui_formats(&mut self, ui: &mut egui::Ui) {
        card(ui, "Image Format", |ui| {
            field_label(ui, "Default export format");
            ui.add_space(2.0);
            segmented(
                ui,
                &[
                    (OutputFormat::Png, "PNG"),
                    (OutputFormat::Jpeg, "JPEG"),
                ],
                &mut self.config.output_format,
            );
            ui.add_space(6.0);
            let format_desc = match self.config.output_format {
                OutputFormat::Png => "Lossless quality. Best for text, code, and graphics.",
                OutputFormat::Jpeg => "Compressed file size with adjustable quality. Best for photos.",
            };
            ui.label(RichText::new(format_desc).size(11.5).color(INK_MUTED));

            ui.add_space(14.0);
            let is_jpeg = self.config.output_format == OutputFormat::Jpeg;
            ui.add_enabled_ui(is_jpeg, |ui| {
                let q = self.config.jpeg_quality;
                let hint = quality_hint(q);
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("JPEG Quality")
                            .size(12.0)
                            .color(if is_jpeg { INK_MUTED } else { Color32::from_rgb(180, 186, 192) }),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(RichText::new(hint).size(11.5).color(INK_MUTED));
                    });
                });
                ui.add_space(4.0);

                ui.scope(|ui| {
                    let v = ui.visuals_mut();
                    v.slider_trailing_fill = true;
                    v.selection.bg_fill = ACCENT;
                    v.widgets.inactive.bg_fill = Color32::from_rgb(222, 226, 230);
                    v.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(210, 215, 220));
                    v.widgets.hovered.bg_fill = Color32::from_rgb(212, 217, 223);

                    let available = ui.available_width();
                    let input_width = 56.0;
                    let slider_width = (available - input_width - 12.0).max(100.0);

                    ui.horizontal(|ui| {
                        ui.spacing_mut().slider_width = slider_width;
                        ui.spacing_mut().slider_rail_height = 6.0;

                        ui.add(
                            egui::Slider::new(&mut self.config.jpeg_quality, 10..=100)
                                .show_value(false)
                                .trailing_fill(true),
                        );

                        let mut q_int = self.config.jpeg_quality as i32;
                        if ui
                            .add_sized(
                                [input_width, 22.0],
                                egui::DragValue::new(&mut q_int)
                                    .range(10..=100)
                                    .speed(1.0)
                                    .suffix("%"),
                            )
                            .changed()
                        {
                            self.config.jpeg_quality = q_int as u8;
                        }
                    });
                });
            });
        });
        ui.add_space(6.0);
    }

    fn apply(&mut self, ctx: &egui::Context) {
        self.config.jpeg_quality = self.config.jpeg_quality_clamped();
        if let Err(err) = self.config.save() {
            self.status = Some(err);
            return;
        }
        if let Err(err) = crate::autostart::set_enabled(self.config.autostart) {
            self.status = Some(err);
            return;
        }
        crate::notify::reload_daemon();
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }
}

/// Two-choice segment — faster than a combo box for exactly two formats.
fn segmented<T: PartialEq + Copy>(ui: &mut egui::Ui, choices: &[(T, &str)], selected: &mut T) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        let count = choices.len();
        for (i, (value, name)) in choices.iter().enumerate() {
            let active = *value == *selected;
            let mut text = RichText::new(*name).size(12.5);
            let btn = if active {
                egui::Button::new(text.strong().color(Color32::WHITE))
                    .fill(ACCENT)
                    .min_size(Vec2::new(88.0, 24.0))
            } else {
                text = text.color(INK);
                egui::Button::new(text)
                    .fill(Color32::WHITE)
                    .stroke(Stroke::new(1.0_f32, HAIRLINE))
                    .min_size(Vec2::new(88.0, 24.0))
            };
            let resp = ui.add(btn);
            if i < count - 1 {
                ui.add_space(-1.0); // shared edge between segments
            }
            if resp.clicked() {
                *selected = *value;
            }
        }
    });
}

/// Shortcut rendered as a keycap. Clicking arms capture; the armed chip
/// breathes in the accent so the state is unmistakable.
fn hotkey_row(
    ui: &mut egui::Ui,
    label: &str,
    enabled: &mut bool,
    hotkey: &Hotkey,
    listening: Option<ListenTarget>,
    target: ListenTarget,
    listening_slot: &mut Option<ListenTarget>,
) {
    ui.horizontal(|ui| {
        ui.checkbox(
            enabled,
            RichText::new(label).color(if *enabled { INK } else { INK_MUTED }),
        );
        // Anchor the keycap to the card's right edge by measuring it first —
        // a right-to-left sub-layout overflows the row it lives in.
        let armed = matches_target(listening, target);
        let caption = if armed {
            "Press a shortcut...".to_owned()
        } else {
            hotkey.display()
        };
        let measure = ui.painter().layout_no_wrap(
            caption.clone(),
            FontId::monospace(12.0),
            Color32::WHITE,
        );
        let chip_width = measure.size().x + 18.0;
        let gap = ui.available_width() - chip_width;
        if gap > 8.0 {
            ui.add_space(gap);
        }
        let chip = keycap(ui, &caption, armed && *enabled, *enabled);
        if chip.clicked() && *enabled {
            *listening_slot = Some(target);
        }
    });
}

fn keycap(ui: &mut egui::Ui, caption: &str, armed: bool, enabled: bool) -> egui::Response {
    let text_color = if armed {
        ACCENT
    } else if enabled {
        INK
    } else {
        INK_MUTED
    };
    let font = FontId::monospace(12.0);
    let galley = ui
        .painter()
        .layout_no_wrap(caption.to_owned(), font, text_color);
    let pad = Vec2::new(18.0, 10.0);
    let (rect, resp) = ui.allocate_exact_size(galley.size() + pad, Sense::click());
    let border = if armed {
        Stroke::new(1.4_f32, ACCENT)
    } else {
        Stroke::new(1.0_f32, HAIRLINE)
    };
    let fill = if armed {
        ACCENT_SOFT
    } else {
        Color32::WHITE
    };
    ui.painter().rect(rect.expand(2.0), 4.0, fill, border);
    if armed {
        // breathe: the armed chip glows and settles each second, so the
        // "listening" state is unmistakable
        let t = (ui.input(|i| i.time) as f32).rem_euclid(1.2) / 1.2;
        let glow = 12.0 + 18.0 * (t * std::f32::consts::TAU).sin().abs();
        ui.painter().rect(
            rect.expand(2.0),
            4.0,
            Color32::from_rgba_unmultiplied(14, 108, 185, glow as u8),
            Stroke::NONE,
        );
    }
    let pos = Pos2::new(
        rect.center().x - galley.size().x / 2.0,
        rect.center().y - galley.size().y / 2.0,
    );
    ui.painter().galley(pos, galley, text_color);
    resp.on_hover_text(if enabled {
        "Click to change"
    } else {
        "Enable this shortcut first"
    })
}

/// Live strftime preview beneath the pattern field, computed from now. An
/// unsupported specifier is called out instead of previewing garbage.
fn filename_preview(ui: &mut egui::Ui, pattern: &str) {
    if let Some(bad) = unsupported_strftime(pattern) {
        ui.label(
            RichText::new(format!("Unsupported specifier %{bad}"))
                .size(11.5)
                .color(DANGER),
        );
        return;
    }
    let rendered = Local::now().format(pattern).to_string();
    ui.horizontal(|ui| {
        ui.label(RichText::new("Preview").size(11.5).color(INK_MUTED));
        ui.label(
            RichText::new(rendered)
                .font(FontId::monospace(11.5))
                .color(INK_MUTED),
        );
    });
}

fn unsupported_strftime(pattern: &str) -> Option<char> {
    const SUPPORTED: &str = "aAbBcCdDeFgGhHIjklmMprRSuUTvVwWxXyYzZ%nfTF";
    let mut chars = pattern.chars();
    while let Some(c) = chars.next() {
        if c == '%' {
            match chars.next() {
                Some(n) if SUPPORTED.contains(n) => {}
                Some(n) => return Some(n),
                None => return Some(' '),
            }
        }
    }
    None
}

fn matches_target(listening: Option<ListenTarget>, target: ListenTarget) -> bool {
    matches!(listening, Some(t) if std::mem::discriminant(&t) == std::mem::discriminant(&target))
}

fn read_binding(ctx: &egui::Context) -> Option<Hotkey> {
    ctx.input(|i| {
        let key = i.keys_down.iter().copied().next()?;
        if i.key_pressed(key) {
            hotkey::from_egui(key, i.modifiers)
        } else {
            None
        }
    })
}

pub(crate) fn quality_hint(q: u8) -> &'static str {
    match q {
        90..=100 => "Maximum quality (larger file)",
        75..=89 => "High quality (recommended)",
        50..=74 => "Balanced compression",
        _ => "Smaller file size (lower quality)",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quality_hint_maps_ranges_appropriately() {
        assert_eq!(quality_hint(100), "Maximum quality (larger file)");
        assert_eq!(quality_hint(90), "Maximum quality (larger file)");
        assert_eq!(quality_hint(89), "High quality (recommended)");
        assert_eq!(quality_hint(75), "High quality (recommended)");
        assert_eq!(quality_hint(74), "Balanced compression");
        assert_eq!(quality_hint(50), "Balanced compression");
        assert_eq!(quality_hint(49), "Smaller file size (lower quality)");
        assert_eq!(quality_hint(10), "Smaller file size (lower quality)");
    }

    #[test]
    fn strftime_validator_detects_bad_specifiers() {
        assert_eq!(unsupported_strftime("%Y-%m-%d"), None);
        assert_eq!(unsupported_strftime("%Y-%q-%d"), Some('q'));
    }
}
