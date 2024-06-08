use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("invalid tick spacing")]
    InvalidTickSpacing,

    #[error("invalid fee")]
    InvalidFee,

    #[error("invalid tick index")]
    InvalidTickIndex,

    #[error("invalid tick")]
    InvalidTick,

    #[error("tokens are the same")]
    TokensAreSame,

    #[error("invalid tick")]
    InvalidInitTick,

    #[error("invalid sqrt price")]
    InvalidInitSqrtPrice,

    #[error("invalid offset")]
    InvalidOffset,

    #[error("multiplication overflow")]
    Mul,

    #[error("division overflow or division by zero")]
    Div,

    #[error("type failed")]
    Cast,

    #[error("addition overflow")]
    Add,

    #[error("subtraction underflow")]
    Sub,

    #[error("update_liquidity: liquidity + liquidity_delta overflow")]
    UpdateLiquidityPlusOverflow,

    #[error("update_liquidity: liquidity - liquidity_delta underflow")]
    UpdateLiquidityMinusOverflow,

    #[error("empty position pokes")]
    EmptyPositionPokes,

    #[error("position not found")]
    PositionNotFound,

    #[error("position add liquidity overflow")]
    PositionAddLiquidityOverflow,

    #[error("position remove liquidity underflow")]
    PositionRemoveLiquidityUnderflow,

    #[error("price limit reached")]
    PriceLimitReached,

    #[error("insufficient liquidity")]
    InsufficientLiquidity,

    #[error("current_timestamp - pool.start_timestamp underflow")]
    TimestampSubOverflow,

    #[error("pool not found")]
    PoolNotFound,

    #[error("pool.liquidity + tick.liquidity_change overflow")]
    PoolAddTickLiquidityOverflow,

    #[error("pool.liquidity - tick.liquidity_change underflow")]
    PoolSubTickLiquidityUnderflow,

    #[error("tick limit reached")]
    TickLimitReached,

    #[error("tick not found")]
    TickNotFound,

    #[error("tick already exist")]
    TickAlreadyExist,

    #[error("tick add liquidity overflow")]
    TickAddLiquidityOverflow,

    #[error("tick remove liquidity underflow")]
    TickRemoveLiquidityUnderflow,

    #[error("Invalid tick liquidity")]
    InvalidTickLiquidity,

    #[error("sqrt_price out of range")]
    SqrtPriceOutOfRange,

    #[error("current_timestamp > last_timestamp failed")]
    TimestampCheckFailed,

    #[error("can not parse from u320 to u256")]
    U320ToU256,

    #[error("tick over bounds")]
    TickOverBounds,

    #[error("calculate_sqrt_price: parsing from scale failed")]
    ParseFromScale,

    #[error("calcaule_sqrt_price::checked_div division failed")]
    CheckedDiv,

    #[error("calculate_sqrt_price: parsing scale failed")]
    ParseScale,

    #[error("extending liquidity overflow")]
    ExtendLiquidityOverflow,

    #[error("big_liquidity -/+ sqrt_price * x")]
    BigLiquidityOverflow,

    #[error("upper_tick is not greater than lower_tick")]
    UpperTickNotGreater,

    #[error("tick_lower > tick_upper")]
    TickLowerGreater,

    #[error("tick initialize tick again")]
    TickReInitialize,

    #[error("Upper Sqrt Price < Current Sqrt Price")]
    UpperSqrtPriceLess,

    #[error("overflow in calculating liquidity")]
    OverflowInCalculatingLiquidity,

    #[error("Current Sqrt Price < Lower Sqrt Price")]
    CurrentSqrtPriceLess,

    #[error("overflow while casting to TokenAmount")]
    OverflowCastingTokenAmount,

    #[error("unauthorized")]
    Unauthorized {},

    #[error("amount is zero")]
    AmountIsZero,

    #[error("wrong limit")]
    WrongLimit,

    #[error("no gain swap")]
    NoGainSwap,

    #[error("swap failed")]
    SwapFailed,

    #[error("amount under minimum amount out")]
    AmountUnderMinimumAmountOut,

    #[error("invalid pool key")]
    InvalidPoolKey,

    #[error("pool already exist")]
    PoolAlreadyExist,

    #[error("pool not created")]
    CreatePoolError,

    #[error("FeeTierNotFound")]
    FeeTierNotFound,
}

impl From<ContractError> for StdError {
    fn from(source: ContractError) -> Self {
        Self::generic_err(source.to_string())
    }
}
