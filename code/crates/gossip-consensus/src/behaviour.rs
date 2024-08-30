use std::time::Duration;

use either::Either;
use libp2p::request_response::ProtocolSupport;
use libp2p::swarm::NetworkBehaviour;
use libp2p::StreamProtocol;
use libp2p::{gossipsub, identify, ping, request_response};
use serde::{Deserialize, Serialize};

pub use libp2p::identity::Keypair;
pub use libp2p::{Multiaddr, PeerId};

use malachite_metrics::Registry;

use crate::{Channel, NetworkType, PROTOCOL_VERSION};

const MAX_TRANSMIT_SIZE: usize = 4 * 1024 * 1024; // 4 MiB

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Request {
    Subscribe(Channel),
    Publish(Channel, Vec<u8>),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Response {
    Ok,
    Error(String),
}

pub type ReqResEvent = request_response::Event<Request, Response>;
pub type ReqResBehaviour = request_response::cbor::Behaviour<Request, Response>;

#[derive(Debug)]
pub enum NetworkEvent {
    Identify(identify::Event),
    Ping(ping::Event),
    GossipSub(gossipsub::Event),
    RequestResponse(ReqResEvent),
}

impl From<identify::Event> for NetworkEvent {
    fn from(event: identify::Event) -> Self {
        Self::Identify(event)
    }
}

impl From<ping::Event> for NetworkEvent {
    fn from(event: ping::Event) -> Self {
        Self::Ping(event)
    }
}

impl From<Either<gossipsub::Event, ReqResEvent>> for NetworkEvent {
    fn from(event: Either<gossipsub::Event, ReqResEvent>) -> Self {
        match event {
            Either::Left(event) => Self::GossipSub(event),
            Either::Right(event) => Self::RequestResponse(event),
        }
    }
}

#[derive(NetworkBehaviour)]
#[behaviour(to_swarm = "NetworkEvent")]
pub struct Behaviour {
    pub identify: identify::Behaviour,
    pub ping: ping::Behaviour,
    pub pubsub: Either<gossipsub::Behaviour, ReqResBehaviour>,
}

fn message_id(message: &gossipsub::Message) -> gossipsub::MessageId {
    use seahash::SeaHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = SeaHasher::new();
    message.hash(&mut hasher);
    gossipsub::MessageId::new(hasher.finish().to_be_bytes().as_slice())
}

fn gossipsub_config() -> gossipsub::Config {
    gossipsub::ConfigBuilder::default()
        .max_transmit_size(MAX_TRANSMIT_SIZE)
        .opportunistic_graft_ticks(3)
        .heartbeat_interval(Duration::from_secs(1))
        .validation_mode(gossipsub::ValidationMode::Strict)
        .history_gossip(3)
        .history_length(5)
        .mesh_n_high(4)
        .mesh_n_low(1)
        .mesh_outbound_min(1)
        .mesh_n(3)
        .message_id_fn(message_id)
        .build()
        .unwrap()
}

impl Behaviour {
    pub fn new_with_metrics(tpe: NetworkType, keypair: &Keypair, registry: &mut Registry) -> Self {
        let identify = identify::Behaviour::new(identify::Config::new(
            PROTOCOL_VERSION.to_string(),
            keypair.public(),
        ));

        let ping = ping::Behaviour::new(ping::Config::new().with_interval(Duration::from_secs(5)));

        let pubsub = match tpe {
            NetworkType::GossipSub => Either::Left(
                gossipsub::Behaviour::new_with_metrics(
                    gossipsub::MessageAuthenticity::Signed(keypair.clone()),
                    gossipsub_config(),
                    registry,
                    Default::default(),
                )
                .unwrap(),
            ),
            NetworkType::Broadcast => Either::Right(request_response::cbor::Behaviour::new(
                [(
                    StreamProtocol::new("/malachite-broadcast-consensus/v1beta1"),
                    ProtocolSupport::Full,
                )],
                request_response::Config::default(),
            )),
        };

        Self {
            identify,
            ping,
            pubsub,
        }
    }
}
