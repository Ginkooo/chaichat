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
use types::{Buffer, CameraFrame, Message};
use utils::run_camera_thread;
use crossterm::terminal::{enable_raw_mode, disable_raw_mode};
use crossterm::event::{poll, read, Event, KeyCode, KeyEvent};
use std::time::Duration;
use std::sync::mpsc::Sender;

fn event_loop(senders: Vec<Sender<Message>>) {
    loop {
        if poll(Duration::from_millis(100)).unwrap() {
            let event = read().unwrap();

            match event {
                Event::Key(key_event) => {
                    if key_event == KeyCode::Esc.into() {
                        for s in &senders {
                            s.send(Message::End).unwrap();
                        }
                    }
                },
                Event::Resize(x, y) => {
                }
                Event::Mouse(e) => {}
            }
        }
    }
}

fn main() {
    let (received_messages_tx, received_messages_rx) = channel();
    let (camera_messages_tx, camera_messages_rx) = channel();
    let (sent_messages_tx, sent_messages_rx) = channel();
    let senders = vec![received_messages_tx.clone(), camera_messages_tx.clone(), sent_messages_tx.clone()];

    let self_bind_port = env::args().nth(1).expect("there is no first argument");
    let other_port = env::args().nth(2);
    let mut display_from_remote = true;
    match other_port {
        Some(port) => {
            display_from_remote = false;
            thread::spawn(move || {
                send_remote_frames(port, sent_messages_rx);
            });
        }
        None => {
            thread::spawn(move || {
                get_remote_frames(self_bind_port, received_messages_tx);
            });
        }
    };

    enable_raw_mode().unwrap();

    thread::spawn(|| {
        event_loop(senders);
    });

    let stdout = stdout();

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).expect("failed to create terminal instance");
    terminal.clear().expect("failed to clear terminal screen");
    let size = terminal.size().expect("failed to get terminal size");
    let width = size.width;
    let height = size.height;

    let buffer = &mut Buffer::new();

    if display_from_remote {
        for message in received_messages_rx {
            match message {
                Message::End => {
                    break;
                },
                Message::CameraFrame(camera_frame) => {
                    for (pos, pixel) in &camera_frame.pixels {
                        buffer.insert(*pos, *pixel);
                    }
                    screen::draw_buffer_on_screen(&mut terminal, buffer);
                }
            }
        }
    } else {
        run_camera_thread(height, width, camera_messages_tx);
        for map in camera_messages_rx {
            match sent_messages_tx.send(map) {
                Ok(_) => {},
                Err(_) => break
            }
        }
    }

    disable_raw_mode().unwrap();
}
