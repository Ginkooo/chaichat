use serde::{Deserialize, Serialize};
use image::{ImageBuffer, Rgb};
use std::collections::HashMap;
use crate::camera_frame::CameraFrame;

pub type Pixels = Vec<((u16, u16), [u8; 3])>;
pub type Buffer = HashMap<(u16, u16), [u8; 3]>;
pub type CameraImage = ImageBuffer<Rgb<u8>, Vec<u8>>;



#[derive(Serialize, Deserialize)]
pub enum Message {
    End,
    CameraFrame(CameraFrame),
}
