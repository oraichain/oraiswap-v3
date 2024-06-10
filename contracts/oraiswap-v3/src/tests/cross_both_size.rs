use cosmwasm_std::coin;
use decimal::{Decimal, Factories};

use crate::{
    create_entry_points_testing, fee_growth::FeeGrowth, liquidity::Liquidity, percentage::Percentage, sqrt_price::{calculate_sqrt_price, SqrtPrice}, tests::helper::MockApp, token_amount::TokenAmount, FeeTier, PoolKey, MAX_SQRT_PRICE, MIN_SQRT_PRICE
};

#[test]
fn test_cross_both_side() {
    let fee_tier = FeeTier::new(Percentage::from_scale(6, 3), 10).unwrap();
    let init_tick = 0;
    let init_sqrt_price = calculate_sqrt_price(init_tick).unwrap();
    let initial_mint = 10u128.pow(10); 
    let mint_token = 10u128.pow(5);

    let mut app = MockApp::new(&[("alice", &[coin(initial_mint, "orai")])]);
    app.set_token_contract(Box::new(create_entry_points_testing!(cw20_base)));

    let dex_addr = app
        .create_dex("alice", Percentage::from_scale(1, 2))
        .unwrap();
    let token_x = app.create_token("alice", "tokenx", mint_token);
    let token_y = app.create_token("alice", "tokeny", mint_token);

    let pool_key = PoolKey::new(token_x.clone(), token_y.clone(), fee_tier.clone()).unwrap();

    app.add_fee_tier("alice", dex_addr.as_str(), fee_tier.clone())
        .unwrap();

    app.create_pool(
        "alice",
        dex_addr.as_str(),
        token_x.as_str(),
        token_y.as_str(),
        fee_tier,
        init_sqrt_price,
        init_tick,
    )
    .unwrap();

    let lower_tick_index = -10;
    let upper_tick_index = 10;

    let mint_amount = 10u128.pow(5);

    app.mint_token("alice", "bob", token_x.as_str(), mint_amount)
        .unwrap();
    app.mint_token("alice", "alice", token_y.as_str(), mint_amount)
        .unwrap();

    app.approve_token("tokenx", "alice", dex_addr.as_str(), mint_amount)
        .unwrap();
    app.approve_token("tokeny", "alice", dex_addr.as_str(), mint_amount)
        .unwrap();

    let liquidity_delta = Liquidity::from_integer(20006000);

    let pool_state = app
        .get_pool(
            dex_addr.as_str(),
            token_x.as_str(),
            token_y.as_str(),
            fee_tier,
        )
        .unwrap();

    app.create_position(
        "alice",
        dex_addr.as_str(),
        &pool_key,
        lower_tick_index,
        upper_tick_index,
        liquidity_delta,
        pool_state.sqrt_price,
        pool_state.sqrt_price,
    )
    .unwrap();

    app.create_position(
        "alice",
        dex_addr.as_str(),
        &pool_key,
        -20,
        lower_tick_index,
        liquidity_delta,
        pool_state.sqrt_price,
        pool_state.sqrt_price,
    )
    .unwrap();

    let pool = app
        .get_pool(
            dex_addr.as_str(),
            token_x.as_str(),
            token_y.as_str(),
            fee_tier,
        )
        .unwrap();

    assert_eq!(pool.liquidity, liquidity_delta);

    let limit_without_cross_tick_amount = TokenAmount(10_068);
    let not_cross_amount = TokenAmount(1);
    let min_amount_to_cross_from_tick_price = TokenAmount(3);
    let crossing_amount_by_amount_out = TokenAmount(20136101434);

    let mint_amount = limit_without_cross_tick_amount.get()
        + not_cross_amount.get()
        + min_amount_to_cross_from_tick_price.get()
        + crossing_amount_by_amount_out.get();

    app.mint_token("alice", "alice", token_x.as_str(), mint_amount)
        .unwrap();
    app.mint_token("alice", "alice", token_y.as_str(), mint_amount)
        .unwrap();

    app.approve_token("tokenx", "alice", dex_addr.as_str(), mint_amount)
        .unwrap();
    app.approve_token("tokeny", "alice", dex_addr.as_str(), mint_amount)
        .unwrap();

    let pool_before = app
        .get_pool(
            dex_addr.as_str(),
            token_x.as_str(),
            token_y.as_str(),
            fee_tier,
        )
        .unwrap();

    // println!("Pool before first swap: {:?}", pool_before);

    let limit_sqrt_price = SqrtPrice::new(MIN_SQRT_PRICE);

    app.swap(
        "alice",
        dex_addr.as_str(),
        &pool_key,
        true,
        limit_without_cross_tick_amount,
        true,
        limit_sqrt_price,
    )
    .unwrap();

    let pool = app
        .get_pool(
            dex_addr.as_str(),
            token_x.as_str(),
            token_y.as_str(),
            fee_tier,
        )
        .unwrap();

    // println!("Pool after first swap: {:?}", pool);

    let expected_tick = -10;
    let expected_price = calculate_sqrt_price(expected_tick).unwrap();

    assert_eq!(pool.current_tick_index, expected_tick);
    assert_eq!(pool.liquidity, pool_before.liquidity);
    assert_eq!(pool.sqrt_price, expected_price);

    app.swap(
        "alice",
        dex_addr.as_str(),
        &pool_key,
        true,
        min_amount_to_cross_from_tick_price,
        true,
        limit_sqrt_price,
    )
    .unwrap();

    app.swap(
        "alice",
        dex_addr.as_str(),
        &pool_key,
        false,
        min_amount_to_cross_from_tick_price,
        true,
        SqrtPrice::new(MAX_SQRT_PRICE),
    )
    .unwrap();

    let massive_x = 10u128.pow(19);
    let massive_y = 10u128.pow(19);

    app.mint_token("alice", "alice", token_x.as_str(), massive_x)
        .unwrap();
    app.mint_token("alice", "alice", token_y.as_str(), massive_y)
        .unwrap();

    app.approve_token("tokenx", "alice", dex_addr.as_str(), massive_x)
        .unwrap();
    app.approve_token("tokeny", "alice", dex_addr.as_str(), massive_y)
        .unwrap();

    let massive_liquidity_delta = Liquidity::from_integer(19996000399699881985603u128);

    app.create_position(
        "alice",
        dex_addr.as_str(),
        &pool_key,
        -20,
        0,
        massive_liquidity_delta,
        SqrtPrice::new(MIN_SQRT_PRICE),
        SqrtPrice::new(MAX_SQRT_PRICE),
    )
    .unwrap();

    app.swap(
        "alice",
        dex_addr.as_str(),
        &pool_key,
        true,
        TokenAmount(1),
        false,
        limit_sqrt_price,
    )
    .unwrap();

    app.swap(
        "alice",
        dex_addr.as_str(),
        &pool_key,
        false,
        TokenAmount(2),
        true,
        SqrtPrice::new(MAX_SQRT_PRICE),
    )
    .unwrap();

    let pool = app
        .get_pool(
            dex_addr.as_str(),
            token_x.as_str(),
            token_y.as_str(),
            fee_tier,
        )
        .unwrap();

    let expected_liquidity = Liquidity::from_integer(19996000399699901991603u128);
    let expected_liquidity_change_on_last_tick =
        Liquidity::from_integer(19996000399699901991603u128);
    let expected_liquidity_change_on_upper_tick = Liquidity::from_integer(20006000);

    assert_eq!(pool.current_tick_index, -20);
    assert_eq!(
        pool.fee_growth_global_x,
        FeeGrowth::new(29991002699190242927121)
    );
    assert_eq!(pool.fee_growth_global_y, FeeGrowth::new(0));
    assert_eq!(pool.fee_protocol_token_x, TokenAmount(4));
    assert_eq!(pool.fee_protocol_token_y, TokenAmount(2));
    assert_eq!(pool.liquidity, expected_liquidity);
    assert_eq!(pool.sqrt_price, SqrtPrice::new(999500149964999999999999));

    let final_last_tick = app.get_tick(dex_addr.as_str(), &pool_key, -20).unwrap();
    assert_eq!(final_last_tick.fee_growth_outside_x, FeeGrowth::new(0));
    assert_eq!(final_last_tick.fee_growth_outside_y, FeeGrowth::new(0));
    assert_eq!(
        final_last_tick.liquidity_change,
        expected_liquidity_change_on_last_tick
    );

    let final_lower_tick = app.get_tick(dex_addr.as_str(), &pool_key, -10).unwrap();
    assert_eq!(
        final_lower_tick.fee_growth_outside_x,
        FeeGrowth::new(29991002699190242927121)
    );
    assert_eq!(final_lower_tick.fee_growth_outside_y, FeeGrowth::new(0));
    assert_eq!(final_lower_tick.liquidity_change, Liquidity::new(0));

    let final_upper_tick = app.get_tick(dex_addr.as_str(), &pool_key, 10).unwrap();
    assert_eq!(final_upper_tick.fee_growth_outside_x, FeeGrowth::new(0));
    assert_eq!(final_upper_tick.fee_growth_outside_y, FeeGrowth::new(0));
    assert_eq!(
        final_upper_tick.liquidity_change,
        expected_liquidity_change_on_upper_tick
    );
}

