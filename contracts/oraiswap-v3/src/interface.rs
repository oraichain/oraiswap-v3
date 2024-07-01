use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    to_binary, Addr, Api, BankMsg, Binary, Coin, CosmosMsg, MessageInfo, StdResult, Uint128,
    WasmMsg,
};
use cw20::{Cw20ExecuteMsg, Expiration};

use crate::{
    fee_growth::FeeGrowth, sqrt_price::SqrtPrice, token_amount::TokenAmount, ContractError, Pool,
    PoolKey, Position, Tick,
};

#[cw_serde]
pub struct CalculateSwapResult {
    pub amount_in: TokenAmount,
    pub amount_out: TokenAmount,
    pub start_sqrt_price: SqrtPrice,
    pub target_sqrt_price: SqrtPrice,
    pub fee: TokenAmount,
    pub pool: Pool,
    pub ticks: Vec<Tick>,
}

#[cw_serde]
pub struct SwapHop {
    pub pool_key: PoolKey,
    pub x_to_y: bool,
}

/// AssetInfo contract_addr is usually passed from the cw20 hook
/// so we can trust the contract_addr is properly validated.
#[cw_serde]
pub enum AssetInfo {
    Token { contract_addr: Addr },
    NativeToken { denom: String },
}

impl AssetInfo {
    pub fn from_denom(api: &dyn Api, denom: &str) -> Self {
        if let Ok(contract_addr) = api.addr_validate(denom) {
            Self::Token { contract_addr }
        } else {
            Self::NativeToken {
                denom: denom.to_string(),
            }
        }
    }
}

#[cw_serde]
pub struct Asset {
    pub info: AssetInfo,
    pub amount: Uint128,
}

impl Asset {
    pub fn transfer(
        &self,
        msgs: &mut Vec<CosmosMsg>,
        info: &MessageInfo,
    ) -> Result<(), ContractError> {
        if !self.amount.is_zero() {
            match &self.info {
                AssetInfo::Token { contract_addr } => {
                    msgs.push(
                        WasmMsg::Execute {
                            contract_addr: contract_addr.to_string(),
                            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                                recipient: info.sender.to_string(),
                                amount: self.amount,
                            })?,
                            funds: vec![],
                        }
                        .into(),
                    );
                }
                AssetInfo::NativeToken { denom } => msgs.push(
                    BankMsg::Send {
                        to_address: info.sender.to_string(),
                        amount: vec![Coin {
                            amount: self.amount,
                            denom: denom.to_string(),
                        }],
                    }
                    .into(),
                ),
            }
        }
        Ok(())
    }

    pub fn transfer_from(
        &self,
        msgs: &mut Vec<CosmosMsg>,
        info: &MessageInfo,
        recipient: String,
    ) -> Result<(), ContractError> {
        if !self.amount.is_zero() {
            match &self.info {
                AssetInfo::Token { contract_addr } => {
                    msgs.push(
                        WasmMsg::Execute {
                            contract_addr: contract_addr.to_string(),
                            msg: to_binary(&Cw20ExecuteMsg::TransferFrom {
                                owner: info.sender.to_string(),
                                recipient,
                                amount: self.amount,
                            })?,
                            funds: vec![],
                        }
                        .into(),
                    );
                }
                AssetInfo::NativeToken { denom } => {
                    match info.funds.iter().find(|x| x.denom.eq(denom)) {
                        Some(coin) => {
                            if coin.amount >= self.amount {
                                let refund_amount = coin.amount - self.amount;
                                // refund for user
                                if !refund_amount.is_zero() {
                                    msgs.push(
                                        BankMsg::Send {
                                            to_address: info.sender.to_string(),
                                            amount: vec![Coin {
                                                amount: refund_amount,
                                                denom: denom.to_string(),
                                            }],
                                        }
                                        .into(),
                                    )
                                }
                            } else {
                                return Err(ContractError::InvalidFunds {
                                    transfer_amount: self.amount,
                                });
                            }
                        }
                        None => {
                            return Err(ContractError::InvalidFunds {
                                transfer_amount: self.amount,
                            });
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

#[cw_serde]
pub struct Approval {
    /// Account that can transfer/send the token
    pub spender: Addr,
    /// When the Approval expires (maybe Expiration::never)
    pub expires: Expiration,
}

#[cw_serde]
pub struct PositionTick {
    pub index: i32,
    pub fee_growth_outside_x: FeeGrowth,
    pub fee_growth_outside_y: FeeGrowth,
    pub seconds_outside: u64,
}

#[cw_serde]
pub struct PoolWithPoolKey {
    pub pool: Pool,
    pub pool_key: PoolKey,
}

#[cw_serde]
pub struct QuoteResult {
    pub amount_in: TokenAmount,
    pub amount_out: TokenAmount,
    pub target_sqrt_price: SqrtPrice,
    pub ticks: Vec<Tick>,
}

#[cw_serde]
pub struct TokensResponse {
    /// Contains all token_ids in lexicographical ordering
    /// If there are more than `limit`, use `start_from` in future queries
    /// to achieve pagination.
    pub tokens: Vec<Binary>,
}

#[cw_serde]
pub struct OwnerOfResponse {
    /// Owner of the token
    pub owner: Addr,
    /// If set this address is approved to transfer/send the token as well
    pub approvals: Vec<Approval>,
}

#[cw_serde]
pub struct ApprovedForAllResponse {
    pub operators: Vec<Approval>,
}

#[cw_serde]
pub struct AllNftInfoResponse {
    /// Who can transfer the token
    pub access: OwnerOfResponse,
    /// Data on the token itself,
    pub info: Position,
}

/// Cw721ReceiveMsg should be de/serialized under `Receive()` variant in a HandleMsg
#[cw_serde]
pub struct Cw721ReceiveMsg {
    pub sender: Addr,
    pub token_id: Binary,
    pub msg: Option<Binary>,
}

impl Cw721ReceiveMsg {
    /// serializes the message
    pub fn into_binary(self) -> StdResult<Binary> {
        let msg = ReceiverHandleMsg::ReceiveNft(self);
        to_binary(&msg)
    }

    /// creates a cosmos_msg sending this struct to the named contract
    pub fn into_cosmos_msg(self, contract_addr: String) -> StdResult<CosmosMsg> {
        let msg = self.into_binary()?;
        let execute = WasmMsg::Execute {
            contract_addr,
            msg,
            funds: vec![],
        };
        Ok(execute.into())
    }
}

/// This is just a helper to properly serialize the above message.
/// The actual receiver should include this variant in the larger HandleMsg enum
#[cw_serde]
enum ReceiverHandleMsg {
    ReceiveNft(Cw721ReceiveMsg),
}
