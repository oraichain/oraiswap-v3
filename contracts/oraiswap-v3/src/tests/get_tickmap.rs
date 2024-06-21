use cosmwasm_std::Addr;
use decimal::{Decimal, Factories};

use crate::get_max_chunk;
use crate::sqrt_price::get_max_tick;
use crate::sqrt_price::get_min_tick;
use crate::{
    liquidity::Liquidity,
    msg,
    percentage::Percentage,
    sqrt_price::{calculate_sqrt_price, SqrtPrice},
    tests::helper::{macros::*, MockApp},
    FeeTier, PoolKey,
};

fn _to_binary(v: (u16, u64)) {
    println!(
        "Chunk Index = {:?} Value = {:?}, Binary = {:b}",
        v.0, v.1, v.1
    );
}

#[test]
fn test_get_tickmap() {
    let mut app = MockApp::new(&[]);
    let dex = create_dex!(app, Percentage::new(0));
    let initial_amount = 10u128.pow(10);
    let (token_x, token_y) = create_tokens!(app, initial_amount, initial_amount);

    approve!(app, token_x, dex, initial_amount, "alice").unwrap();
    approve!(app, token_y, dex, initial_amount, "alice").unwrap();

    let fee_tier = FeeTier::new(Percentage::from_scale(5, 1), 1).unwrap();
    let pool_key = PoolKey::new(token_x.to_string(), token_y.to_string(), fee_tier).unwrap();
    let init_tick = 0;
    let init_sqrt_price = calculate_sqrt_price(init_tick).unwrap();

    add_fee_tier!(app, dex, fee_tier, "alice").unwrap();

    let result = create_pool!(
        app,
        dex,
        token_x,
        token_y,
        fee_tier,
        init_sqrt_price,
        init_tick,
        "alice"
    );
    assert!(result.is_ok());

    let pool = get_pool!(app, dex, token_x, token_y, fee_tier).unwrap();

    let liquidity_delta = Liquidity::new(1000);

    create_position!(
        app,
        dex,
        pool_key,
        -58,
        5,
        liquidity_delta,
        pool.sqrt_price,
        SqrtPrice::max_instance(),
        "alice"
    )
    .unwrap();

    let tickmap: Vec<(u16, u64)> = get_tickmap!(
        app,
        dex,
        pool_key,
        get_min_tick(fee_tier.tick_spacing),
        get_max_tick(fee_tier.tick_spacing),
        false
    )
    .unwrap();

    assert_eq!(
        tickmap[0],
        (
            3465,
            0b1000000000000000000000000000000000000000000000000000000000000001
        )
    );
    assert_eq!(tickmap.len(), 1);
}

#[test]
fn test_get_tickmap_tick_spacing_over_one() {
    let mut app = MockApp::new(&[]);
    let dex = create_dex!(app, Percentage::new(0));
    let initial_amount = 10u128.pow(10);
    let (token_x, token_y) = create_tokens!(app, initial_amount, initial_amount);

    approve!(app, token_x, dex, initial_amount, "alice").unwrap();
    approve!(app, token_y, dex, initial_amount, "alice").unwrap();

    let fee_tier = FeeTier::new(Percentage::from_scale(5, 1), 10).unwrap();
    let pool_key = PoolKey::new(token_x.to_string(), token_y.to_string(), fee_tier).unwrap();
    let init_tick = 0;
    let init_sqrt_price = calculate_sqrt_price(init_tick).unwrap();

    add_fee_tier!(app, dex, fee_tier, "alice").unwrap();

    let result = create_pool!(
        app,
        dex,
        token_x,
        token_y,
        fee_tier,
        init_sqrt_price,
        init_tick,
        "alice"
    );
    assert!(result.is_ok());

    let pool = get_pool!(app, dex, token_x, token_y, fee_tier).unwrap();

    let liquidity_delta = Liquidity::new(1000);

    create_position!(
        app,
        dex,
        pool_key,
        10,
        20,
        liquidity_delta,
        pool.sqrt_price,
        SqrtPrice::max_instance(),
        "alice"
    )
    .unwrap();

    create_position!(
        app,
        dex,
        pool_key,
        get_min_tick(fee_tier.tick_spacing),
        get_max_tick(fee_tier.tick_spacing),
        liquidity_delta,
        pool.sqrt_price,
        SqrtPrice::max_instance(),
        "alice"
    )
    .unwrap();

    let tickmap: Vec<(u16, u64)> = get_tickmap!(
        app,
        dex,
        pool_key,
        get_min_tick(fee_tier.tick_spacing),
        get_max_tick(fee_tier.tick_spacing),
        false
    )
    .unwrap();

    assert_eq!(tickmap[0], (0, 0b1));
    assert_eq!(
        tickmap[1],
        (346, 0b1100000000000000000000000000000000000000)
    );
    assert_eq!(
        tickmap[2],
        (get_max_chunk(fee_tier.tick_spacing), 0b10000000000)
    );
    assert_eq!(tickmap.len(), 3);
}

