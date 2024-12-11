//! 6The function that you hand over to malachite-cli Start.

use tracing::{info, Instrument};

use crate::node::StarknetNode;
use crate::spawn::spawn_node_actor;

use malachite_starknet_p2p_types::Height;
use malachite_app::Node;
use malachite_actors::util::events::TxEvent;

