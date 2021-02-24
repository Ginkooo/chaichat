use std::sync::mpsc::{Sender, Receiver};
use types::DisplayMap;
use std::net::UdpSocket;


const DISPLAY_MAP_SZ: usize = 130568;
const LOCAL_IP: &str = "127.0.0.1";
const REMOTE_IP: &str = "217.182.75.11";


pub fn get_remote_frames(port: String, received_maps_tx: Sender<DisplayMap>) {
    let socket = UdpSocket::bind(format!("0.0.0.0:{}", port)).unwrap();

    loop {
        let mut arr = vec![0u8; DISPLAY_MAP_SZ];

        socket.recv(&mut arr[..]).unwrap();

        if arr.is_empty() {continue;}
        match bincode::deserialize(&arr[..]) {
            Ok(display_map) => {
                received_maps_tx.send(display_map).unwrap();
            },
            Err(_) => {
                println!("Error recieving frame");
                continue;
            }
        };
    }
}

pub fn send_remote_frames(port: String, rx: Receiver<DisplayMap>) {
    let socket = UdpSocket::bind("0.0.0.0:9797").unwrap();
    for display_map in rx {
        let chunks = display_map.chunks(5300);
        for chunk in chunks {
            let encoded = bincode::serialize(&chunk).unwrap();
            socket.send_to(&encoded[..], format!("{}:{}", LOCAL_IP, port)).unwrap();
        }
    }
}
