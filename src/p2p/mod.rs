mod behaviour;
mod event;
mod main_loop;
mod relay;
mod transport;
mod utils;

use crate::commands::Guest;
use crate::commands::Room;
use crate::commands::ROOMS_ADDRESS;
use crate::p2p::behaviour::Behaviour;

use crate::types::ChannelsP2pEnd;
use crate::types::Message;
use futures::channel::mpsc::{UnboundedReceiver, UnboundedSender};

use futures::executor::block_on;
use futures::prelude::*;
use itertools::Itertools;
use libp2p::core::multiaddr::{Multiaddr, Protocol};
use libp2p::floodsub::Topic;

use libp2p::swarm::{SwarmBuilder, SwarmEvent};

use libp2p::{identity, PeerId};
use log::info;
use log::log_enabled;
use reqwest::blocking as reqwest;

use std::convert::TryInto;
use std::error::Error;

use std::net::Ipv4Addr;

use std::str::FromStr;

use crate::consts::RELAY_MULTIADDR;

use self::utils::get_local_peer_id;

pub struct P2p {
    relay_multiaddr: Multiaddr,
    pub peer_id: PeerId,
    key: identity::Keypair,
    channels: ChannelsP2pEnd,
    username: String,
    main_topic: Topic,
}

impl P2p {
    pub fn new(channels: ChannelsP2pEnd) -> Self {
        let (local_peer_id, key) = get_local_peer_id();

        if log_enabled!(log::Level::Debug) {
            block_on(
                channels
                    .in_p2p_sender
                    .clone()
                    .send(Message::Text(format!("My peer id is: {}", local_peer_id))),
            )
            .unwrap();
        }

        Self {
            relay_multiaddr: RELAY_MULTIADDR.parse().unwrap(),
            peer_id: local_peer_id,
            key,
            channels,
            username: local_peer_id.to_string().chars().rev().take(5).collect(),
            main_topic: Topic::new("main"),
        }
    }

    pub fn start(&mut self) -> Result<(), Box<dyn Error>> {
        let client = reqwest::Client::new();
        let rooms: Vec<Room> = client
            .get(format!("{}/rooms", ROOMS_ADDRESS))
            .send()?
            .json()?;

        self.message_on_debug("Got room list");

        let default_room = rooms.iter().find(|&it| it.name == "main").unwrap();

        let peer_ids_to_dial = default_room
            .guests
            .iter()
            .map(|guest| PeerId::from_str(&guest.multiaddr).ok())
            .filter(|it| it.is_some())
            .map(|it| it.unwrap())
            .unique()
            .filter(|&peer_id| peer_id != self.peer_id)
            .collect::<Vec<PeerId>>();

        let guest = Guest {
            id: None,
            name: self.username.clone(),
            multiaddr: self.peer_id.to_string(),
            room_id: default_room.id.unwrap(),
        };

        client
            .post(format!("{}/join", ROOMS_ADDRESS))
            .json(&guest)
            .send()?
            .text()
            .unwrap();

        self.message_on_debug("Got guest list");

        let (transport, client) =
            transport::create_transport(self.key.clone(), self.peer_id.clone());

        let behaviour = Behaviour::new(self.key.public(), client);

        let mut swarm = SwarmBuilder::new(transport, behaviour, self.peer_id)
            .dial_concurrency_factor(10_u8.try_into().unwrap())
            .build();

        swarm
            .listen_on(
                Multiaddr::empty()
                    .with("0.0.0.0".parse::<Ipv4Addr>().unwrap().into())
                    .with(Protocol::Tcp(0)),
            )
            .unwrap();

        block_on(async {
            let mut delay = futures_timer::Delay::new(std::time::Duration::from_secs(1)).fuse();
            loop {
                futures::select! {
                    event = swarm.next() => {
                        match event.unwrap() {
                            SwarmEvent::NewListenAddr { address, .. } => {
                                info!("Listening on {}", address);
                            },
                            event => panic!("{:?}", event),
                        }
                    }
                    _ = delay => {
                        break;
                    }
                }
            }
        });

        self.exchange_public_addresses_with_relay(&mut swarm);

        self.message_on_debug("Done negotiating with the relay");

        swarm
            .listen_on(self.relay_multiaddr.clone().with(Protocol::P2pCircuit))
            .unwrap();

        for peer_id in peer_ids_to_dial {
            swarm
                .dial(
                    self.relay_multiaddr
                        .clone()
                        .with(Protocol::P2pCircuit)
                        .with(Protocol::P2p(peer_id.into())),
                )
                .unwrap();
            if log_enabled!(log::Level::Debug) {
                block_on(
                    self.channels
                        .in_p2p_sender
                        .clone()
                        .send(Message::Text(format!("Dialing {}", peer_id.to_string()))),
                )
                .unwrap();
            }
        }

        self.run_swarm_loop(&mut swarm);

        Ok(())
    }
}
