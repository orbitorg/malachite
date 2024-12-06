use crate::context::address::BaseAddress;
use crate::context::height::BaseHeight;
use crate::context::value::BaseValueId;

#[derive(Debug)]
pub struct Decision {
    pub peer: BaseAddress,
    pub value_id: BaseValueId,
    pub height: BaseHeight,
}
