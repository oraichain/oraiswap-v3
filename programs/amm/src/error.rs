// use anchor_lang::prelude::*;
use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),
    #[error("LOK")]
    LOK,
    #[error("Not approved")]
    NotApproved,
    #[error("invalid update amm config flag")]
    InvalidUpdateConfigFlag,
    #[error("Account lack")]
    AccountLack,
    #[error("Remove liquitity, collect fees owed and reward then you can close position account")]
    ClosePositionErr,

    #[error("Minting amount should be greater than 0")]
    ZeroMintAmount,

    #[error("Tick out of range")]
    InvaildTickIndex,
    #[error("The lower tick must be below the upper tick")]
    TickInvaildOrder,
    #[error("The tick must be greater, or equal to the minimum tick(-221818)")]
    TickLowerOverflow,
    #[error("The tick must be lesser than, or equal to the maximum tick(221818)")]
    TickUpperOverflow,
    #[error("tick % tick_spacing must be zero")]
    TickAndSpacingNotMatch,
    #[error("Invaild tick array account")]
    InvalidTickArray,
    #[error("Invaild tick array boundary")]
    InvalidTickArrayBoundary,

    #[error("Square root price limit overflow")]
    SqrtPriceLimitOverflow,
    // second inequality must be < because the price can never reach the price at the max tick
    #[error("sqrt_price_x64 out of range")]
    SqrtPriceX64,

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

    /// swap errors
    // Non fungible position manager
    #[error("Transaction too old")]
    TransactionTooOld,
    #[error("Price slippage check")]
    PriceSlippageCheck,
    #[error("Too little output received")]
    TooLittleOutputReceived,
    #[error("Too much input paid")]
    TooMuchInputPaid,
    #[error("Swap special amount can not be zero")]
    InvaildSwapAmountSpecified,
    #[error("Input pool vault is invalid")]
    InvalidInputPoolVault,
    #[error("Swap input or output amount is too small")]
    TooSmallInputOrOutputAmount,
    #[error("Not enought tick array account")]
    NotEnoughTickArrayAccount,
    #[error("Invaild first tick array account")]
    InvalidFirstTickArrayAccount,

    /// reward errors
    #[error("Invalid reward index")]
    InvalidRewardIndex,
    #[error("The init reward token reach to the max")]
    FullRewardInfo,
    #[error("The init reward token already in use")]
    RewardTokenAlreadyInUse,
    #[error("The reward tokens must contain one of pool vault mint except the last reward")]
    ExceptPoolVaultMint,
    #[error("Invalid reward init param")]
    InvalidRewardInitParam,
    #[error("Invalid collect reward desired amount")]
    InvalidRewardDesiredAmount,
    #[error("Invalid collect reward input account number")]
    InvalidRewardInputAccountNumber,
    #[error("Invalid reward period")]
    InvalidRewardPeriod,
    #[error(
        "Modification of emissiones is allowed within 72 hours from the end of the previous cycle"
    )]
    NotApproveUpdateRewardEmissiones,
    #[error("uninitialized reward info")]
    UnInitializedRewardInfo,

    #[error("Not support token_2022 mint extension")]
    NotSupportMint,
    #[error("Missing tickarray bitmap extension account")]
    MissingTickArrayBitmapExtensionAccount,
    #[error("Insufficient liquidity for this direction")]
    InsufficientLiquidityForDirection,

    #[error("SqrtPrice Lower Than Min")]
    SplM,
    #[error("SqrtPrice Lower Than Current")]
    SplC,
    #[error("SqrtPrice Upper Than Max")]
    SpuM,
    #[error("SqrtPrice Upper Than Current")]
    SpuC,
}