#[test]
fn test_get_tickmap_edge_ticks_intialized() {
    let mut app = MockApp::new(&[]);
    let dex = create_dex!(app, Percentage::new(0));
    let initial_amount = 10u128.pow(10);
    let (token_x, token_y) = create_tokens!(app, initial_amount, initial_amount);

    approve!(app, token_x, dex, initial_amount, "alice").unwrap();
    approve!(app, token_y, dex, initial_amount, "alice").unwrap();

    let fee_tier = FeeTier::new(Percentage::from_scale(5, 1), 1).unwrap();
    let pool_key = PoolKey::new(token_x.to_string(), token_y.to_string(), fee_tier).unwrap();
    let init_tick = 0;
    let init_sqrt_price = calculate_sqrt_price(init_tick).unwrap();

    add_fee_tier!(app, dex, fee_tier, "alice").unwrap();

    let result = create_pool!(
        app,
        dex,
        token_x,
        token_y,
        fee_tier,
        init_sqrt_price,
        init_tick,
        "alice"
    );
    assert!(result.is_ok());

    let pool = get_pool!(app, dex, token_x, token_y, fee_tier).unwrap();

    let liquidity_delta = Liquidity::new(1000);

    create_position!(
        app,
        dex,
        pool_key,
        -221818,
        -221817,
        liquidity_delta,
        pool.sqrt_price,
        SqrtPrice::max_instance(),
        "alice"
    )
    .unwrap();

    create_position!(
        app,
        dex,
        pool_key,
        221817,
        221818,
        liquidity_delta,
        pool.sqrt_price,
        SqrtPrice::max_instance(),
        "alice"
    )
    .unwrap();

    let tickmap: Vec<(u16, u64)> = get_tickmap!(
        app,
        dex,
        pool_key,
        get_min_tick(fee_tier.tick_spacing),
        get_max_tick(fee_tier.tick_spacing),
        false
    )
    .unwrap();

    assert_eq!(tickmap[0], (0, 0b11));
    assert_eq!(
        tickmap[1],
        (
            get_max_chunk(fee_tier.tick_spacing),
            0b11000000000000000000000000000000000000000000000000000
        )
    );
    assert_eq!(tickmap.len(), 2);
    {
        let tickmap: Vec<(u16, u64)> = get_tickmap!(
            app,
            dex,
            pool_key,
            get_min_tick(fee_tier.tick_spacing),
            get_max_tick(fee_tier.tick_spacing),
            false
        )
        .unwrap();
        assert_eq!(tickmap[0], (0, 0b11));
        assert_eq!(
            tickmap[1],
            (
                get_max_chunk(fee_tier.tick_spacing),
                0b11000000000000000000000000000000000000000000000000000
            )
        );
        assert_eq!(tickmap.len(), 2);

        let tickmap: Vec<(u16, u64)> = get_tickmap!(
            app,
            dex,
            pool_key,
            get_min_tick(fee_tier.tick_spacing),
            get_max_tick(fee_tier.tick_spacing),
            false
        )
        .unwrap();
        assert_eq!(tickmap[0], (0, 0b11));
        assert_eq!(
            tickmap[1],
            (
                get_max_chunk(fee_tier.tick_spacing),
                0b11000000000000000000000000000000000000000000000000000
            )
        );
        assert_eq!(tickmap.len(), 2);

        let tickmap: Vec<(u16, u64)> = get_tickmap!(
            app,
            dex,
            pool_key,
            get_min_tick(fee_tier.tick_spacing),
            get_max_tick(fee_tier.tick_spacing),
            true
        )
        .unwrap();
        assert_eq!(tickmap[1], (0, 0b11));
        assert_eq!(
            tickmap[0],
            (
                get_max_chunk(fee_tier.tick_spacing),
                0b11000000000000000000000000000000000000000000000000000
            )
        );
        assert_eq!(tickmap.len(), 2);
    }
}

