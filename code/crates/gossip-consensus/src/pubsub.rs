use bytes::Bytes;
use either::Either;
use libp2p::swarm;

use crate::behaviour::Behaviour;
use crate::{BoxError, Channel};

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