// #[test]
// fn test_cross_both_side_not_cross_case() {
//     let fee_tier = FeeTier::new(Percentage::from_scale(6, 3), 10).unwrap();
//     let init_tick = 0;
//     let init_sqrt_price = calculate_sqrt_price(init_tick).unwrap();
//     let initial_mint = 10u128.pow(11);

//     let mut app = MockApp::new(&[
//         ("owner", &[coin(initial_mint, "orai")]),
//         ("alice", &[coin(initial_mint, "orai")]),
//     ]);
//     app.set_token_contract(Box::new(create_entry_points_testing!(cw20_base)));

//     let dex_addr = app
//         .create_dex("owner", Percentage::from_scale(1, 2))
//         .unwrap();
//     let token_x = app.create_token("owner", "tokenx", initial_mint);
//     let token_y = app.create_token("owner", "tokeny", initial_mint);

//     let pool_key = PoolKey::new(token_x.clone(), token_y.clone(), fee_tier.clone()).unwrap();

//     app.add_fee_tier("owner", dex_addr.as_str(), fee_tier.clone())
//         .unwrap();

//     let create_pool_msg = ExecuteMsg::CreatePool {
//         token_0: token_x.clone(),
//         token_1: token_y.clone(),
//         fee_tier: fee_tier.clone(),
//         init_sqrt_price: init_sqrt_price.clone(),
//         init_tick,
//     };
//     app.execute(
//         Addr::unchecked("owner"),
//         dex_addr.clone(),
//         &create_pool_msg,
//         &[],
//     )
//     .unwrap();

