use crate::config;
use futures::StreamExt;
use libp2p::{
    core::transport::upgrade::Version,
    multiaddr::Protocol,
    noise, ping, rendezvous,
    swarm::{keep_alive, NetworkBehaviour, SwarmBuilder, SwarmEvent},
    tcp, yamux, Multiaddr, PeerId, Transport,
};
use std::time::Duration;

const NAMESPACE: &str = "rendezvous";

pub async fn discover() {
    let key_pair = config::KEY_PAIR.clone();
    let rendezvous_point_address = format!(
        "/ip4/{}/tcp/{}",
        *config::RZV_SERVER_IP,
        *config::RZV_SERVER_PORT
    )
    .parse::<Multiaddr>()
    .unwrap();
    let rendezvous_point = config::RZV_PEER_ID.parse().unwrap();

    let mut swarm = SwarmBuilder::with_tokio_executor(
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
    )
    .build();

    log::info!("Local peer id: {}", swarm.local_peer_id());

    swarm.dial(rendezvous_point_address.clone()).unwrap();

    let mut discover_tick = tokio::time::interval(Duration::from_secs(30));
    let mut cookie = None;

    loop {
        tokio::select! {
                event = swarm.select_next_some() => match event {
                    SwarmEvent::ConnectionEstablished { peer_id, .. } if peer_id == rendezvous_point => {
                        log::info!(
                            "Connected to rendezvous point, discovering nodes in '{}' namespace ...",
                            NAMESPACE
                        );

                        swarm.behaviour_mut().rendezvous.discover(
                            Some(rendezvous::Namespace::new(NAMESPACE.to_string()).unwrap()),
                            None,
                            None,
                            rendezvous_point,
                        );
                    }
                    SwarmEvent::Behaviour(MyBehaviourEvent::Rendezvous(rendezvous::client::Event::Discovered {
                        registrations,
                        cookie: new_cookie,
                        ..
                    })) => {
                        cookie.replace(new_cookie);

                        for registration in registrations {
                            for address in registration.record.addresses() {
                                let peer = registration.record.peer_id();
                                log::info!("Discovered peer {} at {}", peer, address);

                                let p2p_suffix = Protocol::P2p(*peer.as_ref());
                                let address_with_p2p =
                                    if !address.ends_with(&Multiaddr::empty().with(p2p_suffix.clone())) {
                                        address.clone().with(p2p_suffix)
                                    } else {
                                        address.clone()
                                    };

                                swarm.dial(address_with_p2p).unwrap();
                            }
                        }
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
            },
            _ = discover_tick.tick(), if cookie.is_some() =>
                swarm.behaviour_mut().rendezvous.discover(
                    Some(rendezvous::Namespace::new(NAMESPACE.to_string()).unwrap()),
                    cookie.clone(),
                    None,
                    rendezvous_point
                    )
        }
    }
}

#[derive(NetworkBehaviour)]
struct MyBehaviour {
    rendezvous: rendezvous::client::Behaviour,
    ping: ping::Behaviour,
    keep_alive: keep_alive::Behaviour,
}
