use crate::types::Message;
use async_std::channel::Sender;
use futures::executor::block_on;
use serde::{Deserialize, Serialize};

use reqwest::blocking as reqwest;

const ROOMS_ADDRESS: &str = "http://chaicorp.pl:8000";

#[derive(Serialize, Deserialize)]
struct Guest {
    id: Option<i32>,
    name: String,
    multiaddr: String,
    room_id: i32,
}

#[derive(Serialize, Deserialize)]
struct Room {
    id: Option<i32>,
    name: String,
    guests: Vec<Guest>,
}

pub fn handle_command(string: &String, out_sender: Sender<Message>) -> String {
    let client = reqwest::Client::new();
    if !string.starts_with("/") {
        let msg = Message::Text(string.clone());
        block_on(out_sender.send(msg)).unwrap();
        return string.clone();
    }
    let command = &string[1..];
    let command: Vec<&str> = command.split(" ").collect();
    let argumens = command.get(1..).unwrap_or(&[""]);
    let command = command[0];

    match command {
        "list" => client
            .get(format!("{}/rooms", ROOMS_ADDRESS))
            .send()
            .unwrap()
            .text()
            .unwrap(),
        "add" => {
            let room = Room {
                id: None,
                name: String::from(argumens[0]),
                guests: vec![],
            };
            client
                .post(format!("{}/rooms", ROOMS_ADDRESS))
                .json(&room)
                .send()
                .unwrap()
                .text()
                .unwrap()
        }
        "join" => {
            let guest = Guest {
                id: None,
                name: String::from(argumens[1]),
                multiaddr: String::from(""),
                room_id: argumens[0].parse().unwrap(),
            };

            client
                .post(format!("{}/join", ROOMS_ADDRESS))
                .json(&guest)
                .send()
                .unwrap()
                .text()
                .unwrap()
        }
        _ => String::new(),
    }
}
