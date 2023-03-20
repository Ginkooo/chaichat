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
    let key_pair = identity::Keypair::generate_ed25519();
    let rendezvous_point_address = format!(
        "/ip4/{}/tcp/{}",
        *config::RZV_SERVER_IP,
        *config::RZV_SERVER_PORT
    )
    .parse::<Multiaddr>()
    .unwrap();
    let rendezvous_point = config::RZV_PEER_ID.parse().unwrap();

    let mut swarm = Swarm::with_tokio_executor(
        tcp::tokio::Transport::default()
            .upgrade(Version::V1)
            .authenticate(noise::NoiseAuthenticated::xx(&key_pair).unwrap())
            .multiplex(yamux::YamuxConfig::default())
            .boxed(),
        MyBehaviour {
            rendezvous: rendezvous::client::Behaviour::new(key_pair.clone()),
            ping: ping::Behaviour::new(ping::Config::new().with_interval(Duration::from_secs(1))),
            keep_alive: keep_alive::Behaviour,
        },
        PeerId::from(key_pair.public()),
    );

    // In production the external address should be the publicly facing IP address of the rendezvous point.
    // This address is recorded in the registration entry by the rendezvous point.
    let external_address = "/ip4/127.0.0.1/tcp/0".parse::<Multiaddr>().unwrap();
    swarm.add_external_address(external_address, AddressScore::Infinite);

    log::info!("Local peer id: {}", swarm.local_peer_id());

    swarm.dial(rendezvous_point_address.clone()).unwrap();

    while let Some(event) = swarm.next().await {
        match event {
            SwarmEvent::NewListenAddr { address, .. } => {
                log::info!("Listening on {}", address);
            }
            SwarmEvent::ConnectionClosed {
                peer_id,
                cause: Some(error),
                ..
            } if peer_id == rendezvous_point => {
                log::error!("Lost connection to rendezvous point {}", error);
            }
            SwarmEvent::ConnectionEstablished { peer_id, .. } if peer_id == rendezvous_point => {
                swarm.behaviour_mut().rendezvous.register(
                    rendezvous::Namespace::from_static("rendezvous"),
                    rendezvous_point,
                    None,
                );
                log::info!("Connection established with rendezvous point {}", peer_id);
            }
            // once `/identify` did its job, we know our external address and can register
            SwarmEvent::Behaviour(MyBehaviourEvent::Rendezvous(
                rendezvous::client::Event::Registered {
                    namespace,
                    ttl,
                    rendezvous_node,
                },
            )) => {
                log::info!(
                    "Registered for namespace '{}' at rendezvous point {} for the next {} seconds",
                    namespace,
                    rendezvous_node,
                    ttl
                );
            }
            SwarmEvent::Behaviour(MyBehaviourEvent::Rendezvous(
                rendezvous::client::Event::RegisterFailed(error),
            )) => {
                log::error!("Failed to register {}", error);
                return;
            }
            SwarmEvent::Behaviour(MyBehaviourEvent::Ping(ping::Event {
                peer,
                result: Ok(ping::Success::Ping { rtt }),
            })) if peer != rendezvous_point => {
                log::info!("Ping to {} is {}ms", peer, rtt.as_millis())
            }
            other => {
                log::debug!("Unhandled {:?}", other);
            }
        }
    }
}

#[derive(NetworkBehaviour)]
struct MyBehaviour {
    rendezvous: rendezvous::client::Behaviour,
    ping: ping::Behaviour,
    keep_alive: keep_alive::Behaviour,
}
