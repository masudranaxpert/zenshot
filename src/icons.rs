use eframe::egui::{self, ColorImage, TextureHandle, TextureOptions};

/// One toolbar icon in both of Lightshot's render states: the near-black
/// glyph shown while idle and the cyan-blue glyph shown on hover or while
/// the tool is selected. Both come straight from Lightshot's own resources
/// at 2x (40x40 / 48x40), so the GPU always downsamples instead of
/// magnifying pixel art.
pub struct IconPair {
    pub normal: TextureHandle,
    pub active: TextureHandle,
}

impl IconPair {
    pub fn get(&self, active: bool) -> &TextureHandle {
        if active { &self.active } else { &self.normal }
    }
}

/// Container for all embedded Lightshot toolbar icon textures.
pub struct ToolbarIcons {
    // Horizontal Toolbar icons (100% local, no privacy-leaking upload)
    pub print: IconPair,
    pub copy: IconPair,
    pub save: IconPair,
    pub close: IconPair,

    // Vertical Toolbar icons
    pub pen: IconPair,
    pub line: IconPair,
    pub arrow: IconPair,
    pub rect: IconPair,
    pub marker: IconPair,
    pub text: IconPair,
    pub undo: IconPair,

    // Colour swatch frame (the current colour is tinted through it)
    pub color: TextureHandle,
}

impl ToolbarIcons {
    pub fn load(ctx: &egui::Context) -> Self {
        Self {
            print: pair(ctx, "print", include_bytes!("../assets/icons/btn_print.png"), include_bytes!("../assets/icons/btn_print_active.png")),
            copy: pair(ctx, "copy", include_bytes!("../assets/icons/btn_copy.png"), include_bytes!("../assets/icons/btn_copy_active.png")),
            save: pair(ctx, "save", include_bytes!("../assets/icons/btn_save.png"), include_bytes!("../assets/icons/btn_save_active.png")),
            close: pair(ctx, "close", include_bytes!("../assets/icons/btn_close.png"), include_bytes!("../assets/icons/btn_close_active.png")),

            pen: pair(ctx, "pen", include_bytes!("../assets/icons/btn_pen.png"), include_bytes!("../assets/icons/btn_pen_active.png")),
            line: pair(ctx, "line", include_bytes!("../assets/icons/btn_line.png"), include_bytes!("../assets/icons/btn_line_active.png")),
            arrow: pair(ctx, "arrow", include_bytes!("../assets/icons/btn_arrow.png"), include_bytes!("../assets/icons/btn_arrow_active.png")),
            rect: pair(ctx, "rect", include_bytes!("../assets/icons/btn_rect.png"), include_bytes!("../assets/icons/btn_rect_active.png")),
            marker: pair(ctx, "marker", include_bytes!("../assets/icons/btn_marker.png"), include_bytes!("../assets/icons/btn_marker_active.png")),
            text: pair(ctx, "text", include_bytes!("../assets/icons/btn_text.png"), include_bytes!("../assets/icons/btn_text_active.png")),
            undo: pair(ctx, "undo", include_bytes!("../assets/icons/btn_undo.png"), include_bytes!("../assets/icons/btn_undo_active.png")),

            color: load_texture(ctx, "color", include_bytes!("../assets/icons/btn_color.png")),
        }
    }
}

fn pair(ctx: &egui::Context, name: &str, normal: &[u8], active: &[u8]) -> IconPair {
    IconPair {
        normal: load_texture_named(ctx, &format!("icon_{name}"), normal),
        active: load_texture_named(ctx, &format!("icon_{name}_active"), active),
    }
}

/// The 2x originals already carry Lightshot's own anti-aliasing, so linear
/// filtering plus mipmaps keeps every edge smooth at 100%, 125%, 150% and
/// 200% display scaling (glow backend generates the mip chain).
fn texture_options() -> TextureOptions {
    TextureOptions {
        mipmap_mode: Some(egui::TextureFilter::Linear),
        ..TextureOptions::LINEAR
    }
}

fn load_texture(ctx: &egui::Context, name: &str, png_bytes: &[u8]) -> TextureHandle {
    load_texture_named(ctx, &format!("icon_{name}"), png_bytes)
}

/// Title-bar / taskbar icon. Without it every eframe window shows the
/// framework's own "e" placeholder instead of ZenShot's feather.
pub fn window_icon() -> egui::IconData {
    let img = image::load_from_memory(include_bytes!("../assets/zenshot.png"))
        .expect("embedded window icon failed to load")
        .to_rgba8();
    egui::IconData {
        width: img.width(),
        height: img.height(),
        rgba: img.into_raw(),
    }
}

fn load_texture_named(ctx: &egui::Context, id: &str, png_bytes: &[u8]) -> TextureHandle {
    let img = image::load_from_memory(png_bytes)
        .unwrap_or_else(|e| panic!("Embedded toolbar icon {id} failed to load: {e}"))
        .to_rgba8();
    let size = [img.width() as usize, img.height() as usize];
    let pixels = img.into_raw();
    let color_image = ColorImage::from_rgba_unmultiplied(size, &pixels);
    ctx.load_texture(id, color_image, texture_options())
}
