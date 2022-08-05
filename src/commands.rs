use crate::types::Message;
use async_std::channel::Sender;
use futures::executor::block_on;
use serde::{Deserialize, Serialize};

use crate::types::Res;
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

pub fn handle_command(string: &String, out_sender: Sender<Message>) -> Res<String> {
    let client = reqwest::Client::new();
    if !string.starts_with("/") {
        let msg = Message::Text(string.clone());
        block_on(out_sender.send(msg)).unwrap();
        return Ok(String::new());
    }
    let command = &string[1..];
    let command: Vec<&str> = command.split(" ").collect();
    let argumens = command.get(1..).unwrap_or(&[""]);
    let command = command[0];

    match command {
        "list" => Ok(client
            .get(format!("{}/rooms", ROOMS_ADDRESS))
            .send()?
            .text()?),
        "add" => {
            let room = Room {
                id: None,
                name: String::from(argumens[0]),
                guests: vec![],
            };
            Ok(client
                .post(format!("{}/rooms", ROOMS_ADDRESS))
                .json(&room)
                .send()?
                .text()?)
        }
        "join" => {
            let guest = Guest {
                id: None,
                name: argumens.get(1).ok_or("")?.to_string(),
                multiaddr: String::from(""),
                room_id: argumens.get(0).ok_or("")?.parse()?,
            };

            Ok(client
                .post(format!("{}/join", ROOMS_ADDRESS))
                .json(&guest)
                .send()?
                .text()?)
        }
        _ => Ok(String::new()),
    }
}
