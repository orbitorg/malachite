use crate::context::address::BaseAddress;
use crate::context::height::BaseHeight;
use crate::context::value::BaseValue;

#[derive(Debug)]
pub struct Decision {
    pub peer: BaseAddress,
    pub value: BaseValue,
    pub height: BaseHeight,
}
