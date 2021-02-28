use crate::types::CameraImage;
use camera_capture;
use image::RgbImage;
use std::sync::mpsc::Sender;
use std::thread;

pub fn run_camera_thread(y: u16, x: u16, raw_camera_images_tx: Sender<CameraImage>) {
    thread::spawn(move || {
        let cam = camera_capture::create(0).expect("failed to create camera handle");
        let cam = cam
            .fps(30.0)
            .expect("failed to set FPS mode on camera")
            .start()
            .expect("failed to start camera");
        for frame in cam {
            let frame = RgbImage::from_raw(frame.width(), frame.height(), frame.to_vec())
                .expect("failed to create image from camera frame");
            raw_camera_images_tx.send(frame).unwrap();
        }
    });
}
