use std::marker::PhantomData;
use std::path::PathBuf;

use derive_where::derive_where;
use eyre::eyre;
use ractor::{async_trait, Actor, ActorProcessingErr, ActorRef, RpcReplyPort, SpawnErr};
use tokio::sync::{mpsc, oneshot};
use tracing::{debug, error, info};

use malachite_common::Context;
use malachite_metrics::SharedRegistry;
use malachite_wal as wal;

mod entry;
mod thread;

pub use entry::WalCodec;
pub use entry::WalEntry;

pub type WalRef<Ctx> = ActorRef<Msg<Ctx>>;

#[derive_where(Default)]
pub struct Wal<Ctx, Codec> {
    _marker: PhantomData<(Ctx, Codec)>,
}

impl<Ctx, Codec> Wal<Ctx, Codec>
where
    Ctx: Context,
    Codec: WalCodec<Ctx>,
{
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn spawn(
        _ctx: &Ctx,
        moniker: String,
        codec: Codec,
        path: PathBuf,
        _metrics: SharedRegistry,
    ) -> Result<WalRef<Ctx>, SpawnErr> {
        let (actor_ref, _) = Actor::spawn(
            None,
            Self::new(),
            Args {
                moniker,
                path,
                codec,
            },
        )
        .await?;
        Ok(actor_ref)
    }
}

pub type WalReply<T> = RpcReplyPort<eyre::Result<T>>;

pub enum Msg<Ctx: Context> {
    StartedHeight(Ctx::Height, WalReply<Option<Vec<WalEntry<Ctx>>>>),
    Append(Ctx::Height, WalEntry<Ctx>, WalReply<()>),
    Flush(WalReply<()>),
}

pub struct Args<Codec> {
    pub moniker: String,
    pub path: PathBuf,
    pub codec: Codec,
}

pub struct State<Ctx: Context> {
    height: Ctx::Height,
    wal_sender: mpsc::Sender<self::thread::WalMsg<Ctx>>,
    _handle: std::thread::JoinHandle<()>,
}

impl<Ctx, Codec> Wal<Ctx, Codec>
where
    Ctx: Context,
    Codec: WalCodec<Ctx>,
{
    async fn handle_msg(
        &self,
        _myself: WalRef<Ctx>,
        msg: Msg<Ctx>,
        state: &mut State<Ctx>,
    ) -> Result<(), ActorProcessingErr> {
        match msg {
            Msg::StartedHeight(height, reply_to) => {
                if state.height == height {
                    debug!(%height, "WAL already at height, ignoring");
                    return Ok(());
                }

                state.height = height;

                self.started_height(state, height, reply_to).await?;
            }

            Msg::Append(height, entry, reply_to) => {
                if height != state.height {
                    debug!("Ignoring append at height {} != {}", height, state.height);
                    return Ok(());
                }

                self.write_log(state, entry, reply_to).await?;
            }

            Msg::Flush(reply_to) => {
                self.flush_log(state, reply_to).await?;
            }
        }

        Ok(())
    }

    async fn started_height(
        &self,
        state: &mut State<Ctx>,
        height: <Ctx as Context>::Height,
        reply_to: WalReply<Option<Vec<WalEntry<Ctx>>>>,
    ) -> Result<(), ActorProcessingErr> {
        let (tx, rx) = oneshot::channel();

        state
            .wal_sender
            .send(self::thread::WalMsg::StartedHeight(height, tx))
            .await?;

        let to_replay = rx
            .await?
            .map(|entries| Some(entries).filter(|entries| !entries.is_empty()));

        reply_to
            .send(to_replay)
            .map_err(|e| eyre!("Failed to send reply: {e}"))?;

        Ok(())
    }

    async fn write_log(
        &self,
        state: &mut State<Ctx>,
        msg: impl Into<WalEntry<Ctx>>,
        reply_to: WalReply<()>,
    ) -> Result<(), ActorProcessingErr> {
        let entry = msg.into();
        let (tx, rx) = oneshot::channel();

        state
            .wal_sender
            .send(self::thread::WalMsg::Append(entry, tx))
            .await?;

        let result = rx.await?;

        reply_to
            .send(result)
            .map_err(|e| eyre!("Failed to send reply: {e}"))?;

        Ok(())
    }

    async fn flush_log(
        &self,
        state: &mut State<Ctx>,
        reply_to: WalReply<()>,
    ) -> Result<(), ActorProcessingErr> {
        let (tx, rx) = oneshot::channel();

        state
            .wal_sender
            .send(self::thread::WalMsg::Flush(tx))
            .await?;

        let result = rx.await?;

        reply_to
            .send(result)
            .map_err(|e| eyre!("Failed to send reply: {e}"))?;

        Ok(())
    }
}

#[async_trait]
impl<Ctx, Codec> Actor for Wal<Ctx, Codec>
where
    Ctx: Context,
    Codec: WalCodec<Ctx>,
{
    type Msg = Msg<Ctx>;
    type Arguments = Args<Codec>;
    type State = State<Ctx>;

    async fn pre_start(
        &self,
        _myself: WalRef<Ctx>,
        args: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        let log = wal::Log::open(&args.path)?;
        info!("Opened WAL at {}", args.path.display());

        let (tx, rx) = mpsc::channel(100);

        // Spawn a system thread to perform blocking WAL operations.
        let handle = self::thread::spawn(args.moniker, log, args.codec, rx);

        Ok(State {
            height: Ctx::Height::default(),
            wal_sender: tx,
            _handle: handle,
        })
    }

    async fn handle(
        &self,
        myself: WalRef<Ctx>,
        msg: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        if let Err(e) = self.handle_msg(myself, msg, state).await {
            error!("Failed to handle WAL message: {e}");
        }

        Ok(())
    }

    async fn post_stop(
        &self,
        _: WalRef<Ctx>,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        let _ = state.wal_sender.send(self::thread::WalMsg::Shutdown).await;

        Ok(())
    }
}