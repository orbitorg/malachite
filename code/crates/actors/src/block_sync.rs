use std::marker::PhantomData;
use std::time::Duration;

use async_trait::async_trait;
use derive_where::derive_where;
use ractor::{Actor, ActorProcessingErr, ActorRef};
use tokio::task::JoinHandle;

use malachite_common::Context;
use tracing::info;

use crate::consensus::{ConsensusMsg, ConsensusRef};
use crate::gossip_consensus::{GossipConsensusMsg, GossipConsensusRef, GossipEvent, Status};
use crate::util::forward::forward;

pub type BlockSyncRef<Ctx> = ActorRef<Msg<Ctx>>;

#[derive_where(Clone, Debug)]
pub enum Msg<Ctx: Context> {
    Tick,
    GossipEvent(GossipEvent<Ctx>),
}

#[derive(Debug)]
pub struct Args {
    pub status_update_interval: Duration,
}

impl Default for Args {
    fn default() -> Self {
        Self {
            status_update_interval: Duration::from_secs(1),
        }
    }
}

#[derive_where(Debug)]
pub struct State<Ctx: Context> {
    ticker: JoinHandle<()>,
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
        Actor::spawn(None, self, Args::default()).await
    }
}

#[async_trait]
impl<Ctx> Actor for BlockSync<Ctx>
where
    Ctx: Context,
{
    type Msg = Msg<Ctx>;
    type State = State<Ctx>;
    type Arguments = Args;

    async fn pre_start(
        &self,
        myself: ActorRef<Self::Msg>,
        args: Args,
    ) -> Result<Self::State, ActorProcessingErr> {
        let forward = forward(myself.clone(), Some(myself.get_cell()), Msg::GossipEvent).await?;

        self.gossip_consensus
            .cast(GossipConsensusMsg::Subscribe(forward))?;

        let ticker = tokio::spawn(async move {
            loop {
                tokio::time::sleep(args.status_update_interval).await;

                if let Err(e) = myself.cast(Msg::Tick) {
                    tracing::error!(?e, "Failed to send tick message");
                }
            }
        });

        Ok(State {
            ticker,
            marker: PhantomData,
        })
    }

    #[tracing::instrument(name = "blocksync", skip_all)]
    async fn handle(
        &self,
        _myself: ActorRef<Self::Msg>,
        msg: Self::Msg,
        _state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        #[allow(clippy::single_match)]
        match msg {
            Msg::Tick => {
                let Ok(status) = ractor::call!(self.consensus, ConsensusMsg::GetStatus) else {
                    tracing::error!("Failed to get consensus status");
                    return Ok(());
                };

                self.gossip_consensus
                    .cast(GossipConsensusMsg::PublishStatus(status))?;
            }

            Msg::GossipEvent(GossipEvent::Status(from, Status { height, round })) => {
                info!(%from, %height, %round, "Received peer status");
            }

            Msg::GossipEvent(_) => (), // We don't care about other gossip events,
        }

        Ok(())
    }

    async fn post_stop(
        &self,
        _myself: ActorRef<Self::Msg>,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        state.ticker.abort();
        Ok(())
    }
}
