use image::{ImageBuffer, Rgb};
use std::error::Error;

pub type Res<T> = Result<T, Box<dyn Error + Send + Sync + 'static>>;

pub type Pixels = Vec<((u16, u16), [u8; 3])>;
pub type CameraImage = ImageBuffer<Rgb<u8>, Vec<u8>>;
