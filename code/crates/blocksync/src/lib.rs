use derive_where::derive_where;
use displaydoc::Display;
use libp2p_identity::PeerId;

use malachite_common::{Context, Round};

#[derive(Display)]
#[displaydoc("Status {{ peer_id: {peer_id}, height: {height}, round: {round} }}")]
#[derive_where(Debug, PartialEq, Eq)]
pub struct Status<Ctx: Context> {
    pub peer_id: PeerId,
    pub height: Ctx::Height,
    pub round: Round,
}
