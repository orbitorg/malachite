use std::time::Duration;

use libp2p::swarm::NetworkBehaviour;
use libp2p::{gossipsub, identify};

pub use libp2p::identity::Keypair;
pub use libp2p::{Multiaddr, PeerId};
use malachite_metrics::Registry;

use crate::PROTOCOL_VERSION;

const MAX_TRANSMIT_SIZE: usize = 4 * 1024 * 1024; // 4 MiB

#[derive(NetworkBehaviour)]
#[behaviour(to_swarm = "NetworkEvent")]
pub struct Behaviour {
    pub identify: identify::Behaviour,
    pub gossipsub: gossipsub::Behaviour,
}

fn message_id(message: &gossipsub::Message) -> gossipsub::MessageId {
    let hash = blake3::hash(&message.data);
    gossipsub::MessageId::from(hash.as_bytes().to_vec())
}

fn gossipsub_config() -> gossipsub::Config {
    gossipsub::ConfigBuilder::default()
        .max_transmit_size(MAX_TRANSMIT_SIZE)
        .opportunistic_graft_ticks(3)
        .heartbeat_interval(Duration::from_secs(1))
        .validation_mode(gossipsub::ValidationMode::Strict)
        .history_gossip(3)
        .history_length(5)
        .mesh_n_high(12)
        .mesh_n_low(4)
        .mesh_outbound_min(2)
        .mesh_n(6)
        .message_id_fn(message_id)
        .build()
        .unwrap()
}

impl Behaviour {
    pub fn new(keypair: &Keypair) -> Self {
        Self {
            identify: identify::Behaviour::new(identify::Config::new(
                PROTOCOL_VERSION.to_string(),
                keypair.public(),
            )),
            gossipsub: gossipsub::Behaviour::new(
                gossipsub::MessageAuthenticity::Signed(keypair.clone()),
                gossipsub_config(),
            )
            .unwrap(),
        }
    }

    pub fn new_with_metrics(keypair: &Keypair, registry: &mut Registry) -> Self {
        Self {
            identify: identify::Behaviour::new(identify::Config::new(
                PROTOCOL_VERSION.to_string(),
                keypair.public(),
            )),
            gossipsub: gossipsub::Behaviour::new_with_metrics(
                gossipsub::MessageAuthenticity::Signed(keypair.clone()),
                gossipsub_config(),
                registry,
                Default::default(),
            )
            .unwrap(),
        }
    }
}

#[derive(Debug)]
pub enum NetworkEvent {
    Identify(identify::Event),
    GossipSub(gossipsub::Event),
}

impl From<identify::Event> for NetworkEvent {
    fn from(event: identify::Event) -> Self {
        Self::Identify(event)
    }
}

impl From<gossipsub::Event> for NetworkEvent {
    fn from(event: gossipsub::Event) -> Self {
        Self::GossipSub(event)
    }
}
