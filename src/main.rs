extern crate camera_capture;
extern crate ncurses;
extern crate image;
extern crate bincode;

use image::imageops::resize;
use std::env;
use std::io::prelude::*;
use camera_capture::Frame;
use image::imageops::colorops::grayscale;
use image::{RgbImage, ImageBuffer, Luma, FilterType, Rgb};
use std::collections::HashMap;
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::sync::mpsc::{channel, Sender, Receiver};
use std::mem::transmute;



type DisplayMap = HashMap<u32, Vec<(i32, i32)>>;

const ASCII_GREYSCALE: &str = "$@B%8&WM#*oahkbdpqwmZO0QLCJUYXzcvunxrjft/\\|()1{}[]?-_+~<>i!lI;:,\"^`'.";

fn draw_map_on_screen(map: HashMap<u32, Vec<(i32, i32)>>) {
    for (chr, positions) in map {
        for pos in positions {
            ncurses::mvaddch(pos.0, pos.1, chr);
        }
    }
}
fn get_display_map(frame: ImageBuffer<Luma<u8>, Vec<u8>>, x: i32) -> DisplayMap {
    let mut map = HashMap::new();
    for (i, pixel) in frame.enumerate_pixels().enumerate() {
        let pixel_value = pixel.2.data;
        let value = (ASCII_GREYSCALE.len() - 1) * pixel_value[0] as usize/255 + 1;
        let put_y = (i as i32+1)/x;
        let put_x = i as i32 % x;
        let ch = ASCII_GREYSCALE.chars().rev().nth(value).unwrap() as u32;
        map.entry(ch).or_insert(vec!()).push((put_y, put_x));
    }
    map
}

fn fit_frame_to_screen(frame: ImageBuffer<Rgb<u8>, Frame>, y: i32, x: i32) -> ImageBuffer<Luma<u8>, Vec<u8>> {
    let frame = RgbImage::from_raw(frame.width(), frame.height(), frame.to_vec()).unwrap();
    let frame = resize(&frame, x as u32, y as u32, FilterType::Nearest);
    grayscale(&frame)
}

fn get_remote_frames(port: String, received_maps_tx: Sender<DisplayMap>) {
    let listener = TcpListener::bind(format!("127.0.0.1:{}", port)).unwrap();
    println!("Binding to port {}", port);

    for stream in listener.incoming() {
        let mut stream = stream.unwrap();

        println!("starting reading remote images");
        loop {
            let mut buf = [0u8; 4];
            stream.read_exact(&mut buf).unwrap();
            let sz = unsafe {transmute::<[u8; 4], u32>(buf)};

            let mut arr = vec![0u8; sz as usize];

            stream.read_exact(&mut arr[..]).unwrap();


            if buf.is_empty() {continue;}
            let display_map = bincode::deserialize(&arr[..]).unwrap();
            received_maps_tx.send(display_map).unwrap();
        }
    }
}

fn send_remote_frames(port: String, rx: Receiver<DisplayMap>) {
    let mut stream = TcpStream::connect(format!("127.0.0.1:{}", port)).unwrap();
    for display_map in rx {
        let encoded = bincode::serialize(&display_map).unwrap();
        let to_send = &encoded[..];
        let len = to_send.len() as u32;
        stream.write_all(&len.to_le_bytes()).unwrap();
        stream.write_all(&encoded[..]).unwrap();
    }
}

fn run_camera_thread(camera_maps_tx: Sender<DisplayMap>) {
    let window = ncurses::initscr();
    let mut x: i32 = 0;
    let mut y: i32 = 0;
    ncurses::getmaxyx(window, &mut y, &mut x);
    ncurses::endwin();

    let cam = camera_capture::create(0).unwrap();
    let cam = cam.fps(15.0).unwrap().start().unwrap();
    thread::spawn(move || {
        for frame in cam {
            let frame = fit_frame_to_screen(frame, y, x);
            let map = get_display_map(frame, x);
            camera_maps_tx.send(map).unwrap();
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

    if display_from_remote {
        ncurses::initscr();
        for display_map in received_maps_rx {
            draw_map_on_screen(display_map);
            ncurses::refresh();
        }
    } else {
        run_camera_thread(camera_maps_tx);
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
