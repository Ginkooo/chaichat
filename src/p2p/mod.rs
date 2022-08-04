mod behaviour;
mod event;
mod transport;

use crate::p2p::behaviour::Behaviour;
use crate::p2p::event::Event;
use async_std::{io, task};
use dirs;
use futures::executor::block_on;
use futures::future::FutureExt;
use futures::AsyncReadExt;
use futures::{
    prelude::{stream::StreamExt, *},
    select,
};
use libp2p::core::multiaddr::{Multiaddr, Protocol};
use libp2p::floodsub;
use libp2p::identify::{IdentifyEvent, IdentifyInfo};
use libp2p::relay::v2::client;
use libp2p::swarm::{SwarmBuilder, SwarmEvent};
use libp2p::Swarm;
use libp2p::{identity, PeerId};
use log::info;
use std::convert::TryInto;
use std::error::Error;
use std::fs::File;
use std::io::Read;
use std::io::Write;
use std::net::Ipv4Addr;
use std::str::FromStr;

use crate::consts::RELAY_MULTIADDR;

pub struct P2p {
    relay_multiaddr: Multiaddr,
    peer_id: PeerId,
    key: identity::Keypair,
}

impl P2p {
    pub fn new() -> Self {
        let mut local_key = identity::Keypair::generate_ed25519();
        let mut path = dirs::config_dir().unwrap();
        path.push("chaichar_private_key");

        match File::open(&path) {
            Ok(mut file) => {
                let mut v: Vec<u8> = Vec::new();
                file.read_to_end(&mut v).unwrap();
                local_key = identity::Keypair::from_protobuf_encoding(&v[..]).unwrap();
            }
            Err(_) => {}
        };

        let encoded = local_key.to_protobuf_encoding().unwrap();

        File::create(&path).unwrap().write(&encoded).unwrap();

        let local_peer_id = PeerId::from(local_key.public());

        dbg!(&local_peer_id);

        Self {
            relay_multiaddr: RELAY_MULTIADDR.parse().unwrap(),
            peer_id: local_peer_id,
            key: local_key,
        }
    }

    pub fn start(&self) -> Result<(), Box<dyn Error>> {
        let peer_ids_to_dial = [
            PeerId::from_str("12D3KooWRmxptk9mVYWu69nrDjJtSdRwsqCFLpGiWapJZTZVCuMr").unwrap(),
            PeerId::from_str("12D3KooWJiXZEyXJDoh1FDuCuep2xdRDcm26asX743a9Yv1R3kQU").unwrap(),
        ];

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
        }

        self.run_swarm_loop(&mut swarm, main_topic.clone());

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
        let mut stdin = io::BufReader::new(io::stdin()).lines().fuse();
        block_on(async {
            loop {
                select!(
                    line = stdin.select_next_some() => {
                        swarm.behaviour_mut().floodsub.publish(topic.clone(), line.unwrap().as_bytes());
                    }
                    event = swarm.select_next_some() => match event {
                        SwarmEvent::NewListenAddr { address, .. } => {
                            info!("Listening on {:?}", address);
                        }
                        SwarmEvent::Behaviour(Event::Relay(
                            client::Event::ReservationReqAccepted { .. },
                        )) => {
                            info!("Relay accepted our reservation request.");
                        }
                        SwarmEvent::Behaviour(Event::Relay(event)) => {
                            info!("{:?}", event)
                        }
                        SwarmEvent::Behaviour(Event::Dcutr(event)) => {
                            info!("{:?}", event)
                        }
                        SwarmEvent::Behaviour(Event::Identify(event)) => {
                            info!("{:?}", event)
                        }
                        SwarmEvent::Behaviour(Event::Floodsub(event)) => {
                            info!("{:?}", event);
                        }
                        SwarmEvent::Behaviour(Event::Ping(event)) => {
                        dbg!(event);
                        }
                        SwarmEvent::ConnectionEstablished {
                            peer_id, endpoint, ..
                        } => {
                            info!("Established connection to {:?} via {:?}", peer_id, endpoint);
                            swarm.behaviour_mut().floodsub.add_node_to_partial_view(peer_id);
                        }
                        SwarmEvent::OutgoingConnectionError { peer_id, error } => {
                            info!("Outgoing connection error to {:?}: {:?}", peer_id, error);
                        }
                        _ => {}
                    }
                );
            }
        });
    }
}
