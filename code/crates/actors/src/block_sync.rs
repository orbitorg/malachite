use std::marker::PhantomData;

use async_trait::async_trait;
use derive_where::derive_where;
use ractor::{Actor, ActorProcessingErr, ActorRef};
use tokio::task::JoinHandle;

use malachite_common::Context;

use crate::consensus::ConsensusRef;
use crate::gossip_consensus::{GossipConsensusMsg, GossipConsensusRef, GossipEvent};
use crate::util::forward::forward;

pub type BlockSyncRef<Ctx> = ActorRef<Msg<Ctx>>;

#[derive_where(Clone, Debug)]
pub enum Msg<Ctx: Context> {
    GossipEvent(GossipEvent<Ctx>),
}

#[derive_where(Clone, Debug)]
pub struct State<Ctx: Context> {
    marker: PhantomData<Ctx>,
}

#[allow(dead_code)]
pub struct BlockSync<Ctx: Context> {
    ctx: Ctx,
    gossip_consensus: GossipConsensusRef<Ctx>,
    consensus: ConsensusRef<Ctx>,
}

impl<Ctx> BlockSync<Ctx>
where
    Ctx: Context,
{
    pub fn new(
        ctx: Ctx,
        gossip_consensus: GossipConsensusRef<Ctx>,
        consensus: ConsensusRef<Ctx>,
    ) -> Self {
        Self {
            ctx,
            gossip_consensus,
            consensus,
        }
    }

    pub async fn spawn(self) -> Result<(BlockSyncRef<Ctx>, JoinHandle<()>), ractor::SpawnErr> {
        Actor::spawn(None, self, ()).await
    }
}

#[async_trait]
impl<Ctx> Actor for BlockSync<Ctx>
where
    Ctx: Context,
{
    type Msg = Msg<Ctx>;
    type State = State<Ctx>;
    type Arguments = ();

    async fn pre_start(
        &self,
        myself: ActorRef<Self::Msg>,
        _args: (),
    ) -> Result<Self::State, ActorProcessingErr> {
        let forward = forward(myself.clone(), Some(myself.get_cell()), Msg::GossipEvent).await?;

        self.gossip_consensus
            .cast(GossipConsensusMsg::Subscribe(forward))?;

        Ok(State {
            marker: PhantomData,
        })
    }

    #[tracing::instrument(name = "node", skip_all)]
    async fn handle(
        &self,
        _myself: ActorRef<Self::Msg>,
        msg: Self::Msg,
        _state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match msg {
            Msg::GossipEvent(_event) => {}
        }

        Ok(())
    }
}
