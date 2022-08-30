use futures::channel::mpsc::{unbounded, UnboundedReceiver};
use image::RgbImage;
use nokhwa::{Camera, CameraFormat, FrameFormat};
use std::thread;

use crate::camera_frame::CameraFrame;
use crate::consts::DEFAULT_CAMERA_SIZE;
use crate::types::{CameraImage, Res};

pub fn run_camera_thread() -> Res<UnboundedReceiver<CameraFrame>> {
    let (sender, receiver) = unbounded();

    thread::spawn(move || {
        let cam = Camera::new(
            0,
            Some(CameraFormat::new_from(
                DEFAULT_CAMERA_SIZE[0] as u32,
                DEFAULT_CAMERA_SIZE[1] as u32,
                FrameFormat::MJPEG,
                30,
            )),
        );
        if cam.as_ref().is_err() {
            return;
        }
        let mut cam = cam.unwrap();
        cam.open_stream().unwrap();
        loop {
            let frame = cam.frame().unwrap();
            let frame = RgbImage::from_raw(frame.width(), frame.height(), frame.to_vec())
                .expect("failed to create image from camera frame");
            let frame = CameraImage::from(frame);
            let frame = CameraFrame::from_camera_image(frame);
            sender.unbounded_send(frame);
        }
    });

    Ok(receiver)
}
