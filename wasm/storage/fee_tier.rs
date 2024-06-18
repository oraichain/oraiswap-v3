use crate::alloc::string::ToString;
use crate::errors::SwapError;
use crate::types::percentage::Percentage;
use decimal::*;
use serde::{Deserialize, Serialize};
use tsify::Tsify;
use wasm_bindgen::prelude::*;

#[derive(Default, Debug, Copy, Clone, PartialEq, Serialize, Deserialize, Eq, Hash, Tsify)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub struct FeeTier {
    pub fee: Percentage,
    #[tsify(type = "number")]
    pub tick_spacing: u16,
}

impl FeeTier {
    pub fn new(fee: Percentage, tick_spacing: u16) -> Result<Self, SwapError> {
        if tick_spacing == 0 || tick_spacing > 100 {
            return Err(SwapError::InvalidTickSpacing);
        }

        if fee > Percentage::from_integer(1) {
            return Err(SwapError::InvalidFee);
        }

        Ok(Self { fee, tick_spacing })
    }
}

#[wasm_bindgen(js_name = "newFeeTier")]
pub fn new_fee_tier(fee: Percentage, tick_spacing: u16) -> Result<FeeTier, SwapError> {
    FeeTier::new(fee, tick_spacing)
}
