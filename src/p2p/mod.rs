mod behaviour;
mod event;
mod transport;

use crate::p2p::behaviour::Behaviour;
use crate::p2p::event::Event;
use futures::executor::block_on;
use futures::future::FutureExt;
use futures::stream::StreamExt;
use libp2p::core::multiaddr::{Multiaddr, Protocol};
use libp2p::identify::{IdentifyEvent, IdentifyInfo};
use libp2p::relay::v2::client;
use libp2p::swarm::{SwarmBuilder, SwarmEvent};
use libp2p::Swarm;
use libp2p::{identity, PeerId};
use log::info;
use std::convert::TryInto;
use std::error::Error;
use std::net::Ipv4Addr;

use crate::consts::RELAY_MULTIADDR;

pub struct P2p {
    relay_multiaddr: Multiaddr,
    peer_id: PeerId,
    key: identity::Keypair,
}

impl P2p {
    pub fn new() -> Self {
        let local_key = identity::Keypair::generate_ed25519();
        let local_peer_id = PeerId::from(local_key.public());

        Self {
            relay_multiaddr: RELAY_MULTIADDR.parse().unwrap(),
            peer_id: local_peer_id,
            key: local_key,
        }
    }

    pub fn start(&self) -> Result<(), Box<dyn Error>> {
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

        swarm
            .listen_on(self.relay_multiaddr.clone().with(Protocol::P2pCircuit))
            .unwrap();

        swarm
            .dial(
                self.relay_multiaddr
                    .clone()
                    .with(Protocol::P2pCircuit)
                    .with(Protocol::P2p(self.peer_id.into())),
            )
            .unwrap();

        self.run_swarm_loop(&mut swarm);

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

    fn run_swarm_loop(&self, swarm: &mut Swarm<Behaviour>) {
        block_on(async {
            loop {
                match swarm.next().await.unwrap() {
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
                    SwarmEvent::Behaviour(Event::Ping(_)) => {}
                    SwarmEvent::ConnectionEstablished {
                        peer_id, endpoint, ..
                    } => {
                        info!("Established connection to {:?} via {:?}", peer_id, endpoint);
                    }
                    SwarmEvent::OutgoingConnectionError { peer_id, error } => {
                        info!("Outgoing connection error to {:?}: {:?}", peer_id, error);
                    }
                    _ => {}
                }
            }
        });
    }
}
