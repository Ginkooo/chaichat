use crossbeam::channel::{unbounded, Receiver};
use image::RgbImage;
use nokhwa::{Camera, CameraFormat, FrameFormat};
use std::thread;

use crate::camera_frame::CameraFrame;
use crate::types::{CameraImage, Res};

pub fn run_camera_thread() -> Res<Receiver<CameraFrame>> {
    let (sender, receiver) = unbounded();
    let sender_clone = sender.clone();

    thread::spawn(move || {
        let mut cam = Camera::new(
            0,
            Some(CameraFormat::new_from(640, 480, FrameFormat::MJPEG, 30)),
        )
        .expect("Cannot launch camera");
        cam.open_stream().unwrap();
        loop {
            let frame = cam.frame().unwrap();
            let frame = RgbImage::from_raw(frame.width(), frame.height(), frame.to_vec())
                .expect("failed to create image from camera frame");
            let frame = CameraImage::from(frame);
            let frame = CameraFrame::from_camera_image(frame);
            sender_clone.send(frame).ok();
        }
    });

    Ok(receiver)
}
