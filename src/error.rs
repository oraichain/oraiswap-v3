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

    #[error("Square root price limit overflow")]
    SqrtPriceLimitOverflow,
    // second inequality must be < because the price can never reach the price at the max tick
    #[error("sqrt_price_x64 out of range")]
    SqrtPriceX64,

    #[error("Tick out of range")]
    InvaildTickIndex,
    #[error("The lower tick must be below the upper tick")]
    TickInvaildOrder,
    #[error("The tick must be greater, or equal to the minimum tick(-221818)")]
    TickLowerOverflow,
    #[error("The tick must be lesser than, or equal to the maximum tick(221818)")]
    TickUpperOverflow,

    // Liquidity Sub
    #[error("Liquidity sub delta L must be smaller than before")]
    LiquiditySubValueErr,
    // Liquidity Add
    #[error("Liquidity add delta L must be greater, or equal to before")]
    LiquidityAddValueErr,
    #[error("Invaild liquidity when update position")]
    InvaildLiquidity,
    #[error("Both token amount must not be zero while supply liquidity")]
    ForbidBothZeroForSupplyLiquidity,
    #[error("Liquidity insufficient")]
    LiquidityInsufficient,
    // Add any other custom errors you like here.
    // Look at https://docs.rs/thiserror/1.0.21/thiserror/ for details.
}
