extern crate camera_capture;
extern crate ncurses;
extern crate image;

use image::imageops::resize;
use image::imageops::colorops::grayscale;
use image::FilterType;
use image::RgbImage;
use std::time::Instant;
use std::sync::mpsc;



const ASCII_GREYSCALE: &str = "$@B%8&WM#*oahkbdpqwmZO0QLCJUYXzcvunxrjft/\\|()1{}[]?-_+~<>i!lI;:,\"^`'.";

fn main() {
    let cam = camera_capture::create(0).unwrap();
    let cam = cam.fps(30.0).unwrap().start().unwrap();
    let window = ncurses::initscr();
    let mut x: i32 = 0;
    let mut y: i32 = 0;
    ncurses::getmaxyx(window, &mut y, &mut x);
    for frame in cam {
        let frame = RgbImage::from_raw(frame.width(), frame.height(), frame.to_vec()).unwrap();
        let frame = resize(&frame, x as u32, y as u32, FilterType::Nearest);
        let frame = grayscale(&frame);
        let start = Instant::now();
        for (i, pixel) in frame.enumerate_pixels().enumerate() {
            let dupa = pixel.2.data;
            let value = (ASCII_GREYSCALE.len() - 1) * dupa[0] as usize/255 + 1;
            let put_y = (i as i32+1)/x;
            let put_x = i as i32 % x;
            let ch = ASCII_GREYSCALE.chars().rev().nth(value).unwrap() as u32;
            ncurses::mvaddch(put_y, put_x, ch);
        }
        ncurses::refresh();
        let duration = Instant::now().duration_since(start);
        println!("{}", duration.as_millis());
    }
}
