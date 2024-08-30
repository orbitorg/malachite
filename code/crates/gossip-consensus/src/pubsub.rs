use either::Either;
use libp2p::request_response::ResponseChannel;
use libp2p::swarm;

pub use bytes::Bytes;
pub use libp2p::gossipsub::MessageId;
pub use libp2p::identity::Keypair;
pub use libp2p::{Multiaddr, PeerId};

use crate::behaviour::{Behaviour, Request, Response};
use crate::{BoxError, Channel, State};

pub fn subscribe(
    swarm: &mut swarm::Swarm<Behaviour>,
    channels: &[Channel],
) -> Result<(), BoxError> {
    if let Either::Left(gossipsub) = &mut swarm.behaviour_mut().pubsub {
        for channel in channels {
            gossipsub.subscribe(&channel.to_topic())?;
        }
    }

    Ok(())
}

pub fn subscribe_to_peer(
    swarm: &mut swarm::Swarm<Behaviour>,
    peer: &PeerId,
    channels: &[Channel],
) -> Result<(), BoxError> {
    if let Either::Right(rpc) = &mut swarm.behaviour_mut().pubsub {
        for channel in channels {
            let _request_id = rpc.send_request(peer, Request::Subscribe(*channel));
        }
    }

    Ok(())
}

pub fn publish(
    swarm: &mut swarm::Swarm<Behaviour>,
    state: &mut State,
    channel: Channel,
    data: Bytes,
) -> Result<(), BoxError> {
    match &mut swarm.behaviour_mut().pubsub {
        Either::Left(gossipsub) => {
            gossipsub.publish(channel.topic_hash(), data)?;
        }
        Either::Right(rpc) => {
            for peer in state.subscribers(channel) {
                let _request_id = rpc.send_request(peer, Request::Publish(channel, data.to_vec()));
            }
        }
    }

    Ok(())
}

pub fn reply(
    swarm: &mut swarm::Swarm<Behaviour>,
    reply: ResponseChannel<Response>,
    response: Response,
) -> Result<(), BoxError> {
    if let Either::Right(rpc) = &mut swarm.behaviour_mut().pubsub {
        rpc.send_response(reply, response).unwrap(); // FIXME: unwrap
    }

    Ok(())
}
