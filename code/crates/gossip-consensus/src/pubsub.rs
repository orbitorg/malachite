use bytes::Bytes;
use either::Either;
use libp2p::{swarm, PeerId};

use crate::behaviour::Behaviour;
use crate::{BoxError, Channel};

pub fn add_peer(swarm: &mut swarm::Swarm<Behaviour>, peer_id: PeerId) -> Result<(), BoxError> {
    if let Either::Right(floodsub) = &mut swarm.behaviour_mut().pubsub {
        floodsub.add_node_to_partial_view(peer_id);
    }

    Ok(())
}

pub fn subscribe(
    swarm: &mut swarm::Swarm<Behaviour>,
    channels: &[Channel],
) -> Result<(), BoxError> {
    match &mut swarm.behaviour_mut().pubsub {
        Either::Left(gossipsub) => {
            for channel in channels {
                gossipsub.subscribe(&channel.to_gossipsub_topic())?;
            }
        }
        Either::Right(floodsub) => {
            for channel in channels {
                floodsub.subscribe(channel.to_floodsub_topic());
            }
        }
    }

    Ok(())
}

pub fn publish(
    swarm: &mut swarm::Swarm<Behaviour>,
    channel: Channel,
    data: Bytes,
) -> Result<(), BoxError> {
    match &mut swarm.behaviour_mut().pubsub {
        Either::Left(gossipsub) => {
            gossipsub.publish(channel.to_gossipsub_topic(), data)?;
        }
        Either::Right(floodsub) => {
            floodsub.publish(channel.to_floodsub_topic(), data);
        }
    }

    Ok(())
}
