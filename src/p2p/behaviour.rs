use crate::p2p::event::Event;
use libp2p::dcutr;
use libp2p::identify::{Identify, IdentifyConfig};
use libp2p::identity::PublicKey;
use libp2p::ping::{Ping, PingConfig};
use libp2p::relay::v2::client::Client;
use libp2p::NetworkBehaviour;

#[derive(NetworkBehaviour)]
#[behaviour(out_event = "Event", event_process = false)]
pub struct Behaviour {
    relay_client: Client,
    ping: Ping,
    identify: Identify,
    dcutr: dcutr::behaviour::Behaviour,
}

impl Behaviour {
    pub fn new(public_key: PublicKey, client: Client) -> Self {
        Behaviour {
            relay_client: client,
            ping: Ping::new(PingConfig::new()),
            identify: Identify::new(IdentifyConfig::new("/TODO/0.0.1".to_string(), public_key)),
            dcutr: dcutr::behaviour::Behaviour::new(),
        }
    }
}
