use std::path::Path;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use malachite_common::Round;
use malachite_proto::{Error as ProtoError, Protobuf};
use malachite_starknet_p2p_types::{Height, ProposalPart};

// TODO:
// - [ ] Add Address to key
//
// NOTE: Not sure if this is required as consensus should verify that only the parts signed by the proposer for
//       the height and round should be forwarded here (see the TODOs in consensus)

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("Database error: {0}")]
    Database(#[from] redb::DatabaseError),

    #[error("Storage error: {0}")]
    Storage(#[from] redb::StorageError),

    #[error("Table error: {0}")]
    Table(#[from] redb::TableError),

    #[error("Transaction error: {0}")]
    Transaction(#[from] redb::TransactionError),

    #[error("Commit error: {0}")]
    Commit(#[from] redb::CommitError),

    #[error("Protobuf error: {0}")]
    Proto(#[from] ProtoError),

    #[error("Bincode error: {0}")]
    Bincode(#[from] bincode::Error),

    #[error("Failed to join on task: {0}")]
    TaskJoin(#[from] tokio::task::JoinError),
}

#[derive(Debug)]
struct IndexKey;

impl redb::Value for IndexKey {
    type SelfType<'a> = (Height, Round);
    type AsBytes<'a> = Vec<u8>;

    fn fixed_width() -> Option<usize> {
        <(u64, u64, i64) as redb::Value>::fixed_width()
    }

    fn from_bytes<'a>(data: &'a [u8]) -> Self::SelfType<'a>
    where
        Self: 'a,
    {
        let (fork_id, block_number, round) = <(u64, u64, i64) as redb::Value>::from_bytes(data);
        (Height::new(fork_id, block_number), Round::from(round))
    }

    fn as_bytes<'a, 'b: 'a>((height, round): &'a Self::SelfType<'b>) -> Self::AsBytes<'a>
    where
        Self: 'a,
        Self: 'b,
    {
        <(u64, u64, i64) as redb::Value>::as_bytes(&(
            height.fork_id,
            height.block_number,
            round.as_i64(),
        ))
    }

    fn type_name() -> redb::TypeName {
        redb::TypeName::new("starknet::IndexKey")
    }
}

impl redb::Key for IndexKey {
    fn compare(data1: &[u8], data2: &[u8]) -> std::cmp::Ordering {
        <(u64, u64, i64) as redb::Key>::compare(data1, data2)
    }
}

type PartKey = u64;

const INDEX_TABLE: redb::TableDefinition<IndexKey, PartKeys> =
    redb::TableDefinition::new("parts_index");

const PARTS_TABLE: redb::TableDefinition<PartKey, Vec<u8>> = redb::TableDefinition::new("parts");

pub type Sequence = u64;

#[derive(Debug, Serialize, Deserialize)]
struct PartKeys(Vec<PartKey>);

impl PartKeys {
    fn new() -> Self {
        Self(Vec::new())
    }

    fn next_key(&self) -> PartKey {
        self.0.iter().max().map_or(1, |&key| key + 1)
    }

    fn push(&mut self, key: PartKey) {
        self.0.push(key);
    }

    fn iter(&self) -> impl Iterator<Item = &PartKey> {
        self.0.iter()
    }
}

impl redb::Value for PartKeys {
    type SelfType<'a> = PartKeys;
    type AsBytes<'a> = Vec<u8>;

    fn fixed_width() -> Option<usize> {
        None
    }

    fn from_bytes<'a>(data: &'a [u8]) -> Self::SelfType<'a>
    where
        Self: 'a,
    {
        bincode::deserialize(data).expect("failed to deserialize PartKeys")
    }

    fn as_bytes<'a, 'b: 'a>(value: &'a Self::SelfType<'b>) -> Self::AsBytes<'a>
    where
        Self: 'a,
        Self: 'b,
    {
        bincode::serialize(value).expect("failed to serialize PartKeys")
    }

    fn type_name() -> redb::TypeName {
        redb::TypeName::new("starknet::PartKeys")
    }
}

struct Db {
    db: redb::Database,
}

impl Db {
    fn new(path: impl AsRef<Path>) -> Result<Self, StoreError> {
        let db = Self {
            db: redb::Database::create(path)?,
        };
        db.create_tables()?;
        Ok(db)
    }

    fn create_tables(&self) -> Result<(), StoreError> {
        let tx = self.db.begin_write()?;
        tx.open_table(INDEX_TABLE)?;
        tx.open_table(PARTS_TABLE)?;
        tx.commit()?;
        Ok(())
    }

    pub fn insert(
        &self,
        height: Height,
        round: Round,
        proposal_part: ProposalPart,
    ) -> Result<(), StoreError> {
        // Fetch the part keys for this height and round from the `parts_index` table
        let mut part_keys = {
            let tx = self.db.begin_read()?;
            let table = tx.open_table(INDEX_TABLE)?;

            match table.get(&(height, round))? {
                None => PartKeys::new(),
                Some(value) => value.value(),
            }
        };

        // Compute the next part key
        let next_key = part_keys.next_key();

        // Add it to the list of part keys for this height and round
        part_keys.push(next_key);

        // Open a write transaction to update both the `parts_index` and `parts` tables` atomically
        let tx = self.db.begin_write()?;

        // Store the part keys for this height and round in the `parts_index` table
        {
            let mut table = tx.open_table(INDEX_TABLE)?;
            table.insert((height, round), part_keys)?;
        }

        // Store the proposal part in the `parts` table
        {
            let bytes = proposal_part.to_bytes()?;
            let mut table = tx.open_table(PARTS_TABLE)?;
            table.insert(next_key, bytes.to_vec())?;
        }

        // Commit both writes atomically
        tx.commit()?;

        Ok(())
    }

    pub fn get(&self, height: Height, round: Round) -> Result<Vec<ProposalPart>, StoreError> {
        // Open a read transaction to fetch both the parts keys and the parts themselves
        let tx = self.db.begin_read()?;

        // Fetch the part keys for this height and round from the `parts_index` table
        let part_keys = {
            let table = tx.open_table(INDEX_TABLE)?;
            match table.get(&(height, round))? {
                // If there are no part keys in the index, abort early
                None => return Ok(Vec::new()),
                Some(value) => value.value(),
            }
        };

        let mut parts = Vec::new();

        // Fetch the parts themselves from the `parts` table
        let table = tx.open_table(PARTS_TABLE)?;
        for key in part_keys.iter() {
            if let Some(value) = table.get(key)? {
                let part = ProposalPart::from_bytes(&value.value())?;
                parts.push(part);
            }
        }

        Ok(parts)
    }

    pub fn prune(&self, retain_height: Height) -> Result<(), StoreError> {
        // Open a write transaction to prune both the `parts_index` and `parts` tables
        let tx = self.db.begin_write()?;

        // Get the index key and part keys of all parts under `retain_height`
        let mut all_part_keys = Vec::new();
        let mut index_keys = Vec::new();

        {
            use redb::ReadableTable;
            let table = tx.open_table(INDEX_TABLE)?;
            for (key, part_keys) in table.iter()?.flatten() {
                let (height, round) = key.value();
                if height < retain_height {
                    all_part_keys.push(part_keys.value());
                    index_keys.push((height, round));
                }
            }
        }

        // Remove the keys from the `parts_index` table
        {
            let mut table = tx.open_table(INDEX_TABLE)?;
            for index_key in index_keys {
                table.remove(index_key)?;
            }
        }

        // Remove all parts with corresponding keys from the `parts` table
        {
            let mut table = tx.open_table(PARTS_TABLE)?;
            for part_keys in all_part_keys {
                for part_key in part_keys.iter() {
                    table.remove(part_key)?;
                }
            }
        }

        // Commit both prunes atomically
        tx.commit()?;

        Ok(())
    }
}

#[derive(Clone)]
pub struct PartStore {
    db: Arc<Db>,
}

impl PartStore {
    pub fn new(path: impl AsRef<Path>) -> Result<Self, StoreError> {
        Ok(Self {
            db: Arc::new(Db::new(path)?),
        })
    }

    /// Return all the parts for the given height and round, sorted by sequence in ascending order
    pub fn all_parts(&self, height: Height, round: Round) -> Result<Vec<ProposalPart>, StoreError> {
        self.db.get(height, round)
    }

    pub fn store(
        &self,
        height: Height,
        round: Round,
        proposal_part: ProposalPart,
    ) -> Result<(), StoreError> {
        self.db.insert(height, round, proposal_part)
    }

    pub fn prune(&self, retain_height: Height) -> Result<(), StoreError> {
        self.db.prune(retain_height)
    }

    pub fn is_empty(&self) -> bool {
        true // FIXME: Implement this
    }

    pub fn len(&self) -> usize {
        0 // FIXME: Implement this
    }
}
