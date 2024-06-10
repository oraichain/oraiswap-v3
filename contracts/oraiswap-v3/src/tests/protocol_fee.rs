use decimal::*;

use crate::{
    percentage::Percentage,
    tests::helper::{macros::*, MockApp},
    token_amount::TokenAmount,
    FeeTier, PoolKey,
};

#[test]
fn test_protocol_fee() {
    let mut app = MockApp::new(&[("alice", &[])]);

    let (dex, token_x, token_y) = init_dex_and_tokens!(app);
    init_basic_pool!(app, dex, token_x, token_y);
    init_basic_position!(app, dex, token_x, token_y);
    init_basic_swap!(app, dex, token_x, token_y);

    let fee_tier = FeeTier::new(Percentage::from_scale(6, 3), 10).unwrap();
    let pool_key = PoolKey::new(token_x.clone(), token_y.clone(), fee_tier).unwrap();

    withdraw_protocol_fee!(app, dex, pool_key, "alice").unwrap();

    let amount_x = balance_of!(app, token_x, "alice");
    let amount_y = balance_of!(app, token_y, "alice");
    assert_eq!(amount_x, 9999999501);
    assert_eq!(amount_y, 9999999000);

    let amount_x = balance_of!(app, token_x, dex);
    let amount_y = balance_of!(app, token_y, dex);
    assert_eq!(amount_x, 1499);
    assert_eq!(amount_y, 7);

    let pool_after_withdraw = get_pool!(app, dex, token_x, token_y, fee_tier).unwrap();
    assert_eq!(
        pool_after_withdraw.fee_protocol_token_x,
        TokenAmount::new(0)
    );
    assert_eq!(
        pool_after_withdraw.fee_protocol_token_y,
        TokenAmount::new(0)
    );
}

#[test]
fn test_protocol_fee_not_admin() {
    let mut app = MockApp::new(&[("alice", &[])]);
    let (dex, token_x, token_y) = init_dex_and_tokens!(app);
    init_basic_pool!(app, dex, token_x, token_y);
    init_basic_position!(app, dex, token_x, token_y);
    init_basic_swap!(app, dex, token_x, token_y);

    let pool_key = PoolKey::new(
        token_x,
        token_y,
        FeeTier {
            fee: Percentage::from_scale(6, 3),
            tick_spacing: 10,
        },
    )
    .unwrap();

    withdraw_protocol_fee!(app, dex, pool_key, "bob").unwrap_err();
}

#[test]
fn test_withdraw_fee_not_deployer() {
    let mut app = MockApp::new(&[("alice", &[])]);
    let (dex, token_x, token_y) = init_dex_and_tokens!(app);
    init_basic_pool!(app, dex, token_x, token_y);
    init_basic_position!(app, dex, token_x, token_y);
    init_basic_swap!(app, dex, token_x, token_y);

    let user_address = "bob";

    let fee_tier = FeeTier::new(Percentage::from_scale(6, 3), 10).unwrap();
    let pool_key = PoolKey::new(token_x.clone(), token_y.clone(), fee_tier).unwrap();
    change_fee_receiver!(app, dex, pool_key, user_address, "alice").unwrap();

    let pool = get_pool!(app, dex, token_x, token_y, fee_tier).unwrap();
    assert_eq!(pool.fee_receiver, user_address);

    withdraw_protocol_fee!(app, dex, pool_key, "bob").unwrap();

    let amount_x = balance_of!(app, token_x, user_address);
    let amount_y = balance_of!(app, token_y, user_address);
    assert_eq!(amount_x, 1);
    assert_eq!(amount_y, 993);

    let amount_x = balance_of!(app, token_x, dex);
    let amount_y = balance_of!(app, token_y, dex);
    assert_eq!(amount_x, 1499);
    assert_eq!(amount_y, 7);

    let pool_after_withdraw = get_pool!(app, dex, token_x, token_y, fee_tier).unwrap();
    assert_eq!(
        pool_after_withdraw.fee_protocol_token_x,
        TokenAmount::new(0)
    );
    assert_eq!(
        pool_after_withdraw.fee_protocol_token_y,
        TokenAmount::new(0)
    );
}
