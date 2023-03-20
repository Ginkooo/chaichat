use crate::config;
use futures::StreamExt;
use libp2p::{
    core::transport::upgrade::Version,
    identity, noise, ping, rendezvous,
    swarm::{keep_alive, AddressScore, NetworkBehaviour, Swarm, SwarmEvent},
    tcp, yamux, Multiaddr, PeerId, Transport,
};
use std::time::Duration;

pub async fn register() {
    // In production the external address should be the publicly facing IP address of the rendezvous point.
    // This address is recorded in the registration entry by the rendezvous point.

    while let Some(event) = swarm.next().await {
        match event {}
    }
}

#[derive(NetworkBehaviour)]
struct MyBehaviour {
    rendezvous: rendezvous::client::Behaviour,
    ping: ping::Behaviour,
    keep_alive: keep_alive::Behaviour,
}
