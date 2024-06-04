use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Uint128, Uint256};
use cw721::Cw721ReceiveMsg;

use crate::interface::AssetInfo;

#[cw_serde]
pub struct InstantiateMsg {
    pub nft_address: Addr,
    pub token_0: AssetInfo,
    pub token_1: AssetInfo,
    pub fee: u16,
    pub tick_spacing: u32,
}

#[cw_serde]
pub enum ExecuteMsg {
    Mint {
        recipient: Addr,
        tick_lower: i32,
        tick_upper: i32,
        lp_amount: i128,
    },
    ReceiveNft(Cw721ReceiveMsg),
    Collect {
        token_ids: Vec<String>,
    },
    Swap {
        recipient: Addr,
        is_base_input: bool,
        zero_for_one: bool,
        amount_specified: u64,
        sqrt_price_limit_x64: u128,
    },
}

#[cw_serde]
pub enum Cw721HookMsg {
    Burn {},
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {}
