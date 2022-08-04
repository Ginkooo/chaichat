use crate::types::{CameraImage, Pixels};

use chrono::{DateTime, Utc};
use image::imageops::{resize, FilterType};

#[derive(Clone)]
pub struct CameraFrame {
    pub resolution: (u16, u16),
    pub created: DateTime<Utc>,
    prev_pixels: Pixels,
    pub camera_image: CameraImage,
}

impl CameraFrame {
    pub fn from_camera_image(camera_image: CameraImage) -> Self {
        Self {
            created: Utc::now(),
            resolution: (camera_image.width() as u16, camera_image.height() as u16),
            prev_pixels: Pixels::new(),
            camera_image,
        }
    }

    pub fn resize(self: Self, width: u16, height: u16) -> Self {
        let camera_image = resize(
            &self.camera_image,
            width as u32,
            height as u32,
            FilterType::Lanczos3,
        );
        Self {
            created: self.created,
            resolution: (width, height),
            prev_pixels: Pixels::new(),
            camera_image,
        }
    }

    pub fn get_pixels(self: &mut Self) -> Pixels {
        let mut pixels = Pixels::new();
        let mut difference_pixels = Pixels::new();
        for (x, y, rgb) in self.camera_image.enumerate_pixels() {
            pixels.push(((x as u16, y as u16), rgb.0));
        }
        if self.prev_pixels.is_empty() {
            self.prev_pixels = pixels.clone();
            return pixels;
        }
        for (old_px, new_px) in self.prev_pixels.iter().zip(pixels.iter()) {
            let old_sum: i32 = old_px.1.iter().map(|&x| x as i32).sum();
            let new_sum: i32 = new_px.1.iter().map(|&x| x as i32).sum();
            let diff: i32 = old_sum - new_sum;
            if diff.abs() > 10 {
                difference_pixels.push(((new_px.0 .0, new_px.0 .1), new_px.1));
            }
        }
        self.prev_pixels = pixels;
        difference_pixels
    }
}
