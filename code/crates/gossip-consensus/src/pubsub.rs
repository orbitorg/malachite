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
    if let Some(gs) = swarm.behaviour_mut().gossipsub.as_mut() {
        for channel in channels {
            gs.subscribe(&channel.to_topic())?;
        }
    }

    Ok(())
}

pub fn subscribe_to_peer(
    swarm: &mut swarm::Swarm<Behaviour>,
    peer: &PeerId,
    channels: &[Channel],
) -> Result<(), BoxError> {
    if let Some(rr) = swarm.behaviour_mut().request_response.as_mut() {
        for channel in channels {
            let _request_id = rr.send_request(peer, Request::Subscribe(*channel));
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
    if let Some(gs) = swarm.behaviour_mut().gossipsub.as_mut() {
        gs.publish(channel.topic_hash(), data)?;
        return Ok(());
    }

    if let Some(rr) = swarm.behaviour_mut().request_response.as_mut() {
        for peer in state.subscribers(channel) {
            let _request_id = rr.send_request(peer, Request::Publish(channel, data.to_vec()));
        }
        return Ok(());
    }

    Err("No gossipsub or broadcast protocol enabled".into())
}

pub fn reply(
    swarm: &mut swarm::Swarm<Behaviour>,
    reply: ResponseChannel<Response>,
    response: Response,
) -> Result<(), BoxError> {
    if let Some(rr) = swarm.behaviour_mut().request_response.as_mut() {
        rr.send_response(reply, response).unwrap(); // FIXME: unwrap
    }

    Ok(())
}
