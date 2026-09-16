use eframe::egui::{self, ColorImage, TextureHandle, TextureOptions};

/// Container for all embedded Lightshot toolbar icon textures.
pub struct ToolbarIcons {
    // Horizontal Toolbar icons (100% local, no privacy-leaking upload)
    pub print: TextureHandle,
    pub copy: TextureHandle,
    pub save: TextureHandle,
    pub close: TextureHandle,

    // Vertical Toolbar icons
    pub pen: TextureHandle,
    pub line: TextureHandle,
    pub arrow: TextureHandle,
    pub rect: TextureHandle,
    pub marker: TextureHandle,
    pub text: TextureHandle,
    pub undo: TextureHandle,
}

impl ToolbarIcons {
    pub fn load(ctx: &egui::Context) -> Self {
        Self {
            print: load_texture(ctx, "icon_print", include_bytes!("../assets/icons/btn_print.png")),
            copy: load_texture(ctx, "icon_copy", include_bytes!("../assets/icons/btn_copy.png")),
            save: load_texture(ctx, "icon_save", include_bytes!("../assets/icons/btn_save.png")),
            close: load_texture(ctx, "icon_close", include_bytes!("../assets/icons/btn_close.png")),

            pen: load_texture(ctx, "icon_pen", include_bytes!("../assets/icons/btn_pen.png")),
            line: load_texture(ctx, "icon_line", include_bytes!("../assets/icons/btn_line.png")),
            arrow: load_texture(ctx, "icon_arrow", include_bytes!("../assets/icons/btn_arrow.png")),
            rect: load_texture(ctx, "icon_rect", include_bytes!("../assets/icons/btn_rect.png")),
            marker: load_texture(ctx, "icon_marker", include_bytes!("../assets/icons/btn_marker.png")),
            text: load_texture(ctx, "icon_text", include_bytes!("../assets/icons/btn_text.png")),
            undo: load_texture(ctx, "icon_undo", include_bytes!("../assets/icons/btn_undo.png")),
        }
    }
}

fn load_texture(ctx: &egui::Context, id: &str, png_bytes: &[u8]) -> TextureHandle {
    let img = image::load_from_memory(png_bytes)
        .expect("Embedded toolbar icon failed to load")
        .to_rgba8();
    let size = [img.width() as usize, img.height() as usize];
    let pixels = img.into_raw();
    let color_image = ColorImage::from_rgba_unmultiplied(size, &pixels);
    ctx.load_texture(id, color_image, TextureOptions::LINEAR)
}
