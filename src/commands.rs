use crate::types::{Message, UserMessage};
use futures::channel::mpsc::UnboundedSender;

use futures::prelude::*;
use serde::{Deserialize, Serialize};

use crate::types::Res;
use reqwest::blocking as reqwest;

pub const ROOMS_ADDRESS: &str = "http://chaicorp.pl:8000";

#[derive(Serialize, Deserialize, Debug)]
pub struct Guest {
    pub id: Option<i32>,
    pub name: String,
    pub multiaddr: String,
    pub room_id: i32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Room {
    pub id: Option<i32>,
    pub name: String,
    pub guests: Vec<Guest>,
}

pub fn handle_command(string: &String, out_sender: UnboundedSender<Message>) -> Res<String> {
    let client = reqwest::Client::new();
    if !string.starts_with("/") {
        let msg = Message::UserMessage(UserMessage {
            username: None,
            text: string.clone(),
        });
        out_sender.unbounded_send(msg).unwrap();
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
