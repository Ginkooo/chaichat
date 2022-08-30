use futures::{executor::block_on, prelude::*, select};
use libp2p::dcutr::behaviour::Event::DirectConnectionUpgradeSucceeded;
use libp2p::swarm::SwarmEvent;
use libp2p::{floodsub::FloodsubEvent, relay::v2::client, Swarm};
use log::{info, log_enabled, Level};

use crate::{
    p2p::{event::Event, utils, Behaviour, P2p},
    types::Message,
};

impl P2p {
    pub fn run_swarm_loop(&mut self, swarm: &mut Swarm<Behaviour>) {
        if log_enabled!(Level::Debug) {
            block_on(
                self.channels
                    .in_p2p_sender
                    .send(Message::Text("Running swarm loop".to_string())),
            )
            .unwrap();
        }
        swarm
            .behaviour_mut()
            .floodsub
            .subscribe(self.main_topic.clone());
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
                            if log_enabled!(Level::Debug) {
                                self.channels.in_p2p_sender.send(Message::Text("Relay accepted our reservation request".to_string())).await.unwrap();
                            }
                            info!("Relay accepted our reservation request.");
                        }
                        SwarmEvent::Behaviour(Event::Relay(event)) => {
                            info!("{:?}", event)
                        }
                        SwarmEvent::Behaviour(Event::Dcutr(DirectConnectionUpgradeSucceeded {remote_peer_id})) => {
                            let username = utils::get_username_from_peer_id(remote_peer_id);
                            self.channels.in_p2p_sender.send(Message::Text(format!("{} connected p2p!", username))).await.unwrap();
                            swarm.behaviour_mut().floodsub.add_node_to_partial_view(remote_peer_id);
                        }
                        SwarmEvent::Behaviour(Event::Identify(event)) => {
                            info!("{:?}", event)
                        }
                        SwarmEvent::Behaviour(Event::Floodsub(FloodsubEvent::Message(msg))) => {
                            let message = bincode::deserialize::<Message>(&msg.data).unwrap_or(Message::Empty);
                            self.channels.in_p2p_sender.send(message).await.unwrap();
                        }
                        SwarmEvent::Behaviour(Event::Ping(_event)) => {
                        }
                        SwarmEvent::ConnectionEstablished {
                            peer_id: _, endpoint: _, ..
                        } => {}
                        SwarmEvent::OutgoingConnectionError { peer_id: _, error } => {
                            info!("{:?}", error);
                            if log_enabled!(Level::Debug) {
                                self.channels.in_p2p_sender.send(Message::Text(format!("{:?}", error))).await.unwrap();
                            }
                        }
                        _ => {}
                    },
                    message = self.channels.out_p2p_receiver.next() => match message {
                        None => {},
                        Some(mut msg) => {
                            match msg {
                                Message::UserMessage(ref mut msg) => {
                                    msg.username = Some(self.username.clone());
                                }
                                _ => {}
                            }
                            let encoded = bincode::serialize(&msg).unwrap();
                            swarm.behaviour_mut().floodsub.publish(self.main_topic.clone(), encoded);
                        }
                    }
                );
            }
        });
    }
}
