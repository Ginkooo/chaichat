use crate::types::{CameraFrame, Message};
use camera_capture::Frame;
use image::imageops::resize;
use image::{FilterType, ImageBuffer, Rgb, RgbImage};
use std::sync::mpsc::Sender;
use std::thread;

fn get_camera_frame(
    frame: ImageBuffer<Rgb<u8>, Vec<u8>>,
    old_frame: &mut CameraFrame,
) -> CameraFrame {
    let mut difference_frame = CameraFrame::default();
    let mut all_frame = CameraFrame::default();
    for (x, y, rgb) in frame.enumerate_pixels() {
        all_frame.pixels.push(((x as u16, y as u16), rgb.data));
    }
    if old_frame.pixels.is_empty() {
        *old_frame = all_frame.clone();
        return all_frame;
    }
    for (old_px, new_px) in old_frame.pixels.iter().zip(all_frame.pixels.iter()) {
        let old_sum: i32 = old_px.1.iter().map(|&x| x as i32).sum();
        let new_sum: i32 = new_px.1.iter().map(|&x| x as i32).sum();
        let diff: i32 = old_sum - new_sum;
        if diff.abs() > 20 {
            difference_frame
                .pixels
                .push(((new_px.0 .0, new_px.0 .1), new_px.1));
        }
    }
    *old_frame = all_frame;
    difference_frame
}

fn fit_frame_to_screen(
    frame: ImageBuffer<Rgb<u8>, Frame>,
    y: u16,
    x: u16,
) -> ImageBuffer<Rgb<u8>, Vec<u8>> {
    let frame = RgbImage::from_raw(frame.width(), frame.height(), frame.to_vec())
        .expect("failed to create image from camera frame");
    resize(&frame, x as u32, y as u32, FilterType::Nearest)
}

pub fn run_camera_thread(y: u16, x: u16, camera_frames_tx: Sender<Message>) {
    thread::spawn(move || {
        let mut old_frame = CameraFrame::default();

        let cam = camera_capture::create(0).expect("failed to create camera handle");
        let cam = cam
            .fps(30.0)
            .expect("failed to set FPS mode on camera")
            .start()
            .expect("failed to start camera");
        for frame in cam {
            let frame = fit_frame_to_screen(frame, y, x);
            let camera_frame = get_camera_frame(frame, &mut old_frame);
            camera_frames_tx
                .send(Message::CameraFrame(camera_frame))
                .expect("failed to send camera map to channel");
        }
    });
}
