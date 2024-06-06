use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Uint128};

#[cw_serde]
pub struct Asset {
    pub info: AssetInfo,
    pub amount: Uint128,
}

#[cw_serde]
pub enum AssetInfo {
    Token { contract_addr: Addr },
    NativeToken { denom: String },
}

#[cw_serde]
pub struct NftExtentions {
    pub pool: Addr,
    pub tick_lower: i32,
    pub tick_upper: i32,
}

#[cw_serde]
pub enum Cw721BaseExecuteMsg<T> {
    Mint {
        /// Unique ID of the NFT
        token_id: String,
        /// The owner of the newly minter NFT
        owner: String,
        /// Universal resource identifier for this NFT
        /// Should point to a JSON file that conforms to the ERC721
        /// Metadata JSON Schema
        token_uri: Option<String>,
        /// Any custom extension used by this contract
        extension: T,
    },
}
