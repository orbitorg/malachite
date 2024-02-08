use std::collections::{BTreeMap, HashSet};

use itf::de::{As, Integer, Same};
use serde::Deserialize;

use crate::consensus::{ConsensusInput, ConsensusState};
use crate::types::{Address, Height, Proposal, Round, Step, Timeout, Value, Vote, Weight};
use crate::votekeeper::{VoteKeeper, VoteKeeperOutput};

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct N4F1State {
    #[serde(rename = "line28Test::N4F1::system")]
    pub system: BTreeMap<Address, NodeState>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct N7F1State {
    #[serde(rename = "line28Test::N7F1::system")]
    pub system: BTreeMap<Address, NodeState>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeState {
    #[serde(rename = "es")]
    pub driver_state: DriverState,
    #[serde(with = "As::<HashSet<(Same, Integer, Integer)>>")]
    pub timeouts: HashSet<(Timeout, Height, Round)>,
    pub incoming_votes: HashSet<Vote>,
    pub incoming_proposals: HashSet<Proposal>,
    #[serde(with = "As::<HashSet<(Integer, Integer)>>")]
    pub get_value_requests: HashSet<(Height, Round)>,
    pub next_value_to_propose: Value,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DriverState {
    #[serde(rename = "bk")]
    pub bookkeeper: VoteKeeper,
    #[serde(rename = "cs")]
    pub consensus_state: ConsensusState,
    pub proposals: HashSet<Proposal>,
    #[serde(with = "As::<BTreeMap<Same, Integer>>")]
    pub valset: BTreeMap<Address, Weight>,
    #[serde(with = "As::<Vec<(Same, Integer, Integer, Same)>>")]
    pub executed_inputs: Vec<(ConsensusInput, Height, Round, Step)>,
    #[serde(with = "As::<HashSet<(Same, Integer, Integer)>>")]
    pub pending_inputs: HashSet<(ConsensusInput, Height, Round)>,
    pub pending_step_change: Step,
    pub started: bool,
    pub vote_keeper_output: VoteKeeperOutput,
    pub chain: Vec<Value>,
}
