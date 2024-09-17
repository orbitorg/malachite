use derive_where::derive_where;
use libp2p_identity::PeerId;
use std::collections::HashSet;
use std::hash::Hash;

use malachite_common::*;

use crate::GossipMsg;

/// Maintains latest seen (p, h, r, s, id(v)) for each peer

#[derive_where(Eq, Hash, PartialEq)]
pub struct CrdtKey<Ctx>
where
    Ctx: Context
{
    peer_id: PeerId,
    height: Ctx::Height,
    round: Round,
    value: NilOrVal<<Ctx::Value as Value>::Id>,
}

/// Maintains latest seen (p, h, r, s, id(v)) for each peer
pub struct Crdt<Ctx>
where
    Ctx: Context,
{
    pub peer_state: HashSet<CrdtKey<Ctx>>,
}

impl<Ctx> Crdt<Ctx>
where
    Ctx: Context,
{
    pub fn store_msg(&mut self, peer_id: PeerId, msg: GossipMsg<Ctx>) {
        let key = CrdtKey {
            peer_id,
            height: msg.msg_height(),
            round: msg.msg_round(),
            value: msg.msg_value_id(),
        };
        self
            .peer_state
            .insert(key);
    }
}
