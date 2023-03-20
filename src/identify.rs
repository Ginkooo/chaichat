use crate::config;
use futures::StreamExt;
use libp2p::{
    core::transport::upgrade::Version,
    identify, identity, noise, ping, rendezvous,
    swarm::{keep_alive, NetworkBehaviour, Swarm, SwarmEvent},
    tcp, yamux, Multiaddr, PeerId, Transport,
};
use std::time::Duration;

pub async fn identify() {
    let key_pair = config::KEY_PAIR.clone();
    let rendezvous_point_address = "/ip4/127.0.0.1/tcp/62649".parse::<Multiaddr>().unwrap();
    let rendezvous_point = config::RZV_PEER_ID.parse().unwrap();

    let mut swarm = Swarm::with_tokio_executor(
        tcp::tokio::Transport::default()
            .upgrade(Version::V1)
            .authenticate(noise::NoiseAuthenticated::xx(&key_pair).unwrap())
            .multiplex(yamux::YamuxConfig::default())
            .boxed(),
        MyBehaviour {
            identify: identify::Behaviour::new(identify::Config::new(
                "rendezvous-example/1.0.0".to_string(),
                key_pair.public(),
            )),
            rendezvous: rendezvous::client::Behaviour::new(key_pair.clone()),
            ping: ping::Behaviour::new(ping::Config::new().with_interval(Duration::from_secs(1))),
            keep_alive: keep_alive::Behaviour,
        },
        PeerId::from(key_pair.public()),
    );

    log::info!("Local peer id: {}", swarm.local_peer_id());

    let _ = swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse().unwrap());

    swarm.dial(rendezvous_point_address.clone()).unwrap();

    while let Some(event) = swarm.next().await {
        match event {}
    }
}

#[derive(NetworkBehaviour)]
struct MyBehaviour {
    identify: identify::Behaviour,
    rendezvous: rendezvous::client::Behaviour,
    ping: ping::Behaviour,
    keep_alive: keep_alive::Behaviour,
}
