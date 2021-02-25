use crate::types::DisplayMap;
use image::imageops::resize;
use camera_capture::Frame;
use image::{RgbImage, ImageBuffer, FilterType, Rgb};
use std::sync::mpsc::Sender;
use std::thread;

fn get_display_map(frame: ImageBuffer<Rgb<u8>, Vec<u8>>, old_map: &mut DisplayMap) -> DisplayMap {
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
        if diff.abs() > 20 {
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

pub fn run_camera_thread(y: i32, x: i32, camera_maps_tx: Sender<DisplayMap>) {
    thread::spawn(move || {
        let mut old_map = DisplayMap::new();

        let cam = camera_capture::create(0).unwrap();
        let cam = cam.fps(30.0).unwrap().start().unwrap();
        for frame in cam {
            let frame = fit_frame_to_screen(frame, y, x);
            let difference_map = get_display_map(frame, &mut old_map);
            camera_maps_tx.send(difference_map).unwrap();
        }
    });
}
