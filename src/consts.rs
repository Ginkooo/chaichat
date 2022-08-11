use const_format::formatcp;

const RELAY_ADDRESS: &str = "146.59.94.180";
const RELAY_PORT: i32 = 4001;
const RELAY_PEER_ID: &str = "12D3KooWDpJ7As7BWAwRMfu1VU2WCqNjvq387JEYKDBj4kx6nXTN";
pub const DEFAULT_CAMERA_SIZE: &[u16; 2] = &[640, 480];
pub const RELAY_MULTIADDR: &str = formatcp!(
    "/ip4/{}/tcp/{}/p2p/{}",
    RELAY_ADDRESS,
    RELAY_PORT,
    RELAY_PEER_ID
);
