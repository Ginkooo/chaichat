extern crate camera_capture;
extern crate pancurses;
extern crate image;
extern crate bincode;

use image::imageops::resize;
use std::env;
use std::time::Instant;
use std::io::prelude::*;
use camera_capture::Frame;
use image::imageops::colorops::grayscale;
use image::{RgbImage, ImageBuffer, Luma, FilterType, Rgb};
use std::collections::{HashMap, HashSet};
use std::net::{TcpListener, TcpStream, UdpSocket};
use std::thread;
use std::sync::mpsc::{channel, Sender, Receiver};
use std::mem::transmute;
use pancurses::Window;



type DisplayMap = Vec<((i32, i32), u32)>;

const ASCII_GREYSCALE: &str = "$@B%8&WM#*oahkbdpqwmZO0QLCJUYXzcvunxrjft/\\|()1{}[]?-_+~<>i!lI;:,\"^`'.";
const LOCAL_IP: &str = "127.0.0.1";
const REMOTE_IP: &str = "217.182.75.11";
const DISPLAY_MAP_SZ: usize = 130568;
const TOMEK_IP: &str = "192.168.1.53";

fn draw_map_on_screen(window: &Window, map: DisplayMap) {
    for (position, chr) in map {
        window.mvaddch(position.0, position.1, chr);
    }
}
fn get_display_map(frame: ImageBuffer<Luma<u8>, Vec<u8>>, x: i32, old_map: &mut DisplayMap) -> DisplayMap {
    let mut difference_map = DisplayMap::new();
    for (i, pixel) in frame.enumerate_pixels().enumerate() {
        let pixel_value = pixel.2.data;
        let value = (ASCII_GREYSCALE.len() - 1) * pixel_value[0] as usize/255 + 1;
        let put_y = (i as i32+1)/x;
        let put_x = i as i32 % x;
        let ch = ASCII_GREYSCALE.chars().rev().nth(value).unwrap() as u32;
        difference_map.push(((put_y, put_x), ch));
    }
    difference_map
}

fn fit_frame_to_screen(frame: ImageBuffer<Rgb<u8>, Frame>, y: i32, x: i32) -> ImageBuffer<Luma<u8>, Vec<u8>> {
    let frame = RgbImage::from_raw(frame.width(), frame.height(), frame.to_vec()).unwrap();
    let frame = resize(&frame, x as u32, y as u32, FilterType::Nearest);
    grayscale(&frame)
}

fn get_remote_frames(port: String, received_maps_tx: Sender<DisplayMap>) {
    let socket = UdpSocket::bind(format!("0.0.0.0:{}", port)).unwrap();
    println!("Binding to port {}", port);

    loop {
        let mut arr = vec![0u8; DISPLAY_MAP_SZ];

         socket.recv(&mut arr[..]).unwrap();

        if arr.is_empty() {continue;}
        match bincode::deserialize(&arr[..]) {
            Ok(display_map) => {
                received_maps_tx.send(display_map).unwrap()
            },
            Err(_) => {
                continue;
            }
        };
    }
}

fn send_remote_frames(port: String, rx: Receiver<DisplayMap>) {
    let socket = UdpSocket::bind("0.0.0.0:9797").unwrap();
    for display_map in rx {
        let chunks = display_map.chunks(9000 / 12);
        for chunk in chunks {
            let encoded = bincode::serialize(&chunk).unwrap();
            socket.send_to(&encoded[..], format!("{}:{}", REMOTE_IP, port)).unwrap();
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

    let window = pancurses::initscr();
    let (y, x) = window.get_max_yx();

    if display_from_remote {
        for display_map in received_maps_rx {
            draw_map_on_screen(&window, display_map);
            window.refresh();
        }
    } else {
        run_camera_thread(y, x, camera_maps_tx);
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
