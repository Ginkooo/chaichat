use crossterm::event::Event;
use futures::channel::mpsc::{UnboundedReceiver, UnboundedSender};
use image::{ImageBuffer, Rgb};
use serde::{Deserialize, Serialize};
use std::error::Error;

use crate::camera_frame::CameraFrame;

pub type Res<T> = Result<T, Box<dyn Error + Send + Sync + 'static>>;

pub type Pixels = Vec<((u16, u16), [u8; 3])>;

pub type CameraImage = ImageBuffer<Rgb<u8>, Vec<u8>>;

#[derive(Serialize, Deserialize, Debug)]
pub struct UserMessage {
    pub username: Option<String>,
    pub text: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Message {
    Empty,
    RawCameraImage(Vec<u8>),
    Text(String),
    UserMessage(UserMessage),
}

#[derive(thiserror::Error, Debug)]
pub enum ChaiError {
    #[error("User caused exit")]
    EscClicked(String),
}

pub struct ChannelsTerminalEnd<'a> {
    pub receiver_camera: &'a mut UnboundedReceiver<CameraFrame>,
    pub input_event_receiver: &'a mut UnboundedReceiver<Event>,
    pub in_p2p_receiver: &'a mut UnboundedReceiver<Message>,
    pub out_p2p_sender: UnboundedSender<Message>,
}

pub struct ChannelsP2pEnd {
    pub in_p2p_sender: UnboundedSender<Message>,
    pub out_p2p_receiver: UnboundedReceiver<Message>,
}
