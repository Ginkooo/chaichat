use futures::executor::block_on;
use futures::prelude::*;
use libp2p::{
    identify::{IdentifyEvent, IdentifyInfo},
    Swarm,
};
use libp2p_swarm::SwarmEvent;
use log::{info, log_enabled};

use crate::types::Message;

use super::{behaviour::Behaviour, event::Event, P2p};

impl P2p {
    pub fn exchange_public_addresses_with_relay(&self, swarm: &mut Swarm<Behaviour>) {
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

        if log_enabled!(log::Level::Debug) {
            block_on(
                self.in_sender
                    .clone()
                    .send(Message::Text("Exchanged addresses with relay".to_string())),
            )
            .unwrap();
        }
    }
}
