use futures::{executor::block_on, prelude::*, select};
use libp2p::dcutr::behaviour::Event::DirectConnectionUpgradeSucceeded;
use libp2p::{floodsub::FloodsubEvent, relay::v2::client, Swarm};
use libp2p_swarm::SwarmEvent;
use log::{error, info, log_enabled, Level};

use crate::{
    p2p::{event::Event, utils, Behaviour, P2p},
    types::Message,
};

impl P2p {
    pub fn run_swarm_loop(&self, swarm: &mut Swarm<Behaviour>) {
        swarm
            .behaviour_mut()
            .floodsub
            .subscribe(self.main_topic.clone());
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
                            info!("Relay accepted our reservation request.");
                        }
                        SwarmEvent::Behaviour(Event::Relay(event)) => {
                            info!("{:?}", event)
                        }
                        SwarmEvent::Behaviour(Event::Dcutr(DirectConnectionUpgradeSucceeded {remote_peer_id})) => {
                            let username = utils::get_username_from_peer_id(remote_peer_id);
                            in_sender.send(Message::Text(format!("{} connected p2p!", username))).await.unwrap();
                            swarm.behaviour_mut().floodsub.add_node_to_partial_view(remote_peer_id);
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
                            peer_id, endpoint: _, ..
                        } => {}
                        SwarmEvent::OutgoingConnectionError { peer_id: _, error } => {
                            error!("{:?}", error);
                            if log_enabled!(Level::Debug) {
                                in_sender.send(Message::Text(format!("{:?}", error))).await.unwrap();
                            }
                        }
                        _ => {}
                    },
                    message = out_receiver.next() => match message {
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
