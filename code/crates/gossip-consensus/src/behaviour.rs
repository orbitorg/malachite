use std::time::Duration;

use either::Either;
use libp2p::swarm::NetworkBehaviour;
use libp2p::{floodsub, gossipsub, identify, ping};

pub use libp2p::identity::Keypair;
pub use libp2p::{Multiaddr, PeerId};

use malachite_metrics::Registry;

use crate::{PubSubProtocol, PROTOCOL_VERSION};

const MAX_TRANSMIT_SIZE: usize = 4 * 1024 * 1024; // 4 MiB

#[derive(Debug)]
pub enum NetworkEvent {
    Identify(identify::Event),
    Ping(ping::Event),
    GossipSub(gossipsub::Event),
    FloodSub(floodsub::FloodsubEvent),
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

impl From<gossipsub::Event> for NetworkEvent {
    fn from(event: gossipsub::Event) -> Self {
        Self::GossipSub(event)
    }
}

impl From<floodsub::FloodsubEvent> for NetworkEvent {
    fn from(event: floodsub::FloodsubEvent) -> Self {
        Self::FloodSub(event)
    }
}

impl<A, B> From<Either<A, B>> for NetworkEvent
where
    A: Into<NetworkEvent>,
    B: Into<NetworkEvent>,
{
    fn from(event: Either<A, B>) -> Self {
        match event {
            Either::Left(event) => event.into(),
            Either::Right(event) => event.into(),
        }
    }
}

#[derive(NetworkBehaviour)]
#[behaviour(to_swarm = "NetworkEvent")]
pub struct Behaviour {
    pub identify: identify::Behaviour,
    pub ping: ping::Behaviour,
    pub pubsub: Either<gossipsub::Behaviour, floodsub::Floodsub>,
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
    pub fn new_with_metrics(
        tpe: PubSubProtocol,
        keypair: &Keypair,
        registry: &mut Registry,
    ) -> Self {
        let identify = identify::Behaviour::new(identify::Config::new(
            PROTOCOL_VERSION.to_string(),
            keypair.public(),
        ));

        let ping = ping::Behaviour::new(ping::Config::new().with_interval(Duration::from_secs(5)));

        let pubsub = match tpe {
            PubSubProtocol::GossipSub => Either::Left(
                gossipsub::Behaviour::new_with_metrics(
                    gossipsub::MessageAuthenticity::Signed(keypair.clone()),
                    gossipsub_config(),
                    registry,
                    Default::default(),
                )
                .unwrap(),
            ),
            PubSubProtocol::FloodSub => {
                let local_peer_id = PeerId::from_public_key(&keypair.public());
                Either::Right(floodsub::Floodsub::new(local_peer_id))
            }
        };

        Self {
            identify,
            ping,
            pubsub,
        }
    }
}
