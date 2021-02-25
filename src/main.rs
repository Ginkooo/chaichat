mod screen;
mod types;
mod network;
mod utils;
mod chunk;

use std::env;
use std::thread;
use std::sync::mpsc::channel;
use std::io::stdout;
use tui::Terminal;
use tui::backend::CrosstermBackend;
use types::Buffer;
use network::{get_remote_frames, send_remote_frames};
use utils::run_camera_thread;


fn main() {
    let (received_maps_tx, received_maps_rx) = channel();
    let (camera_maps_tx, camera_maps_rx) = channel();
    let (sent_maps_tx, sent_maps_rx) = channel();
    let self_bind_port = env::args().nth(1).unwrap();
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
        },
        None => {
            read_thread = Some(thread::spawn(move || {
                get_remote_frames(self_bind_port, received_maps_tx);
            }));
        }
    };


    let stdout = stdout();

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.clear().unwrap();
    let width = terminal.size().unwrap().width;
    let height = terminal.size().unwrap().height;

    let buffer = &mut Buffer::new();

    if display_from_remote {
        for display_map in received_maps_rx {
            for (pos, pixel) in &display_map {
                buffer.insert(*pos, *pixel);
            }
            screen::draw_buffer_on_screen(&mut terminal, buffer);
        }
    } else {
        run_camera_thread(height as i32, width as i32, camera_maps_tx);
        for map in camera_maps_rx {
            sent_maps_tx.send(map).unwrap();
        }
    }

    match read_thread {
        Some(t) => {t.join().unwrap();},
        None => {}
    };
    match send_thread {
        Some(t) => {t.join().unwrap()},
        None => {}
    };
}
