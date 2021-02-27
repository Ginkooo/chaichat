mod network;
mod screen;
mod types;
mod utils;

use network::{get_remote_frames, send_remote_frames};
use std::env;
use std::io::stdout;
use std::sync::mpsc::channel;
use std::thread;
use tui::backend::CrosstermBackend;
use tui::Terminal;
use types::Buffer;
use utils::run_camera_thread;

fn main() {
    let (received_frames_tx, received_frames_rx) = channel();
    let (camera_frames_tx, camera_frames_rx) = channel();
    let (sent_maps_tx, sent_maps_rx) = channel();
    let self_bind_port = env::args().nth(1).expect("there is no first argument");
    let other_port = env::args().nth(2);
    let mut display_from_remote = true;
    let mut read_thread: Option<thread::JoinHandle<()>> = None;
    let mut send_thread: Option<thread::JoinHandle<()>> = None;
    match other_port {
        Some(port) => {
            display_from_remote = false;
            send_thread = Some(thread::spawn(move || {
                send_remote_frames(port, sent_maps_rx);
            }));
        }
        None => {
            read_thread = Some(thread::spawn(move || {
                get_remote_frames(self_bind_port, received_frames_tx);
            }));
        }
    };

    let stdout = stdout();

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).expect("failed to create terminal instance");
    terminal.clear().expect("failed to clear terminal screen");
    let size = terminal.size().expect("failed to get terminal size");
    let width = size.width;
    let height = size.height;

    let buffer = &mut Buffer::new();

    if display_from_remote {
        for camera_frame in received_frames_rx {
            for (pos, pixel) in &camera_frame.pixels {
                buffer.insert(*pos, *pixel);
            }
            screen::draw_buffer_on_screen(&mut terminal, buffer);
        }
    } else {
        run_camera_thread(height, width, camera_frames_tx);
        for map in camera_frames_rx {
            sent_maps_tx
                .send(map)
                .expect("failed to send camera frame to channel");
        }
    }

    match read_thread {
        Some(t) => {
            t.join().expect("failed to join read thread");
        }
        None => {}
    };
    match send_thread {
        Some(t) => t.join().expect("failed to join send thread"),
        None => {}
    };
}
