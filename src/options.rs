use crate::config::{Config, OutputFormat};
use crate::hotkey::{self, Hotkey};
use eframe::egui::{self, Color32, RichText, Vec2, ViewportBuilder};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    General,
    Hotkeys,
    Formats,
}

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
    let mut config = Config::load_or_default();
    #[cfg(windows)]
    {
        config.autostart = crate::autostart::is_enabled();
    }
    let native_options = eframe::NativeOptions {
        viewport: ViewportBuilder::default()
            .with_title("Options")
            .with_inner_size([540.0, 360.0])
            .with_min_inner_size([480.0, 320.0])
            .with_resizable(true),
        ..Default::default()
    };
    eframe::run_native(
        "ZenShot Options",
        native_options,
        Box::new(|_cc| Ok(Box::new(OptionsApp::new(config)))),
    )
}

impl eframe::App for OptionsApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Some(target) = self.listening {
            if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                self.listening = None;
            } else if let Some(hotkey) = read_binding(ctx) {
                match target {
                    ListenTarget::Capture => self.config.hotkey_capture = hotkey,
                    ListenTarget::SaveFullscreen => self.config.hotkey_save_fullscreen = hotkey,
                }
                self.listening = None;
            }
        }

        egui::TopBottomPanel::top("tabs").show(ctx, |ui| {
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.tab, Tab::General, "General");
                if cfg!(windows) {
                    ui.selectable_value(&mut self.tab, Tab::Hotkeys, "Hotkeys");
                }
                ui.selectable_value(&mut self.tab, Tab::Formats, "Formats");
            });
            ui.add_space(4.0);
            ui.separator();
        });

        egui::TopBottomPanel::bottom("buttons").show(ctx, |ui| {
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if let Some(status) = &self.status {
                    ui.colored_label(Color32::from_rgb(180, 60, 60), status);
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Cancel").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                    if ui
                        .add_sized(Vec2::new(72.0, 24.0), egui::Button::new("OK"))
                        .clicked()
                    {
                        self.apply(ctx);
                    }
                });
            });
            ui.add_space(6.0);
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add_space(8.0);
            match self.tab {
                Tab::General => self.ui_general(ui),
                Tab::Hotkeys => self.ui_hotkeys(ui),
                Tab::Formats => self.ui_formats(ui),
            }
        });
    }
}

impl OptionsApp {
    fn ui_general(&mut self, ui: &mut egui::Ui) {
        ui.add_space(4.0);

        #[cfg(windows)]
        {
            ui.checkbox(&mut self.config.autostart, "Start ZenShot when I log in");
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

        ui.add_space(12.0);
        ui.label("Save directory");
        ui.add(egui::TextEdit::singleline(&mut self.config.save_dir).desired_width(460.0));

        ui.add_space(8.0);
        ui.label("Filename pattern (strftime)");
        ui.add(egui::TextEdit::singleline(&mut self.config.filename_format).desired_width(460.0));

        #[cfg(not(windows))]
        {
            ui.add_space(16.0);
            ui.label(
                RichText::new(
                    "Linux does not need a tray process. Bind PrintScreen (or any key) in your desktop settings to the command `zenshot`.",
                )
                .weak(),
            );
        }
    }

    fn ui_hotkeys(&mut self, ui: &mut egui::Ui) {
        ui.add_space(4.0);
        ui.label(RichText::new("Click a shortcut, then press the new key combination. Esc cancels.").weak());
        ui.add_space(10.0);

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

        ui.add_space(14.0);
        if ui.button("Use Print Screen").clicked() {
            self.config.hotkey_capture = Hotkey::PRINT_SCREEN;
            self.config.hotkey_capture_enabled = true;
            self.listening = None;
        }
    }

    fn ui_formats(&mut self, ui: &mut egui::Ui) {
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label("Save using the format");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                egui::ComboBox::from_id_salt("fmt")
                    .selected_text(self.config.output_format.label())
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.config.output_format, OutputFormat::Png, "PNG");
                        ui.selectable_value(&mut self.config.output_format, OutputFormat::Jpeg, "JPEG");
                    });
            });
        });

        ui.add_space(12.0);
        ui.horizontal(|ui| {
            ui.label("JPEG quality");
            ui.add_enabled_ui(self.config.output_format == OutputFormat::Jpeg, |ui| {
                ui.add(egui::Slider::new(&mut self.config.jpeg_quality, 10..=100).show_value(false));
                let mut q = self.config.jpeg_quality as i32;
                if ui
                    .add(egui::DragValue::new(&mut q).range(10..=100).speed(1.0))
                    .changed()
                {
                    self.config.jpeg_quality = q as u8;
                }
            });
        });
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
        ui.checkbox(enabled, "");
        ui.label(label);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let caption = if listening.is_some() && matches_target(listening, target) {
                "Press a shortcut...".to_string()
            } else {
                hotkey.display()
            };
            let mut text = caption;
            let response = ui.add_enabled(
                *enabled,
                egui::TextEdit::singleline(&mut text).desired_width(180.0),
            );
            if response.gained_focus() || response.clicked() {
                *listening_slot = Some(target);
            }
        });
    });
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
