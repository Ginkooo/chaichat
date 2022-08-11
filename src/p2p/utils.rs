use std::io::prelude::*;
use std::{fs::File, path::PathBuf};

use futures::executor::block_on;
use libp2p::{identity::Keypair, PeerId};
use log::log_enabled;

use crate::types::Message;

use super::P2p;

impl P2p {
    pub fn message_on_debug(&self, text: &str) {
        if log_enabled!(log::Level::Debug) {
            block_on(self.in_sender.send(Message::Text(text.to_string()))).unwrap()
        }
    }
}

pub fn get_local_peer_id() -> (PeerId, Keypair) {
    let mut local_key = Keypair::generate_ed25519();
    let mut path = dirs::config_dir().unwrap();
    let mut path_clone = path.clone();
    path_clone.push("temporary_chaichat");
    match File::create(&path_clone) {
        Ok(_) => {}
        Err(_) => path = PathBuf::from("."),
    }
    std::fs::remove_file(&path_clone).ok();

    path.push("chaichat_local_key");

    match File::open(&path) {
        Ok(mut file) => {
            let mut v: Vec<u8> = Vec::new();
            file.read_to_end(&mut v).unwrap();
            local_key = Keypair::from_protobuf_encoding(&v[..]).unwrap_or(local_key);
        }
        Err(_) => {}
    };

    let encoded = local_key.to_protobuf_encoding().unwrap();

    let mut file = File::create(&path).unwrap();
    file.write(&encoded).unwrap();

    (PeerId::from(local_key.public()), local_key)
}

pub fn get_username_from_peer_id(peer_id: PeerId) -> String {
    peer_id.to_string().chars().rev().take(5).collect()
}
