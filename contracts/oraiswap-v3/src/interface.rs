use cosmwasm_schema::cw_serde;
use cosmwasm_std::{to_binary, Addr, Api, BankMsg, Coin, CosmosMsg, MessageInfo, Uint128, WasmMsg};
use cw20::Cw20ExecuteMsg;

use crate::{sqrt_price::SqrtPrice, token_amount::TokenAmount, ContractError, Pool, PoolKey, Tick};

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
