#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod autostart;
mod capture;
mod clipboard;
mod config;

#[cfg(windows)]
mod daemon;
mod export;
mod hotkey;
mod icons;
mod instance;
mod notify;
mod options;

use app::ZenShotApp;
use config::Config;
use eframe::egui::ViewportBuilder;
use std::env;

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() -> eframe::Result<()> {
    #[cfg(windows)]
    attach_parent_console();
    #[cfg(windows)]
    enable_dpi();

    match Mode::from_args(env::args().skip(1).collect()) {
        Mode::Help => {
            print_help();
            Ok(())
        }
        Mode::Version => {
            println!("zenshot {VERSION}");
            Ok(())
        }
        Mode::PrintConfig => {
            match Config::config_path() {
                Some(p) => println!("{}", p.display()),
                None => println!("ZenShot config path could not be resolved."),
            }
            Ok(())
        }
        Mode::Options => options::run(),
        Mode::Daemon => run_daemon(),
        Mode::SetAutostart(on) => {
            if let Err(err) = Config::set_autostart(on) {
                eprintln!("{err}");
            }
            run_daemon()
        }
        Mode::SaveFullscreen => save_fullscreen(),
        #[cfg(target_os = "linux")]
        Mode::ClipboardServe => {
            if let Err(err) = clipboard::serve_from_stdin() {
                eprintln!("{err}");
            }
            Ok(())
        }
        Mode::Capture => run_capture(),
    }
}

fn run_capture() -> eframe::Result<()> {
    let _guard = instance::try_acquire("Local\\ZenShotCapture");
    if cfg!(windows) && _guard.is_none() {
        return Ok(());
    }

    let config = Config::load_or_default();

    let screen_image = match capture::capture_screen(config.capture_cursor) {
        Ok(img) => img,
        Err(err) => {
            eprintln!("Error capturing screen: {err}");
            #[cfg(not(windows))]
            eprintln!("On Wayland, allow the screenshot permission if a portal dialog appears.");
            return Ok(());
        }
    };

    let native_options = overlay_native_options(&screen_image);

    eframe::run_native(
        "ZenShot",
        native_options,
        Box::new(move |cc| {
            let app = ZenShotApp::new(config, screen_image, &cc.egui_ctx);
            Ok(Box::new(app))
        }),
    )
}

fn overlay_native_options(screen_image: &image::RgbaImage) -> eframe::NativeOptions {
    let width = screen_image.width() as f32;
    let height = screen_image.height() as f32;
    let (logical_w, logical_h) = overlay_logical_size(width, height);

    let builder = ViewportBuilder::default()
        .with_title("ZenShot")
        .with_decorations(false)
        .with_resizable(false)
        .with_taskbar(false)
        .with_always_on_top()
        .with_fullscreen(false)
        .with_transparent(true)
        .with_position(eframe::egui::pos2(0.0, 0.0))
        .with_inner_size(eframe::egui::vec2(logical_w, logical_h));

    eframe::NativeOptions {
        persist_window: false,
        dithering: false,
        viewport: builder,
        ..Default::default()
    }
}

fn overlay_logical_size(physical_w: f32, physical_h: f32) -> (f32, f32) {
    let scale = display_scale_factor().max(1.0);
    (physical_w / scale, physical_h / scale)
}

fn display_scale_factor() -> f32 {
    #[cfg(windows)]
    {
        use windows_sys::Win32::Foundation::POINT;
        use windows_sys::Win32::Graphics::Gdi::{MonitorFromPoint, MONITOR_DEFAULTTOPRIMARY};
        use windows_sys::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};

        let pt = POINT { x: 0, y: 0 };
        let hmon = unsafe { MonitorFromPoint(pt, MONITOR_DEFAULTTOPRIMARY) };
        let mut dpi_x = 96u32;
        let mut dpi_y = 96u32;
        let res = unsafe { GetDpiForMonitor(hmon, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y) };
        if res == 0 && dpi_x > 0 {
            (dpi_x as f32 / 96.0).max(1.0)
        } else {
            (unsafe { windows_sys::Win32::UI::HiDpi::GetDpiForSystem() } as f32 / 96.0).max(1.0)
        }
    }
    #[cfg(not(windows))]
    {
        let Ok(monitors) = xcap::Monitor::all() else {
            return 1.0;
        };
        let monitor = monitors
            .iter()
            .find(|m| m.is_primary().unwrap_or(false))
            .or_else(|| monitors.first());
        monitor
            .and_then(|m| m.scale_factor().ok())
            .unwrap_or(1.0)
            .max(1.0)
    }
}

