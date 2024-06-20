use std::convert::Infallible;

use prost_types::Any;
use thiserror::Error;

use prost::{DecodeError, EncodeError, Message};

include!(concat!(env!("OUT_DIR"), "/malachite.rs"));

mod impls;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to decode Protobuf message")]
    Decode(#[from] DecodeError),

    #[error("Failed to encode Protobuf message")]
    Encode(#[from] EncodeError),

    #[error("Unable to decode Protobuf message `{type_url}`: missing field `{field}`")]
    MissingField {
        type_url: String,
        field: &'static str,
    },

    #[error("Unknown message type: `{type_url}`")]
    UnknownMessageType { type_url: String },

    #[error("{0}")]
    Other(String),
}

impl Error {
    pub fn missing_field<N: prost::Name>(field: &'static str) -> Self {
        let type_url = N::full_name();
        Self::MissingField { type_url, field }
    }
}

impl From<String> for Error {
    fn from(s: String) -> Self {
        Self::Other(s)
    }
}

impl From<Infallible> for Error {
    fn from(_: Infallible) -> Self {
        unreachable!()
    }
}

pub trait Protobuf: Sized {
    type Proto: Message + Default;

    fn from_proto(proto: Self::Proto) -> Result<Self, Error>;

    fn to_proto(&self) -> Result<Self::Proto, Error>;

    fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        let proto = Self::Proto::decode(bytes)?;
        let result = Self::from_proto(proto)?;
        Ok(result)
    }

    fn to_bytes(&self) -> Result<Vec<u8>, Error> {
        let proto = self.to_proto()?;
        Ok(proto.encode_to_vec())
    }

    fn from_any(any: &Any) -> Result<Self, Error>
    where
        Self::Proto: prost::Name,
    {
        Self::from_proto(any.to_msg::<Self::Proto>()?)
    }

    fn to_any(&self) -> Result<Any, Error>
    where
        Self::Proto: prost::Name,
    {
        Ok(Any::from_msg(&self.to_proto()?)?)
    }
}
