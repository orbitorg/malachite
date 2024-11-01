use std::path::Path;
use std::sync::Arc;

use prost::Message;
use redb::ReadableTable;
use thiserror::Error;

use malachite_blocksync::SyncedBlock;
use malachite_common::Value;
use malachite_common::{Certificate, Proposal, SignedProposal, SignedVote};
use malachite_proto::{Error as ProtoError, Protobuf};
use malachite_starknet_p2p_proto as proto;
use malachite_starknet_p2p_types::{Block, Height, Transaction, Transactions};

use crate::codec::{decode_sync_block, encode_synced_block};
use crate::mock::context::MockContext;

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("Database error")]
    Database(#[from] redb::DatabaseError),

    #[error("Storage error")]
    Storage(#[from] redb::StorageError),

    #[error("Table error")]
    Table(#[from] redb::TableError),

    #[error("Transaction error")]
    Transaction(#[from] redb::TransactionError),

    #[error("Commit error")]
    Commit(#[from] redb::CommitError),

    #[error("Protobuf error")]
    Proto(#[from] ProtoError),

    #[error("Failed to join on task")]
    TaskJoin(#[from] tokio::task::JoinError),
}

#[derive(Clone, Debug)]
pub struct DecidedBlock {
    pub block: Block,
    pub proposal: SignedProposal<MockContext>,
    pub certificate: Certificate<MockContext>,
}

impl DecidedBlock {
    // TODO: Define our own zero-copy Protobuf struct
    fn to_bytes(&self) -> Result<Vec<u8>, ProtoError> {
        let synced_block = SyncedBlock {
            block_bytes: self.block.to_bytes()?,
            proposal: self.proposal.clone(),
            certificate: self.certificate.clone(),
        };

        let proto = encode_synced_block(synced_block)?;
        Ok(proto.encode_to_vec())
    }

    // TODO: Define our own zero-copy Protobuf struct
    fn from_bytes(bytes: &[u8]) -> Result<Self, ProtoError> {
        let synced_block = proto::blocksync::SyncedBlock::decode(bytes)?;
        let synced_block = decode_sync_block(synced_block)?;
        let block = Block::from_bytes(synced_block.block_bytes.as_ref())?;

        Ok(Self {
            block,
            proposal: synced_block.proposal,
            certificate: synced_block.certificate,
        })
    }
}

#[derive(Copy, Clone, Debug)]
struct HeightKey;

impl redb::Value for HeightKey {
    type SelfType<'a> = Height;
    type AsBytes<'a> = Vec<u8>;

    fn fixed_width() -> Option<usize> {
        Some(core::mem::size_of::<u64>() * 2)
    }

    fn from_bytes<'a>(data: &'a [u8]) -> Self::SelfType<'a>
    where
        Self: 'a,
    {
        let (fork_id, block_number) = <(u64, u64) as redb::Value>::from_bytes(data);

        Height {
            fork_id,
            block_number,
        }
    }

    fn as_bytes<'a, 'b: 'a>(value: &'a Self::SelfType<'b>) -> Self::AsBytes<'a>
    where
        Self: 'a,
        Self: 'b,
    {
        <(u64, u64) as redb::Value>::as_bytes(&(value.fork_id, value.block_number))
    }

    fn type_name() -> redb::TypeName {
        redb::TypeName::new("starknet::Height")
    }
}

impl redb::Key for HeightKey {
    fn compare(data1: &[u8], data2: &[u8]) -> std::cmp::Ordering {
        <(u64, u64) as redb::Key>::compare(data1, data2)
    }
}

const TABLE: redb::TableDefinition<HeightKey, Vec<u8>> = redb::TableDefinition::new("blocks");

struct Db {
    db: redb::Database,
}

impl Db {
    fn new(path: impl AsRef<Path>) -> Result<Self, StoreError> {
        Ok(Self {
            db: redb::Database::create(path)?,
        })
    }

    fn get(&self, height: Height) -> Result<Option<DecidedBlock>, StoreError> {
        let tx = self.db.begin_read()?;
        let table = tx.open_table(TABLE)?;
        let value = table.get(&height)?;
        let block = value
            .map(|value| DecidedBlock::from_bytes(&value.value()))
            .transpose()?;
        Ok(block)
    }

    fn insert(&self, decided_block: DecidedBlock) -> Result<(), StoreError> {
        let height = decided_block.block.height;

        let tx = self.db.begin_write()?;
        {
            let mut table = tx.open_table(TABLE)?;
            table.insert(height, decided_block.to_bytes()?)?;
        }
        tx.commit()?;
        Ok(())
    }

    fn prune(&self, retain_height: Height) -> Result<(), StoreError> {
        let tx = self.db.begin_write()?;
        {
            let mut table = tx.open_table(TABLE)?;
            table.retain(|key, _| key >= retain_height)?;
        }
        tx.commit()?;
        Ok(())
    }

    fn first_key(&self) -> Option<Height> {
        let tx = self.db.begin_read().ok()?;
        let table = tx.open_table(TABLE).ok()?;
        let (key, _) = table.first().ok()??;
        Some(key.value())
    }

    fn last_key(&self) -> Option<Height> {
        let tx = self.db.begin_read().ok()?;
        let table = tx.open_table(TABLE).ok()?;
        let (key, _) = table.last().ok()??;
        Some(key.value())
    }
}

#[derive(Clone)]
pub struct BlockStore {
    db: Arc<Db>,
}

impl BlockStore {
    pub fn new(path: impl AsRef<Path>) -> Result<Self, StoreError> {
        Ok(Self {
            db: Arc::new(Db::new(path)?),
        })
    }

    pub fn first_height(&self) -> Option<Height> {
        self.db.first_key()
    }

    pub fn last_height(&self) -> Option<Height> {
        self.db.last_key()
    }

    pub async fn get(&self, height: Height) -> Result<Option<DecidedBlock>, StoreError> {
        let db = Arc::clone(&self.db);
        tokio::task::spawn_blocking(move || db.get(height)).await?
    }

    pub async fn store(
        &self,
        proposal: &SignedProposal<MockContext>,
        txes: &[Transaction],
        commits: &[SignedVote<MockContext>],
    ) -> Result<(), StoreError> {
        let block_id = proposal.value().id();

        let certificate = Certificate {
            commits: commits.to_vec(),
        };

        let decided_block = DecidedBlock {
            block: Block {
                height: proposal.height(),
                block_hash: block_id,
                transactions: Transactions::new(txes.to_vec()),
            },
            proposal: proposal.clone(),
            certificate,
        };

        let db = Arc::clone(&self.db);
        tokio::task::spawn_blocking(move || db.insert(decided_block)).await?
    }

    pub async fn prune(&self, retain_height: Height) -> Result<(), StoreError> {
        let db = Arc::clone(&self.db);
        tokio::task::spawn_blocking(move || db.prune(retain_height)).await?
    }
}
