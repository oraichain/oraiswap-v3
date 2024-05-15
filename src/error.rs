use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Invalid funds")]
    InvalidFunds {},

    #[error("Invalid ticks")]
    InvalidTicks {},

    #[error("Zero lp amount")]
    ZeroLpAmount {},

    #[error("Invalid sqrt price limit x96")]
    InvalidSqrtPriceLimitX96 {},

    #[error("Invalid sqrt price limit")]
    InvalidSqrtPrice {},

    #[error("Zero amount")]
    ZeroAmount {},

    #[error("Invalid price limit")]
    InvalidPriceLimit {},

    #[error("Can't find the next tick")]
    NoNextTick {},

    #[error("Cannot compute swap step")]
    CannotComputeSwapStep {},
    // Add any other custom errors you like here.
    // Look at https://docs.rs/thiserror/1.0.21/thiserror/ for details.
}
