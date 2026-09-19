use interprocess::local_socket::{prelude::*, GenericNamespaced, ListenerOptions, Stream};
use std::io::{Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub const SOCKET_NAME: &str = "zenshot.sock";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum IpcCommand {
    Capture,
    Quit,
}

/// Attempts to send a capture command to a running warm ZenShot instance.
/// Returns true if the message was delivered, false if no warm instance is listening.
pub fn send_command(cmd: IpcCommand) -> bool {
    let Ok(name) = SOCKET_NAME.to_ns_name::<GenericNamespaced>() else {
        return false;
    };
    let Ok(mut stream) = Stream::connect(name) else {
        return false;
    };

    let _ = stream.set_recv_timeout(Some(std::time::Duration::from_millis(300)));

    let msg: &[u8] = match cmd {
        IpcCommand::Capture => b"capture\n",
        IpcCommand::Quit => b"quit\n",
    };

    if stream.write_all(msg).is_err() || stream.flush().is_err() {
        return false;
    }

    let mut ack = [0u8; 4];
    stream.read(&mut ack).is_ok()
}

/// Spawns a background thread listening for IPC commands.
pub fn start_listener(
    trigger_flag: Arc<AtomicBool>,
    quit_flag: Arc<AtomicBool>,
    ctx: eframe::egui::Context,
) -> std::io::Result<()> {
    let name = SOCKET_NAME
        .to_ns_name::<GenericNamespaced>()
        .map_err(|e| std::io::Error::other(e.to_string()))?;

    let listener = ListenerOptions::new()
        .name(name)
        .create_sync()?;

    std::thread::Builder::new()
        .name("zenshot-ipc".to_string())
        .spawn(move || {
            for conn in listener.incoming() {
                if let Ok(mut stream) = conn {
                    let mut buf = [0u8; 32];
                    if let Ok(n) = stream.read(&mut buf) {
                        let text = std::str::from_utf8(&buf[..n]).unwrap_or("").trim();
                        if text == "capture" {
                            trigger_flag.store(true, Ordering::SeqCst);
                            ctx.request_repaint();
                            let _ = stream.write_all(b"ok\n");
                            let _ = stream.flush();
                        } else if text == "quit" {
                            quit_flag.store(true, Ordering::SeqCst);
                            ctx.request_repaint();
                            let _ = stream.write_all(b"ok\n");
                            let _ = stream.flush();
                        }
                    }
                }
            }
        })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ipc_roundtrip() {
        let trigger = Arc::new(AtomicBool::new(false));
        let quit = Arc::new(AtomicBool::new(false));
        let ctx = eframe::egui::Context::default();

        if start_listener(trigger.clone(), quit.clone(), ctx).is_err() {
            return;
        }

        assert!(send_command(IpcCommand::Capture));
        assert!(trigger.load(Ordering::SeqCst));

        assert!(send_command(IpcCommand::Quit));
        assert!(quit.load(Ordering::SeqCst));
    }
}