fn save_fullscreen() -> eframe::Result<()> {
    let config = Config::load_or_default();
    match capture::capture_screen(config.capture_cursor) {
        Ok(img) => match export::save_image(&img, &config) {
            Ok(path) => {
                if config.show_notifications {
                    notify::saved(&path);
                }
                println!("{}", path.display());
            }
            Err(err) => eprintln!("{err}"),
        },
        Err(err) => eprintln!("{err}"),
    }
    Ok(())
}

fn run_daemon() -> eframe::Result<()> {
    #[cfg(windows)]
    {
        daemon::run()
    }
    #[cfg(not(windows))]
    {
        eprintln!(
            "ZenShot does not stay resident on Linux.\n\
             Bind Ctrl+Shift+S in your desktop settings to `zenshot`,\n\
             and open settings with `zenshot --options`."
        );
        Ok(())
    }
}

#[cfg(windows)]
fn attach_parent_console() {
    // Release builds are a GUI subsystem binary, so Explorer would otherwise
    // allocate a console. Attaching to an already-open terminal keeps
    // `zenshot --help` visible when launched from cmd/PowerShell.
    const ATTACH_PARENT_PROCESS: u32 = 0xFFFF_FFFF;
    unsafe {
        let _ = windows_sys::Win32::System::Console::AttachConsole(ATTACH_PARENT_PROCESS);
    }
}

#[cfg(windows)]
fn enable_dpi() {
    unsafe {
        let _ = windows_sys::Win32::UI::HiDpi::SetProcessDpiAwarenessContext(
            windows_sys::Win32::UI::HiDpi::DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
        );
    }
}

enum Mode {
    Capture,
    Daemon,
    Options,
    SaveFullscreen,
    PrintConfig,
    Help,
    Version,
    SetAutostart(bool),
    #[cfg(target_os = "linux")]
    ClipboardServe,
}

impl Mode {
    fn from_args(args: Vec<String>) -> Self {
        #[cfg(target_os = "linux")]
        if args.first().map(String::as_str) == Some(clipboard::CLIPBOARD_SERVE_ARG) {
            return Self::ClipboardServe;
        }
        if args.iter().any(|a| a == "--help" || a == "-h") {
            return Self::Help;
        }
        if args.iter().any(|a| a == "--version" || a == "-V") {
            return Self::Version;
        }
        if args.iter().any(|a| a == "--config") {
            return Self::PrintConfig;
        }
        if args.iter().any(|a| a == "--options" || a == "-o") {
            return Self::Options;
        }
        if args.iter().any(|a| a == "--enable-autostart") {
            return Self::SetAutostart(true);
        }
        if args.iter().any(|a| a == "--disable-autostart") {
            return Self::SetAutostart(false);
        }
        if args.iter().any(|a| a == "--daemon") {
            return Self::Daemon;
        }
        if args.iter().any(|a| a == "--save-fullscreen") {
            return Self::SaveFullscreen;
        }
        if args.iter().any(|a| a == "--capture" || a == "-c") {
            return Self::Capture;
        }
        if let Some(unknown) = args.iter().find(|a| a.starts_with('-')) {
            eprintln!("Unknown option: {unknown}");
            return Self::Help;
        }

        if cfg!(windows) {
            Self::Daemon
        } else {
            Self::Capture
        }
    }
}

fn print_help() {
    println!("ZenShot {VERSION} — featherlight screen capture");
    println!();
    println!("Usage: zenshot [OPTIONS]");
    println!();
    println!("Options:");
    println!("  --capture, -c       Open the region-select overlay");
    println!("  --options, -o       Open the settings window");
    println!("  --save-fullscreen   Capture the whole screen and save");
    println!("  --daemon            Stay in the tray (Windows)");
    println!("  --enable-autostart  Start with Windows, then stay in the tray");
    println!("  --disable-autostart Do not start with Windows, then stay in the tray");
    println!("  --config            Print the configuration file path");
    println!("  -V, --version       Print version");
    println!("  -h, --help          Show this help");
    println!();
    if cfg!(windows) {
        println!("With no arguments on Windows, ZenShot starts in the tray.");
        println!("Print Screen (or the hotkey in Options) opens the overlay.");
    } else {
        println!("With no arguments on Linux, ZenShot opens the overlay.");
        println!("Bind Ctrl+Shift+S in your desktop settings to `zenshot`.");
    }
    println!();
    println!("Shortcuts inside overlay:");
    println!("  Ctrl+A      Select the full screen");
    println!("  Ctrl+C      Copy selection to clipboard & exit");
    println!("  Ctrl+S      Save screenshot & exit");
    println!("  Ctrl+P      Print selection & exit");
    println!("  Ctrl+Z      Undo last annotation");
    println!("  Ctrl+X / Esc  Close without saving");
}
