use decimal::*;

use crate::{
    interface::SwapHop,
    liquidity::Liquidity,
    percentage::Percentage,
    sqrt_price::calculate_sqrt_price,
    tests::helper::{macros::*, MockApp},
    token_amount::TokenAmount,
    FeeTier, PoolKey,
};

#[test]
fn swap_route() {
    let protocol_fee = Percentage::from_scale(6, 3);
    let initial_amount = 10u128.pow(10);
    let mut app = MockApp::new(&[]);
    let dex = create_dex!(app, protocol_fee);

    let (token_x, token_y, token_z) =
        create_3_tokens!(app, initial_amount, initial_amount, initial_amount);

    approve!(app, token_x, dex, u64::MAX as u128, "alice").unwrap();
    approve!(app, token_y, dex, u64::MAX as u128, "alice").unwrap();
    approve!(app, token_z, dex, u64::MAX as u128, "alice").unwrap();

    let amount = 1000;
    mint!(app, token_x, "bob", amount, "alice").unwrap();
    approve!(app, token_x, dex, amount, "bob").unwrap();
    approve!(app, token_y, dex, u64::MAX as u128, "bob").unwrap();

    let fee_tier = FeeTier::new(protocol_fee, 1).unwrap();

    add_fee_tier!(app, dex, fee_tier, "alice").unwrap();

    let init_tick = 0;
    let init_sqrt_price = calculate_sqrt_price(init_tick).unwrap();
    create_pool!(
        app,
        dex,
        token_x,
        token_y,
        fee_tier,
        init_sqrt_price,
        init_tick,
        "alice"
    )
    .unwrap();

    let init_tick = 0;
    let init_sqrt_price = calculate_sqrt_price(init_tick).unwrap();
    create_pool!(
        app,
        dex,
        token_y,
        token_z,
        fee_tier,
        init_sqrt_price,
        init_tick,
        "alice"
    )
    .unwrap();

    let pool_key_1 = PoolKey::new(token_x.clone(), token_y.clone(), fee_tier).unwrap();
    let pool_key_2 = PoolKey::new(token_y.clone(), token_z.clone(), fee_tier).unwrap();

    let liquidity_delta = Liquidity::new(2u128.pow(63) - 1);

    let pool_1 = get_pool!(app, dex, token_x, token_y, fee_tier).unwrap();
    let slippage_limit_lower = pool_1.sqrt_price;
    let slippage_limit_upper = pool_1.sqrt_price;
    create_position!(
        app,
        dex,
        pool_key_1,
        -1,
        1,
        liquidity_delta,
        slippage_limit_lower,
        slippage_limit_upper,
        "alice"
    )
    .unwrap();

    let pool_2 = get_pool!(app, dex, token_y, token_z, fee_tier).unwrap();
    let slippage_limit_lower = pool_2.sqrt_price;
    let slippage_limit_upper = pool_2.sqrt_price;
    create_position!(
        app,
        dex,
        pool_key_2,
        -1,
        1,
        liquidity_delta,
        slippage_limit_lower,
        slippage_limit_upper,
        "alice"
    )
    .unwrap();

    let amount_in = TokenAmount(1000);
    let slippage = Percentage::new(0);
    let swaps = vec![
        SwapHop {
            pool_key: pool_key_1,
            x_to_y: true,
        },
        SwapHop {
            pool_key: pool_key_2,
            x_to_y: true,
        },
    ];

    let expected_token_amount = quote_route!(app, dex, amount_in, swaps.clone()).unwrap();

    swap_route!(
        app,
        dex,
        amount_in,
        expected_token_amount,
        slippage,
        swaps.clone(),
        "bob"
    )
    .unwrap();

    let bob_amount_x = balance_of!(app, token_x, "bob");
    let bob_amount_y = balance_of!(app, token_y, "bob");
    let bob_amount_z = balance_of!(app, token_z, "bob");

    assert_eq!(bob_amount_x, 0);
    assert_eq!(bob_amount_y, 0);
    assert_eq!(bob_amount_z, 986);

    let pool_1_after = get_pool!(app, dex, token_x, token_y, fee_tier).unwrap();
    assert_eq!(pool_1_after.fee_protocol_token_x, TokenAmount(1));
    assert_eq!(pool_1_after.fee_protocol_token_y, TokenAmount(0));

    let pool_2_after = get_pool!(app, dex, token_y, token_z, fee_tier).unwrap();
    assert_eq!(pool_2_after.fee_protocol_token_x, TokenAmount(1));
    assert_eq!(pool_2_after.fee_protocol_token_y, TokenAmount(0));

    let alice_amount_x_before = balance_of!(app, token_x, "alice");
    let alice_amount_y_before = balance_of!(app, token_y, "alice");
    let alice_amount_z_before = balance_of!(app, token_z, "alice");

    claim_fee!(app, dex, 0, "alice").unwrap();
    claim_fee!(app, dex, 1, "alice").unwrap();

    let alice_amount_x_after = balance_of!(app, token_x, "alice");
    let alice_amount_y_after = balance_of!(app, token_y, "alice");
    let alice_amount_z_after = balance_of!(app, token_z, "alice");

    assert_eq!(alice_amount_x_after - alice_amount_x_before, 4);
    assert_eq!(alice_amount_y_after - alice_amount_y_before, 4);
    assert_eq!(alice_amount_z_after - alice_amount_z_before, 0);
}
