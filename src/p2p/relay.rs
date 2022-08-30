use futures::executor::block_on;
use futures::prelude::*;
use libp2p::swarm::SwarmEvent;
use libp2p::{
    identify::{IdentifyEvent, IdentifyInfo},
    Swarm,
};
use log::info;

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
                    SwarmEvent::Dialing { .. } => {
                        self.message_on_debug("Dialing the relay");
                    }
                    SwarmEvent::ConnectionEstablished { .. } => {
                        self.message_on_debug("Established connection with the relay");
                    }
                    SwarmEvent::Behaviour(Event::Ping(_)) => {}
                    SwarmEvent::Behaviour(Event::Identify(IdentifyEvent::Sent { .. })) => {
                        info!("Told relay its public address.");
                        self.message_on_debug("Told relay its public address");
                        told_relay_observed_addr = true;
                    }
                    SwarmEvent::Behaviour(Event::Identify(IdentifyEvent::Received {
                        info: IdentifyInfo { observed_addr, .. },
                        ..
                    })) => {
                        self.message_on_debug("Relay told us our public address");
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

        self.message_on_debug("Exchanged address with relay");
    }
}
