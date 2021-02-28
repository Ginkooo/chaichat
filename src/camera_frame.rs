use crate::types::{Pixels, CameraImage};

use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};
use image::imageops::resize;
use image::FilterType;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CameraFrame {
    pub resolution: Option<(u16, u16)>,
    pub pixels: Pixels,
    pub created: DateTime<Utc>,
}

impl Default for CameraFrame {
    fn default() -> Self {
        Self {
            created: Utc::now(),
            resolution: None,
            pixels: Pixels::new(),
        }
    }
}

impl CameraFrame {
    pub fn from_camera_image(camera_image: CameraImage, width: u16, height: u16, old_frame: &mut Self) -> Self {
        let image = resize(&camera_image, width as u32, height as u32, FilterType::Nearest);
        Self::get_pixels_from_camera_image(&image, old_frame)
    }

    fn get_pixels_from_camera_image(camera_image: &CameraImage, old_frame: &mut Self) -> Self {
        let mut difference_frame = Self::default();
        let mut all_frame = Self::default();
        for (x, y, rgb) in camera_image.enumerate_pixels() {
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
}
