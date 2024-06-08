use cosmwasm_std::{coin, Addr};
use decimal::{Decimal, Factories};

use crate::{
    create_entry_points_testing,
    fee_growth::FeeGrowth,
    liquidity::Liquidity,
    msg::{ExecuteMsg, QueryMsg},
    percentage::Percentage,
    sqrt_price::{calculate_sqrt_price, SqrtPrice},
    tests::helper::MockApp,
    token_amount::TokenAmount,
    FeeTier, Pool, PoolKey, Tick, MIN_SQRT_PRICE,
};

#[test]
fn test_cross_both_side() {
    let fee_tier = FeeTier::new(Percentage::from_scale(6, 3), 10).unwrap();
    let init_tick = 0;
    let init_sqrt_price = calculate_sqrt_price(init_tick).unwrap();
    let initial_mint = 10u128.pow(10); // Tăng số lượng mint ban đầu
    let mint_token = 10u128.pow(5);

    let mut app = MockApp::new(&[
        ("owner", &[coin(initial_mint, "orai")]),
        ("alice", &[coin(initial_mint, "orai")]),
    ]);
    app.set_token_contract(Box::new(create_entry_points_testing!(cw20_base)));

    let dex_addr = app
        .create_dex("owner", Percentage::from_scale(1, 2))
        .unwrap();
    let token_x = app.create_token("alice", "tokenx", mint_token);
    let token_y = app.create_token("alice", "tokeny", mint_token);

    let pool_key = PoolKey::new(token_x.clone(), token_y.clone(), fee_tier.clone()).unwrap();

    app.add_fee_tier("owner", dex_addr.as_str(), fee_tier.clone())
        .unwrap();

    let create_pool_msg = ExecuteMsg::CreatePool {
        token_0: token_x.clone(),
        token_1: token_y.clone(),
        fee_tier: fee_tier.clone(),
        init_sqrt_price: init_sqrt_price.clone(),
        init_tick,
    };
    app.execute(
        Addr::unchecked("alice"),
        dex_addr.clone(),
        &create_pool_msg,
        &[],
    )
    .unwrap();

    let lower_tick_index = -10;
    let upper_tick_index = 10;
    let liquidity_delta = Liquidity::from_integer(20006000);

    app.approve_token("tokenx", "alice", dex_addr.as_str(), initial_mint)
        .unwrap();
    app.approve_token("tokeny", "alice", dex_addr.as_str(), initial_mint)
        .unwrap();

    // create pos 1
    let create_position_msg = ExecuteMsg::CreatePosition {
        pool_key: pool_key.clone(),
        lower_tick: lower_tick_index,
        upper_tick: upper_tick_index,
        liquidity_delta,
        slippage_limit_lower: SqrtPrice::new(0),
        slippage_limit_upper: SqrtPrice::max_instance(),
    };
    app.execute(
        Addr::unchecked("alice"),
        dex_addr.clone(),
        &create_position_msg,
        &[],
    )
    .unwrap();

    let limit_without_cross_tick_amount = TokenAmount(10_068);
    let not_cross_amount = TokenAmount(1);
    let min_amount_to_cross_from_tick_price = TokenAmount(3);
    let crossing_amount_by_amount_out = TokenAmount(20136101434);

    let mint_amount = limit_without_cross_tick_amount.get()
        + not_cross_amount.get()
        + min_amount_to_cross_from_tick_price.get()
        + crossing_amount_by_amount_out.get();

    app.set_token_balances("alice", &[("alice", &[("alice", mint_amount + initial_mint)])]).unwrap();
    app.set_token_balances("alice", &[("alice", &[("alice", mint_amount + initial_mint)])]).unwrap();

    app.approve_token("tokenx", "alice", dex_addr.as_str(), mint_amount)
        .unwrap();
    app.approve_token("tokeny", "alice", dex_addr.as_str(), mint_amount)
        .unwrap();

    let pool_before: Pool = app
        .query(
            dex_addr.clone(),
            &QueryMsg::Pool {
                token_0: token_x.clone(),
                token_1: token_y.clone(),
                fee_tier: fee_tier.clone(),
            },
        )
        .unwrap();

    println!("Pool before first swap: {:?}", pool_before);

    let limit_sqrt_price = SqrtPrice::new(MIN_SQRT_PRICE);

    // swap 1
    let swap_msg = ExecuteMsg::Swap {
        pool_key: pool_key.clone(),
        x_to_y: true,
        amount: limit_without_cross_tick_amount,
        by_amount_in: true,
        sqrt_price_limit: limit_sqrt_price,
    };
    app.execute(Addr::unchecked("alice"), dex_addr.clone(), &swap_msg, &[])
        .unwrap();

    let pool: Pool = app
        .query(
            dex_addr.clone(),
            &QueryMsg::Pool {
                token_0: token_x.clone(),
                token_1: token_y.clone(),
                fee_tier: fee_tier.clone(),
            },
        )
        .unwrap();

    println!("Pool after first swap: {:?}", pool);

    let expected_tick = -10;
    let expected_price = calculate_sqrt_price(expected_tick).unwrap();

    assert_eq!(pool.current_tick_index, expected_tick);
    assert_eq!(pool.liquidity, liquidity_delta);
    assert_eq!(pool.sqrt_price, expected_price);

    // error here
    let swap_msg = ExecuteMsg::Swap {
        pool_key: pool_key.clone(),
        x_to_y: true,
        amount: min_amount_to_cross_from_tick_price,
        by_amount_in: true,
        sqrt_price_limit: SqrtPrice::new(0),
    };
    app.execute(Addr::unchecked("alice"), dex_addr.clone(), &swap_msg, &[])
        .unwrap();

    // let swap_msg = ExecuteMsg::Swap {
    //     pool_key: pool_key.clone(),
    //     x_to_y: false,
    //     amount: min_amount_to_cross_from_tick_price,
    //     by_amount_in: true,
    //     sqrt_price_limit: SqrtPrice::new(0),
    // };
    // app.execute(Addr::unchecked("alice"), dex_addr.clone(), &swap_msg, &[])
    //     .unwrap();

    // let massive_x = 10u128.pow(19);
    // let massive_y = 10u128.pow(19);

    // app.approve_token("tokenx", "alice", dex_addr.as_str(), massive_x)
    //     .unwrap();
    // app.approve_token("tokeny", "alice", dex_addr.as_str(), massive_y)
    //     .unwrap();

    // let massive_liquidity_delta = Liquidity::from_integer(19996000399699881985603u128);

    // let create_position_msg = ExecuteMsg::CreatePosition {
    //     pool_key: pool_key.clone(),
    //     lower_tick: -20,
    //     upper_tick: 0,
    //     liquidity_delta: massive_liquidity_delta,
    //     slippage_limit_lower: SqrtPrice::new(0),
    //     slippage_limit_upper: SqrtPrice::max_instance(),
    // };
    // app.execute(
    //     Addr::unchecked("alice"),
    //     dex_addr.clone(),
    //     &create_position_msg,
    //     &[],
    // )
    // .unwrap();

    // let swap_msg = ExecuteMsg::Swap {
    //     pool_key: pool_key.clone(),
    //     x_to_y: true,
    //     amount: TokenAmount(1),
    //     by_amount_in: false,
    //     sqrt_price_limit: limit_sqrt_price,
    // };
    // app.execute(Addr::unchecked("alice"), dex_addr.clone(), &swap_msg, &[])
    //     .unwrap();

    // let swap_msg = ExecuteMsg::Swap {
    //     pool_key: pool_key.clone(),
    //     x_to_y: false,
    //     amount: TokenAmount(2),
    //     by_amount_in: true,
    //     sqrt_price_limit: SqrtPrice::new(0),
    // };
    // app.execute(Addr::unchecked("alice"), dex_addr.clone(), &swap_msg, &[])
    //     .unwrap();

    // let pool: Pool = app
    //     .query(
    //         dex_addr.clone(),
    //         &QueryMsg::Pool {
    //             token_0: token_x.clone(),
    //             token_1: token_y.clone(),
    //             fee_tier: fee_tier.clone(),
    //         },
    //     )
    //     .unwrap();

    // let expected_liquidity = Liquidity::from_integer(19996000399699901991603u128);
    // let expected_liquidity_change_on_last_tick =
    //     Liquidity::from_integer(19996000399699901991603u128);
    // let expected_liquidity_change_on_upper_tick = Liquidity::from_integer(20006000);

    // assert_eq!(pool.current_tick_index, -20);
    // assert_eq!(
    //     pool.fee_growth_global_x,
    //     FeeGrowth::new(29991002699190242927121)
    // );
    // assert_eq!(pool.fee_growth_global_y, FeeGrowth::new(0));
    // assert_eq!(pool.fee_protocol_token_x, TokenAmount(4));
    // assert_eq!(pool.fee_protocol_token_y, TokenAmount(2));
    // assert_eq!(pool.liquidity, expected_liquidity);
    // assert_eq!(pool.sqrt_price, SqrtPrice::new(999500149964999999999999));

    // let final_last_tick: Tick = app
    //     .query(
    //         dex_addr.clone(),
    //         &QueryMsg::Tick {
    //             key: pool_key.clone(),
    //             index: -20,
    //         },
    //     )
    //     .unwrap();
    // assert_eq!(final_last_tick.fee_growth_outside_x, FeeGrowth::new(0));
    // assert_eq!(final_last_tick.fee_growth_outside_y, FeeGrowth::new(0));
    // assert_eq!(
    //     final_last_tick.liquidity_change,
    //     expected_liquidity_change_on_last_tick
    // );

    // let final_lower_tick: Tick = app
    //     .query(
    //         dex_addr.clone(),
    //         &QueryMsg::Tick {
    //             key: pool_key.clone(),
    //             index: -10,
    //         },
    //     )
    //     .unwrap();
    // assert_eq!(
    //     final_lower_tick.fee_growth_outside_x,
    //     FeeGrowth::new(29991002699190242927121)
    // );
    // assert_eq!(final_lower_tick.fee_growth_outside_y, FeeGrowth::new(0));
    // assert_eq!(final_lower_tick.liquidity_change, Liquidity::new(0));

    // let final_upper_tick: Tick = app
    //     .query(
    //         dex_addr.clone(),
    //         &QueryMsg::Tick {
    //             key: pool_key.clone(),
    //             index: 10,
    //         },
    //     )
    //     .unwrap();
    // assert_eq!(final_upper_tick.fee_growth_outside_x, FeeGrowth::new(0));
    // assert_eq!(final_upper_tick.fee_growth_outside_y, FeeGrowth::new(0));
    // assert_eq!(
    //     final_upper_tick.liquidity_change,
    //     expected_liquidity_change_on_upper_tick
    // );
}

// #[test]
// fn test_cross_both_side_not_cross_case() {
//     let fee_tier = FeeTier::new(Percentage::from_scale(6, 3), 10).unwrap();
//     let init_tick = 0;
//     let init_sqrt_price = calculate_sqrt_price(init_tick).unwrap();
//     let initial_mint = 10u128.pow(11); // Tăng số lượng mint ban đầu

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
