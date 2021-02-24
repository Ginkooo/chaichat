extern crate camera_capture;
extern crate pancurses;
extern crate image;
extern crate bincode;
extern crate crossterm;
extern crate tui;
extern crate chrono;
extern crate termion;

use image::imageops::resize;
use std::env;
use camera_capture::Frame;
use image::imageops::colorops::grayscale;
use std::collections::HashMap;
use image::{RgbImage, ImageBuffer, Luma, FilterType, Rgb};
use std::net::{UdpSocket};
use std::thread;
use std::sync::mpsc::{channel, Sender, Receiver};
use std::io::{Write, stdout};
use tui::Terminal;
use tui::backend::{CrosstermBackend, TermionBackend};
use tui::widgets::{Block, Borders, Clear, canvas::{Canvas, Rectangle, Map, MapResolution}};
use tui::style::Color;
use std::io;
use std::time::Instant;
use std::mem::size_of;
use crossterm::{
    terminal::{enable_raw_mode, disable_raw_mode, EnterAlternateScreen},
    execute
};



type TerminalWithBackend = Terminal<CrosstermBackend<io::Stdout>>;
type DisplayMap = Vec<((i32, i32), [u8; 3])>;
type Buffer = HashMap<(i32, i32), [u8; 3]>;

const ASCII_GREYSCALE: &str = "$@B%8&WM#*oahkbdpqwmZO0QLCJUYXzcvunxrjft/\\|()1{}[]?-_+~<>i!lI;:,\"^`'.";
const LOCAL_IP: &str = "127.0.0.1";
const REMOTE_IP: &str = "217.182.75.11";
const DISPLAY_MAP_SZ: usize = 130568;
const TOMEK_IP: &str = "192.168.1.53";

fn ascii_position(ch: char) -> usize {
    ASCII_GREYSCALE.chars().position(|c| c == ch).unwrap()
}

fn draw_buffer_on_screen(terminal: &mut TerminalWithBackend, buffer: &mut Buffer) {
    terminal.draw(|f| {
        let size = f.size();
        let width = size.width as f64;
        let height = size.height as f64;
        let canvas = Canvas::default()
            .x_bounds([0.0, width])
            .y_bounds([0.0, height])
            .paint(|ctx| {
                for ((x, y), pixel) in buffer.clone() {
                    ctx.draw(&Rectangle{
                        x: width -x as f64,
                        y: height - y as f64,
                        width: 1.0,
                        height: 1.0,
                        color: Color::Rgb(pixel[0], pixel[1], pixel[2]),
                    });
                }
            });
        f.render_widget(canvas, size);
    }).unwrap();
}
fn get_display_map(frame: ImageBuffer<Rgb<u8>, Vec<u8>>, x: i32, old_map: &mut DisplayMap) -> DisplayMap {
    let mut difference_map = DisplayMap::new();
    let mut all_map = DisplayMap::new();
    for (x, y, rgb) in frame.enumerate_pixels() {
        all_map.push(((x as i32, y as i32), rgb.data));
    }
    if old_map.is_empty() {
        *old_map = all_map.clone();
        return all_map;
    }
    for (old_px, new_px) in old_map.iter().zip(all_map.iter()) {
        let old_sum: i32 = old_px.1.iter().map(|&x| x as i32).sum();
        let new_sum: i32 = new_px.1.iter().map(|&x| x as i32).sum();
        let diff: i32 = old_sum - new_sum;
        if diff.abs() > 10 {
            difference_map.push(((new_px.0.0, new_px.0.1), new_px.1));
        }
    }
    *old_map = all_map;
    difference_map
}

fn fit_frame_to_screen(frame: ImageBuffer<Rgb<u8>, Frame>, y: i32, x: i32) -> ImageBuffer<Rgb<u8>, Vec<u8>> {
    let frame = RgbImage::from_raw(frame.width(), frame.height(), frame.to_vec()).unwrap();
    resize(&frame, x as u32, y as u32, FilterType::Nearest)
}

fn get_remote_frames(port: String, received_maps_tx: Sender<DisplayMap>) {
    let socket = UdpSocket::bind(format!("0.0.0.0:{}", port)).unwrap();

    loop {
        let mut arr = vec![0u8; DISPLAY_MAP_SZ];

        socket.recv(&mut arr[..]).unwrap();

        if arr.is_empty() {continue;}
        match bincode::deserialize(&arr[..]) {
            Ok(display_map) => {
                received_maps_tx.send(display_map).unwrap();
            },
            Err(_) => {
                println!("Error recieving frame");
                continue;
            }
        };
    }
}

fn send_remote_frames(port: String, rx: Receiver<DisplayMap>) {
    let socket = UdpSocket::bind("0.0.0.0:9797").unwrap();
    for display_map in rx {
        let chunks = display_map.chunks(8972 / size_of::<DisplayMap>());
        for chunk in chunks {
            let encoded = bincode::serialize(&chunk).unwrap();
            socket.send_to(&encoded[..], format!("{}:{}", LOCAL_IP, port)).unwrap();
        }
    }
}

fn run_camera_thread(y: i32, x: i32, camera_maps_tx: Sender<DisplayMap>) {
    thread::spawn(move || {
        let mut old_map = DisplayMap::new();

        let cam = camera_capture::create(0).unwrap();
        let cam = cam.fps(30.0).unwrap().start().unwrap();
        for frame in cam {
            let frame = fit_frame_to_screen(frame, y, x);
            let difference_map = get_display_map(frame, x, &mut old_map);
            camera_maps_tx.send(difference_map).unwrap();
        }
    });
}

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
            draw_buffer_on_screen(&mut terminal, buffer);
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
