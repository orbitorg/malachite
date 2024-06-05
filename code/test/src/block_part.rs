use signature::Signer;

use malachite_common::{Round, SignedBlockPart, Transaction};
use malachite_proto::{self as proto};

use crate::{Address, Height, PrivateKey, TestContext, Value};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlockMetadata {
    proof: Vec<u8>,
    value: Value,
}

impl BlockMetadata {
    pub fn new(proof: Vec<u8>, value: Value) -> Self {
        Self { proof, value }
    }

    pub fn value(&self) -> Value {
        self.value
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        proto::Protobuf::to_bytes(self).unwrap()
    }
}

impl proto::Protobuf for BlockMetadata {
    type Proto = proto::BlockMetadata;

    fn from_proto(proto: Self::Proto) -> Result<Self, proto::Error> {
        Ok(Self {
            proof: proto.proof,
            value: Value::from_proto(
                proto
                    .value
                    .ok_or_else(|| proto::Error::missing_field::<Self::Proto>("height"))?,
            )?,
        })
    }

    fn to_proto(&self) -> Result<Self::Proto, proto::Error> {
        Ok(proto::BlockMetadata {
            proof: self.proof.clone(),
            value: Option::from(self.value.to_proto().unwrap()),
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TransactionBatch {
    transactions: Vec<Transaction>,
}

impl TransactionBatch {
    pub fn new(transactions: Vec<Transaction>) -> Self {
        TransactionBatch { transactions }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        proto::Protobuf::to_bytes(self).unwrap()
    }
}

impl proto::Protobuf for TransactionBatch {
    type Proto = proto::TransactionBatch;

    fn from_proto(proto: Self::Proto) -> Result<Self, proto::Error> {
        Ok(Self {
            transactions: proto
                .transactions
                .iter()
                .map(|t| Transaction::from_proto(t.clone()).unwrap())
                .collect(),
        })
    }

    fn to_proto(&self) -> Result<Self::Proto, proto::Error> {
        Ok(proto::TransactionBatch {
            transactions: self
                .transactions
                .iter()
                .map(|t| t.to_proto().unwrap())
                .collect(),
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Content {
    transaction_batch: TransactionBatch,
    block_metadata: Option<BlockMetadata>,
}

impl Content {
    pub fn new(transaction_batch: TransactionBatch, block_metadata: Option<BlockMetadata>) -> Self {
        Self {
            transaction_batch,
            block_metadata,
        }
    }
}

impl proto::Protobuf for Content {
    type Proto = proto::Content;

    fn from_proto(proto: Self::Proto) -> Result<Self, proto::Error> {
        let block_metadata = match proto.metadata {
            Some(meta) => Some(BlockMetadata::from_proto(meta)?),
            None => None,
        };

        Ok(Content {
            transaction_batch: TransactionBatch::from_proto(proto.tx_batch.unwrap())?,
            block_metadata,
        })
    }

    fn to_proto(&self) -> Result<Self::Proto, proto::Error> {
        //TODO fix
        let metadata = match self.block_metadata.clone() {
            Some(meta) => Some(meta.to_proto()?),
            None => None,
        };
        Ok(proto::Content {
            tx_batch: Some(self.transaction_batch.to_proto()?),
            metadata,
        })
    }
}

/// A part of a value for a height, round. Identified in this scope by the sequence.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlockPart {
    height: Height,
    round: Round,
    sequence: u64,
    content: Content,
    validator_address: Address,
}

impl BlockPart {
    pub fn new(
        height: Height,
        round: Round,
        sequence: u64,
        validator_address: Address,
        content: Content,
    ) -> Self {
        Self {
            height,
            round,
            sequence,
            content,
            validator_address,
        }
    }

    pub fn height(&self) -> Height {
        self.height
    }

    pub fn round(&self) -> Round {
        self.round
    }

    pub fn sequence(&self) -> u64 {
        self.sequence
    }

    pub fn validator_address(&self) -> &Address {
        &self.validator_address
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        proto::Protobuf::to_bytes(self).unwrap()
    }

    pub fn signed(self, private_key: &PrivateKey) -> SignedBlockPart<TestContext> {
        let signature = private_key.sign(&self.to_bytes());

        SignedBlockPart {
            block_part: self,
            signature,
        }
    }
    pub fn metadata(&self) -> Option<BlockMetadata> {
        self.content.block_metadata.clone()
    }
}

impl malachite_common::BlockPart<TestContext> for BlockPart {
    fn height(&self) -> Height {
        self.height()
    }

    fn round(&self) -> Round {
        self.round()
    }

    fn sequence(&self) -> u64 {
        self.sequence()
    }

    fn validator_address(&self) -> &Address {
        self.validator_address()
    }
}

impl proto::Protobuf for BlockPart {
    type Proto = proto::BlockPart;

    fn from_proto(proto: Self::Proto) -> Result<Self, proto::Error> {
        Ok(Self {
            height: Height::from_proto(
                proto
                    .height
                    .ok_or_else(|| proto::Error::missing_field::<Self::Proto>("height"))?,
            )?,
            round: Round::from_proto(
                proto
                    .round
                    .ok_or_else(|| proto::Error::missing_field::<Self::Proto>("round"))?,
            )?,
            sequence: proto.sequence,
            content: Content::from_proto(
                proto
                    .content
                    .ok_or_else(|| proto::Error::missing_field::<Self::Proto>("content"))?,
            )?,
            validator_address: Address::from_proto(
                proto.validator_address.ok_or_else(|| {
                    proto::Error::missing_field::<Self::Proto>("validator_address")
                })?,
            )?,
        })
    }

    fn to_proto(&self) -> Result<Self::Proto, proto::Error> {
        Ok(proto::BlockPart {
            height: Some(self.height.to_proto()?),
            round: Some(self.round.to_proto()?),
            sequence: self.sequence,
            content: Some(self.content.to_proto()?),
            validator_address: Some(self.validator_address.to_proto()?),
        })
    }
}