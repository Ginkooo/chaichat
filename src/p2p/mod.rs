mod behaviour;
mod event;
mod transport;

use crate::commands::Guest;
use crate::commands::Room;
use crate::commands::ROOMS_ADDRESS;
use crate::p2p::behaviour::Behaviour;
use crate::p2p::event::Event;
use crate::types::Message;
use async_std::channel::{Receiver, Sender};
use bincode;
use dirs;
use futures::executor::block_on;
use futures::future::FutureExt;
use futures::{
    prelude::{stream::StreamExt, *},
    select,
};
use itertools::Itertools;
use libp2p::core::multiaddr::{Multiaddr, Protocol};
use libp2p::floodsub::{self, FloodsubEvent};
use libp2p::identify::{IdentifyEvent, IdentifyInfo};
use libp2p::relay::v2::client;
use libp2p::swarm::{SwarmBuilder, SwarmEvent};
use libp2p::Swarm;
use libp2p::{identity, PeerId};
use log::info;
use reqwest::blocking as reqwest;
use std::collections::HashMap;
use std::convert::TryInto;
use std::error::Error;
use std::fs::File;
use std::io::Read;
use std::io::Write;
use std::net::Ipv4Addr;
use std::path::PathBuf;
use std::str::FromStr;

use crate::consts::RELAY_MULTIADDR;

pub struct P2p {
    relay_multiaddr: Multiaddr,
    pub peer_id: PeerId,
    key: identity::Keypair,
    in_sender: Sender<Message>,
    out_receiver: Receiver<Message>,
    username: String,
}

impl P2p {
    pub fn new(in_sender: Sender<Message>, out_receiver: Receiver<Message>) -> Self {
        let mut local_key = identity::Keypair::generate_ed25519();
        let mut path = dirs::config_dir().unwrap();
        let mut path_clone = path.clone();
        path_clone.push("temporary_chaichat");
        match File::create(&path_clone) {
            Ok(_) => {
            }
            Err(_) => {
                path = PathBuf::from(".")
            }
        }
        std::fs::remove_file(&path_clone).ok();

        path.push("chaichat_local_key");

        match File::open(&path) {
            Ok(mut file) => {
                let mut v: Vec<u8> = Vec::new();
                file.read_to_end(&mut v).unwrap();
                local_key = identity::Keypair::from_protobuf_encoding(&v[..]).unwrap_or(local_key);
            }
            Err(_) => {}
        };

        let encoded = local_key.to_protobuf_encoding().unwrap();

        let mut file = File::create(&path).unwrap();
        file.write(&encoded).unwrap();

        let local_peer_id = PeerId::from(local_key.public());

        block_on(
            in_sender
                .clone()
                .send(Message::Text(format!("My peer id is: {}", local_peer_id))),
        )
        .unwrap();

        Self {
            relay_multiaddr: RELAY_MULTIADDR.parse().unwrap(),
            peer_id: local_peer_id,
            key: local_key,
            in_sender,
            out_receiver,
            username: local_peer_id.to_string().chars().rev().take(5).collect(),
        }
    }

    pub fn start(&self) -> Result<(), Box<dyn Error>> {
        let client = reqwest::Client::new();
        let rooms: Vec<Room> = client
            .get(format!("{}/rooms", ROOMS_ADDRESS))
            .send()?
            .json()?;

        let default_room = rooms.iter().find(|&it| it.name == "main").unwrap();

        let peer_ids_to_dial = default_room
            .guests
            .iter()
            .map(|guest| PeerId::from_str(&guest.multiaddr).ok())
            .filter(|it| it.is_some())
            .map(|it| it.unwrap())
            .unique().filter(|&peer_id| peer_id != self.peer_id)
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
            .text().unwrap();

        let (transport, client) =
            transport::create_transport(self.key.clone(), self.peer_id.clone());

        let main_topic = floodsub::Topic::new("main");

        let mut behaviour = Behaviour::new(self.key.public(), client);

        behaviour.floodsub.subscribe(main_topic.clone());

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

        block_on(
            self.in_sender
                .clone()
                .send(Message::Text("Exchanged addresses with relay".to_string())),
        )
        .unwrap();

        swarm
            .listen_on(self.relay_multiaddr.clone().with(Protocol::P2pCircuit))
            .unwrap();

        // for peer_id in peer_ids_to_dial {
        //     swarm
        //         .dial(
        //             self.relay_multiaddr
        //                 .clone()
        //                 .with(Protocol::P2pCircuit)
        //                 .with(Protocol::P2p(peer_id.into())),
        //         )
        //         .unwrap();
        //     block_on(
        //         self.in_sender
        //             .clone()
        //             .send(Message::Text(format!("Dialing {}", peer_id.to_string()))),
        //     )
        //     .unwrap();
        // }

        self.run_swarm_loop(&mut swarm, main_topic);

        Ok(())
    }

