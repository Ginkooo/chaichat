use futures::executor::block_on;
use libp2p::core::muxing::StreamMuxerBox;
use libp2p::core::transport::{Boxed, OrTransport};
use libp2p::core::upgrade;
use libp2p::dns::DnsConfig;
use libp2p::{
    identity::Keypair,
    noise,
    relay::v2::client::Client,
    tcp::{GenTcpConfig, TcpTransport},
};
use libp2p::{PeerId, Transport};

pub fn create_transport(
    key: Keypair,
    peer_id: PeerId,
) -> (Boxed<(PeerId, StreamMuxerBox)>, Client) {
    let (relay_transport, client) = Client::new_transport_and_behaviour(peer_id);

    let noise_keys = noise::Keypair::<noise::X25519Spec>::new()
        .into_authentic(&key)
        .expect("Signing libp2p-noise static DH keypair failed.");

    (
        OrTransport::new(
            relay_transport,
            block_on(DnsConfig::system(TcpTransport::new(
                GenTcpConfig::default().port_reuse(true),
            )))
            .unwrap(),
        )
        .upgrade(upgrade::Version::V1)
        .authenticate(noise::NoiseConfig::xx(noise_keys).into_authenticated())
        .multiplex(libp2p_yamux::YamuxConfig::default())
        .boxed(),
        client,
    )
}
