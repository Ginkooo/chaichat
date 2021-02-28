use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::clone::Clone;
use std::collections::HashMap;

pub type Pixels = Vec<((u16, u16), [u8; 3])>;
pub type Buffer = HashMap<(u16, u16), [u8; 3]>;


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

#[derive(Serialize, Deserialize)]
pub enum Message {
    End,
    CameraFrame(CameraFrame),
}