//     let lower_tick_index = -10;
//     let upper_tick_index = 10;
//     let liquidity_delta = Liquidity::from_integer(20006000);

//     app.approve_token("tokenx", "alice", dex_addr.as_str(), initial_mint)
//         .unwrap();
//     app.approve_token("tokeny", "alice", dex_addr.as_str(), initial_mint)
//         .unwrap();

//     let create_position_msg = ExecuteMsg::CreatePosition {
//         pool_key: pool_key.clone(),
//         lower_tick: lower_tick_index,
//         upper_tick: upper_tick_index,
//         liquidity_delta,
//         slippage_limit_lower: SqrtPrice::new(0),
//         slippage_limit_upper: SqrtPrice::new(0),
//     };
//     app.execute(
//         Addr::unchecked("alice"),
//         dex_addr.clone(),
//         &create_position_msg,
//         &[],
//     )
//     .unwrap();

//     let limit_without_cross_tick_amount = TokenAmount(10_068);
//     let not_cross_amount = TokenAmount(1);
//     let min_amount_to_cross_from_tick_price = TokenAmount(3);
//     let crossing_amount_by_amount_out = TokenAmount(20136101434);

//     let mint_amount = limit_without_cross_tick_amount.get()
//         + not_cross_amount.get()
//         + min_amount_to_cross_from_tick_price.get()
//         + crossing_amount_by_amount_out.get();

//     app.approve_token("tokenx", "alice", dex_addr.as_str(), mint_amount)
//         .unwrap();
//     app.approve_token("tokeny", "alice", dex_addr.as_str(), mint_amount)
//         .unwrap();

//     let pool_before: Pool = app
//         .query(
//             dex_addr.clone(),
//             &QueryMsg::Pool {
//                 token_0: token_x.clone(),
//                 token_1: token_y.clone(),
//                 fee_tier: fee_tier.clone(),
//             },
//         )
//         .unwrap();

//     let limit_sqrt_price = SqrtPrice::new(0);

//     let swap_msg = ExecuteMsg::Swap {
//         pool_key: pool_key.clone(),
//         x_to_y: true,
//         amount: limit_without_cross_tick_amount,
//         by_amount_in: true,
//         sqrt_price_limit: limit_sqrt_price,
//     };
//     app.execute(Addr::unchecked("alice"), dex_addr.clone(), &swap_msg, &[])
//         .unwrap();

//     let pool: Pool = app
//         .query(
//             dex_addr.clone(),
//             &QueryMsg::Pool {
//                 token_0: token_x.clone(),
//                 token_1: token_y.clone(),
//                 fee_tier: fee_tier.clone(),
//             },
//         )
//         .unwrap();

//     let expected_tick = -10;
//     let expected_price = calculate_sqrt_price(expected_tick).unwrap();

//     assert_eq!(pool.current_tick_index, expected_tick);
//     assert_eq!(pool.liquidity, pool_before.liquidity);
//     assert_eq!(pool.sqrt_price, expected_price);

//     let slippage = SqrtPrice::new(0);

//     let swap_msg = ExecuteMsg::Swap {
//         pool_key: pool_key.clone(),
//         x_to_y: true,
//         amount: not_cross_amount,
//         by_amount_in: true,
//         sqrt_price_limit: slippage,
//     };
//     let result = app.execute(Addr::unchecked("alice"), dex_addr.clone(), &swap_msg, &[]);
//     assert!(result.unwrap_err().contains("error executing WasmMsg"));
// }
