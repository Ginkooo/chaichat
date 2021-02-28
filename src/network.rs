use crate::types::Message;
use std::net::UdpSocket;
use std::sync::mpsc::{Receiver, Sender};
use crate::camera_frame::CameraFrame;

const DISPLAY_MAP_SZ: usize = 200568;
const LOCAL_IP: &str = "127.0.0.1";
const REMOTE_IP: &str = "217.182.75.11";

pub fn get_remote_frames(port: String, received_messages_tx: Sender<Message>) {
    let socket =
        UdpSocket::bind(format!("0.0.0.0:{}", port)).expect("failed to bind receiving udp socket");

    loop {
        let mut arr = vec![0u8; DISPLAY_MAP_SZ];

        socket
            .recv(&mut arr[..])
            .expect("failed to receive some bytes via udp");

        if arr.is_empty() {
            continue;
        }
        match bincode::deserialize(&arr[..]) {
            Ok(message) => {
                match received_messages_tx.send(message) {
                    Ok(_) => {}
                    Err(_) => break
                };
            }
            Err(_) => {
                eprintln!("Error recieving frame");
                continue;
            }
        };
    }
}

pub fn send_remote_frames(port: String, rx: Receiver<Message>) {
    let socket = UdpSocket::bind("0.0.0.0:9797").expect("faled to bind sending udp socket");
    for message in rx {
        match message {
            Message::End => {
                break;
            },
            Message::CameraFrame(camera_frame) => {
                let chunks = camera_frame.pixels.chunks(5300);
                for chunk in chunks {
                    let mut camera_frame = CameraFrame::default();
                    camera_frame.pixels = chunk.to_vec();
                    let message = Message::CameraFrame(camera_frame);
                    let encoded = bincode::serialize(&message)
                        .expect("failed to deserialize bytes into CameraFrame instance");
                    socket
                        .send_to(&encoded[..], format!("{}:{}", LOCAL_IP, port))
                        .expect("failed to send bytes via udp");
                }
            }
        }
    }
}