#[test]
fn test_get_tickmap_more_chunks_above() {
    let mut app = MockApp::new(&[]);
    let dex = create_dex!(app, Percentage::new(0));
    let initial_amount = 10u128.pow(10);
    let (token_x, token_y) = create_tokens!(app, initial_amount, initial_amount);

    approve!(app, token_x, dex, initial_amount, "alice").unwrap();
    approve!(app, token_y, dex, initial_amount, "alice").unwrap();

    let fee_tier = FeeTier::new(Percentage::from_scale(5, 1), 1).unwrap();
    let pool_key = PoolKey::new(token_x.to_string(), token_y.to_string(), fee_tier).unwrap();
    let init_tick = 0;
    let init_sqrt_price = calculate_sqrt_price(init_tick).unwrap();

    add_fee_tier!(app, dex, fee_tier, "alice").unwrap();

    let result = create_pool!(
        app,
        dex,
        token_x,
        token_y,
        fee_tier,
        init_sqrt_price,
        init_tick,
        "alice"
    );
    assert!(result.is_ok());

    let pool = get_pool!(app, dex, token_x, token_y, fee_tier).unwrap();

    let liquidity_delta = Liquidity::new(1000);

    for i in (6..52500).step_by(64) {
        create_position!(
            app,
            dex,
            pool_key,
            i,
            i + 1,
            liquidity_delta,
            pool.sqrt_price,
            SqrtPrice::max_instance(),
            "alice"
        )
        .unwrap();
    }

    let tickmap: Vec<(u16, u64)> = get_tickmap!(
        app,
        dex,
        pool_key,
        get_min_tick(fee_tier.tick_spacing),
        get_max_tick(fee_tier.tick_spacing),
        false
    )
    .unwrap();

    for (i, _) in (0..tickmap.len()).enumerate() {
        let current = 3466 + i as u16;
        assert_eq!(tickmap[i], (current, 0b11));
    }
}

#[test]
fn test_get_tickmap_more_chunks_below() {
    let mut app = MockApp::new(&[]);
    let dex = create_dex!(app, Percentage::new(0));
    let initial_amount = 10u128.pow(10);
    let (token_x, token_y) = create_tokens!(app, initial_amount, initial_amount);

    approve!(app, token_x, dex, initial_amount, "alice").unwrap();
    approve!(app, token_y, dex, initial_amount, "alice").unwrap();

    let fee_tier = FeeTier::new(Percentage::from_scale(5, 1), 1).unwrap();
    let pool_key = PoolKey::new(token_x.to_string(), token_y.to_string(), fee_tier).unwrap();
    let init_tick = 0;
    let init_sqrt_price = calculate_sqrt_price(init_tick).unwrap();

    add_fee_tier!(app, dex, fee_tier, "alice").unwrap();

    let result = create_pool!(
        app,
        dex,
        token_x,
        token_y,
        fee_tier,
        init_sqrt_price,
        init_tick,
        "alice"
    );
    assert!(result.is_ok());

    let pool = get_pool!(app, dex, token_x, token_y, fee_tier).unwrap();

    let liquidity_delta = Liquidity::new(1000);

    for i in (-52544..6).step_by(64) {
        create_position!(
            app,
            dex,
            pool_key,
            i,
            i + 1,
            liquidity_delta,
            pool.sqrt_price,
            SqrtPrice::max_instance(),
            "alice"
        )
        .unwrap();
    }

    let tickmap: Vec<(u16, u64)> = get_tickmap!(
        app,
        dex,
        pool_key,
        get_min_tick(fee_tier.tick_spacing),
        get_max_tick(fee_tier.tick_spacing),
        false
    )
    .unwrap();
    for (i, _) in (0..tickmap.len()).enumerate() {
        let current = 2644 + i as u16;
        assert_eq!(
            tickmap[i],
            (
                current,
                0b110000000000000000000000000000000000000000000000000000000000
            )
        );
    }
}

#[test]
fn test_get_tickmap_max_chunks_returned() {
    let mut app = MockApp::new(&[]);
    let dex = create_dex!(app, Percentage::new(0));
    let initial_amount = 10u128.pow(10);
    let (token_x, token_y) = create_tokens!(app, initial_amount, initial_amount);

    approve!(app, token_x, dex, initial_amount, "alice").unwrap();
    approve!(app, token_y, dex, initial_amount, "alice").unwrap();

    let fee_tier = FeeTier::new(Percentage::from_scale(5, 1), 1).unwrap();
    let pool_key = PoolKey::new(token_x.to_string(), token_y.to_string(), fee_tier).unwrap();
    let init_tick = -200_000;
    let init_sqrt_price = calculate_sqrt_price(init_tick).unwrap();

    add_fee_tier!(app, dex, fee_tier, "alice").unwrap();

    let result = create_pool!(
        app,
        dex,
        token_x,
        token_y,
        fee_tier,
        init_sqrt_price,
        init_tick,
        "alice"
    );
    assert!(result.is_ok());

    let pool = get_pool!(app, dex, token_x, token_y, fee_tier).unwrap();

    let liquidity_delta = Liquidity::new(1000);

    for i in (0..104832).step_by(64) {
        create_position!(
            app,
            dex,
            pool_key,
            i,
            i + 1,
            liquidity_delta,
            pool.sqrt_price,
            SqrtPrice::max_instance(),
            "alice"
        )
        .unwrap();
    }

    let tickmap: Vec<(u16, u64)> = get_tickmap!(
        app,
        dex,
        pool_key,
        get_min_tick(fee_tier.tick_spacing),
        get_max_tick(fee_tier.tick_spacing),
        false
    )
    .unwrap();

    assert_eq!(tickmap.len(), 1638);
}
