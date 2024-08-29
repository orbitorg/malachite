use core::fmt;

use libp2p::{floodsub, gossipsub};
use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Channel {
    Consensus,
    ProposalParts,
}

impl Channel {
    pub fn all() -> &'static [Channel] {
        &[Channel::Consensus, Channel::ProposalParts]
    }

    pub fn to_gossipsub_topic(self) -> gossipsub::IdentTopic {
        gossipsub::IdentTopic::new(self.as_str())
    }

    pub fn to_floodsub_topic(self) -> floodsub::Topic {
        floodsub::Topic::new(self.to_string())
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Channel::Consensus => "/consensus",
            Channel::ProposalParts => "/proposal_parts",
        }
    }

    pub fn has_gossipsub_topic(topic_hash: &gossipsub::TopicHash) -> bool {
        Self::all()
            .iter()
            .any(|channel| &channel.to_gossipsub_topic().hash() == topic_hash)
    }

    pub fn has_floodsub_topic(topic: &floodsub::Topic) -> bool {
        Self::all()
            .iter()
            .any(|channel| &channel.to_floodsub_topic() == topic)
    }

    pub fn from_gossipsub_topic_hash(topic: &gossipsub::TopicHash) -> Option<Self> {
        match topic.as_str() {
            "/consensus" => Some(Channel::Consensus),
            "/proposal_parts" => Some(Channel::ProposalParts),
            _ => None,
        }
    }

    pub fn from_floodsub_topic(topic: &floodsub::Topic) -> Option<Self> {
        match topic.id() {
            "/consensus" => Some(Channel::Consensus),
            "/proposal_parts" => Some(Channel::ProposalParts),
            _ => None,
        }
    }
}

impl fmt::Display for Channel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.as_str().fmt(f)
    }
}
