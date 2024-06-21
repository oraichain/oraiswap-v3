use decimal::*;

use crate::{
    get_tick_at_sqrt_price,
    liquidity::Liquidity,
    percentage::Percentage,
    sqrt_price::SqrtPrice,
    tests::helper::{macros::*, MockApp},
    token_amount::TokenAmount,
    FeeTier, PoolKey, MIN_SQRT_PRICE,
};

#[test]
fn max_tick_cross() {
    let mut app = MockApp::new(&[("alice", &[])]);
    let (dex, token_x, token_y) = init_dex_and_tokens!(app);
    init_basic_pool!(app, dex, token_x, token_y);

    let mint_amount = u128::MAX;

    approve!(app, token_x, dex, mint_amount, "alice").unwrap();
    approve!(app, token_y, dex, mint_amount, "alice").unwrap();

    let liquidity = Liquidity::from_integer(10000000);

    let fee_tier = FeeTier::new(Percentage::from_scale(6, 3), 10).unwrap();

    let pool_key = PoolKey::new(token_x.to_string(), token_y.to_string(), fee_tier).unwrap();

    for i in (-2560..20).step_by(10) {
        let pool = get_pool!(app, dex, token_x, token_y, fee_tier).unwrap();

        let slippage_limit_lower = pool.sqrt_price;
        let slippage_limit_upper = pool.sqrt_price;

        create_position!(
            app,
            dex,
            pool_key,
            i,
            i + 10,
            liquidity,
            slippage_limit_lower,
            slippage_limit_upper,
            "alice"
        )
        .unwrap();
    }

    let pool = get_pool!(app, dex, token_x, token_y, fee_tier).unwrap();
    assert_eq!(pool.liquidity, liquidity);

    let amount = 760_000;

    mint!(app, token_x, "bob", amount, "alice").unwrap();
    let amount_x = balance_of!(app, token_x, "bob");
    assert_eq!(amount_x, amount);
    approve!(app, token_x, dex, amount, "bob").unwrap();

    let pool_before = get_pool!(app, dex, token_x, token_y, pool_key.fee_tier).unwrap();

    let swap_amount = TokenAmount::new(amount);
    let slippage = SqrtPrice::new(MIN_SQRT_PRICE);
    let quote_result = quote!(app, dex, pool_key, true, swap_amount, true, slippage).unwrap();

    let pool_after_quote = get_pool!(app, dex, token_x, token_y, pool_key.fee_tier).unwrap();

    let crosses_after_quote =
        ((pool_after_quote.current_tick_index - pool_before.current_tick_index) / 10).abs();
    assert_eq!(crosses_after_quote, 0);
    assert_eq!(quote_result.ticks.len() - 1, 145);

    swap!(app, dex, pool_key, true, swap_amount, true, slippage, "bob").unwrap();

    let pool_after = get_pool!(app, dex, token_x, token_y, pool_key.fee_tier).unwrap();

    let crosses = ((pool_after.current_tick_index - pool_before.current_tick_index) / 10).abs();
    assert_eq!(crosses, 146);
    assert_eq!(
        pool_after.current_tick_index,
        get_tick_at_sqrt_price(quote_result.target_sqrt_price, 10).unwrap()
    );
}
