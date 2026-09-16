mod app;
mod capture;
mod clipboard;
mod config;
mod icons;

use app::ZenShotApp;
use config::Config;
use eframe::egui::ViewportBuilder;
use std::env;

fn main() -> eframe::Result<()> {
    let args: Vec<String> = env::args().collect();

    // CLI option handling
    if args.contains(&"--help".to_string()) || args.contains(&"-h".to_string()) {
        println!("ZenShot - The 100% Lightshot Clone for Linux & Windows");
        println!("Version 0.1.0 (Zero-Disk In-Memory Performance)");
        println!();
        println!("Usage: zenshot [OPTIONS]");
        println!();
        println!("Options:");
        println!("  --config    Print the path to the configuration file");
        println!("  -h, --help  Show this help message");
        println!();
        println!("Shortcuts inside overlay:");
        println!("  Ctrl+C      Copy selection to clipboard in RAM & exit");
        println!("  Ctrl+S      Save screenshot to configured directory & exit");
        println!("  Ctrl+Z      Undo last drawn annotation");
        println!("  Esc         Cancel and exit immediately (zero I/O)");
        return Ok(());
    }

    if args.contains(&"--config".to_string()) {
        if let Some(p) = Config::config_path() {
            println!("ZenShot config path: {}", p.display());
        } else {
            println!("ZenShot config path could not be resolved.");
        }
        return Ok(());
    }

    let config = Config::load_or_default();

    // Freeze screen into RAM before showing window to match Lightshot instant capture
    let screen_image = match capture::capture_screen() {
        Ok(img) => img,
        Err(err) => {
            eprintln!("Error capturing screen: {}", err);
            return Ok(());
        }
    };

    let width = screen_image.width() as f32;
    let height = screen_image.height() as f32;

    let native_options = eframe::NativeOptions {
        viewport: ViewportBuilder::default()
            .with_title("ZenShot")
            .with_fullscreen(true)
            .with_decorations(false)
            .with_always_on_top()
            .with_inner_size([width, height]),
        ..Default::default()
    };

    eframe::run_native(
        "ZenShot",
        native_options,
        Box::new(|_cc| Ok(Box::new(ZenShotApp::new(config, screen_image)))),
    )
}
