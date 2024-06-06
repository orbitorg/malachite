use std::collections::BTreeSet;
use std::sync::Arc;

use async_trait::async_trait;
use libp2p::identity::Keypair;
use malachite_common::MempoolTransactionBatch;
use ractor::ActorCell;
use ractor::ActorProcessingErr;
use ractor::ActorRef;
use ractor::{Actor, RpcReplyPort};
use tokio::task::JoinHandle;

use malachite_gossip_mempool::handle::CtrlHandle;
use malachite_gossip_mempool::{Channel, Config, Event, PeerId};
use malachite_proto::Protobuf;

pub type GossipMempoolRef = ActorRef<Msg>;

pub struct GossipMempool;

impl GossipMempool {
    pub async fn spawn(
        keypair: Keypair,
        config: Config,
        supervisor: Option<ActorCell>,
    ) -> Result<ActorRef<Msg>, ractor::SpawnErr> {
        let args = Args { keypair, config };

        let (actor_ref, _) = if let Some(supervisor) = supervisor {
            Actor::spawn_linked(None, Self, args, supervisor).await?
        } else {
            Actor::spawn(None, Self, args).await?
        };

        Ok(actor_ref)
    }
}

pub struct Args {
    pub keypair: Keypair,
    pub config: Config,
}

pub enum State {
    Stopped,
    Running {
        peers: BTreeSet<PeerId>,
        subscribers: Vec<ActorRef<Arc<Event>>>,
        ctrl_handle: CtrlHandle,
        recv_task: JoinHandle<()>,
    },
}

pub enum Msg {
    /// Subscribe to gossip events
    Subscribe(ActorRef<Arc<Event>>),

    /// Broadcast a message to all peers
    Broadcast(Channel, MempoolTransactionBatch),

    /// Request the number of connected peers
    GetState { reply: RpcReplyPort<usize> },

    // Internal message
    #[doc(hidden)]
    NewEvent(Event),
}

#[async_trait]
impl Actor for GossipMempool {
    type Msg = Msg;
    type State = State;
    type Arguments = Args;

    async fn pre_start(
        &self,
        myself: ActorRef<Msg>,
        args: Args,
    ) -> Result<State, ActorProcessingErr> {
        let handle = malachite_gossip_mempool::spawn(args.keypair, args.config).await?;
        let (mut recv_handle, ctrl_handle) = handle.split();

        let recv_task = tokio::spawn({
            async move {
                while let Some(event) = recv_handle.recv().await {
                    myself.cast(Msg::NewEvent(event)).unwrap(); // FIXME
                }
            }
        });

        Ok(State::Running {
            peers: BTreeSet::new(),
            subscribers: Vec::new(),
            ctrl_handle,
            recv_task,
        })
    }

    async fn post_start(
        &self,
        _myself: ActorRef<Msg>,
        _state: &mut State,
    ) -> Result<(), ActorProcessingErr> {
        Ok(())
    }

    #[tracing::instrument(name = "gossip.mempool", skip(self, _myself, msg, state))]
    async fn handle(
        &self,
        _myself: ActorRef<Msg>,
        msg: Msg,
        state: &mut State,
    ) -> Result<(), ActorProcessingErr> {
        let State::Running {
            peers,
            subscribers,
            ctrl_handle,
            ..
        } = state
        else {
            return Ok(());
        };

        match msg {
            Msg::Subscribe(subscriber) => subscribers.push(subscriber),
            Msg::Broadcast(channel, batch) => {
                let bytes = batch.to_bytes().unwrap();
                ctrl_handle.broadcast(channel, bytes).await?
            }
            Msg::NewEvent(event) => {
                match event {
                    Event::PeerConnected(peer_id) => {
                        peers.insert(peer_id);
                    }
                    Event::PeerDisconnected(peer_id) => {
                        peers.remove(&peer_id);
                    }
                    _ => {}
                }

                let event = Arc::new(event);
                for subscriber in subscribers {
                    subscriber.cast(Arc::clone(&event))?;
                }
            }
            Msg::GetState { reply } => {
                let number_peers = match state {
                    State::Stopped => 0,
                    State::Running { peers, .. } => peers.len(),
                };
                reply.send(number_peers)?;
            }
        }

        Ok(())
    }

    async fn post_stop(
        &self,
        _myself: ActorRef<Msg>,
        state: &mut State,
    ) -> Result<(), ActorProcessingErr> {
        let state = std::mem::replace(state, State::Stopped);

        if let State::Running {
            ctrl_handle,
            recv_task,
            ..
        } = state
        {
            ctrl_handle.wait_shutdown().await?;
            recv_task.await?;
        }

        Ok(())
    }
}