    fn exchange_public_addresses_with_relay(&self, swarm: &mut Swarm<Behaviour>) {
        swarm.dial(self.relay_multiaddr.clone()).unwrap();
        block_on(async {
            let mut learned_observed_addr = false;
            let mut told_relay_observed_addr = false;

            loop {
                match swarm.next().await.unwrap() {
                    SwarmEvent::NewListenAddr { .. } => {}
                    SwarmEvent::Dialing { .. } => {}
                    SwarmEvent::ConnectionEstablished { .. } => {}
                    SwarmEvent::Behaviour(Event::Ping(_)) => {}
                    SwarmEvent::Behaviour(Event::Identify(IdentifyEvent::Sent { .. })) => {
                        info!("Told relay its public address.");
                        told_relay_observed_addr = true;
                    }
                    SwarmEvent::Behaviour(Event::Identify(IdentifyEvent::Received {
                        info: IdentifyInfo { observed_addr, .. },
                        ..
                    })) => {
                        info!("Relay told us our public address: {:?}", observed_addr);
                        learned_observed_addr = true;
                    }
                    event => panic!("{:?}", event),
                }

                if learned_observed_addr && told_relay_observed_addr {
                    break;
                }
            }
        });
    }

    fn run_swarm_loop(&self, swarm: &mut Swarm<Behaviour>, topic: floodsub::Topic) {
        let mut out_receiver = self.out_receiver.clone();
        let in_sender = self.in_sender.clone();
        block_on(async {
            loop {
                select!(
                    event = swarm.select_next_some() => match event {
                        SwarmEvent::NewListenAddr { address, .. } => {
                            info!("Listening on {:?}", address);
                        }
                        SwarmEvent::Behaviour(Event::Relay(
                            client::Event::ReservationReqAccepted { .. },
                        )) => {
                            in_sender.send(Message::Text("Relay accepted out reservation request".to_string())).await.unwrap();
                            info!("Relay accepted our reservation request.");
                        }
                        SwarmEvent::Behaviour(Event::Relay(event)) => {
                            in_sender.send(Message::Text(format!("{:?}", event))).await.unwrap();;
                            info!("{:?}", event)
                        }
                        SwarmEvent::Behaviour(Event::Dcutr(event)) => {
                            info!("dcutr: {:?}", event);
                            dbg!(event);
                        }
                        SwarmEvent::Behaviour(Event::Identify(event)) => {
                            info!("{:?}", event)
                        }
                        SwarmEvent::Behaviour(Event::Floodsub(FloodsubEvent::Message(msg))) => {
                            let message = bincode::deserialize::<Message>(&msg.data).unwrap_or(Message::Empty);
                            in_sender.send(message).await.unwrap();
                        }
                        SwarmEvent::Behaviour(Event::Ping(_event)) => {
                        }
                        SwarmEvent::ConnectionEstablished {
                            peer_id, endpoint, ..
                        } => {
                            in_sender.send(Message::Text(format!("{} connected!", peer_id))).await.unwrap();
                            swarm.behaviour_mut().floodsub.add_node_to_partial_view(peer_id);
                        }
                        SwarmEvent::OutgoingConnectionError { peer_id, error } => {
                            in_sender.send(Message::Text(format!("{} disconnected! ({})", match peer_id {
                                Some(peer_id) => {
                                    // swarm.behaviour_mut().floodsub.remove_node_from_partial_view(&peer_id);
                                    peer_id.to_string()
                                }
                                None => "somebody".to_string()
                            }, error))).await.unwrap();
                        }
                        _ => {}
                    },
                    message = out_receiver.next() => match message {
                        None => {},
                        Some(msg) => {
                            let encoded = bincode::serialize(&msg).unwrap();
                            swarm.behaviour_mut().floodsub.publish(topic.clone(), encoded);
                        }
                    }
                );
            }
        });
    }
}
