use cosmwasm_std::coin;
use decimal::*;

use crate::{
    percentage::Percentage,
    tests::helper::{macros::*, MockApp},
    FeeTier,
};

#[test]
fn test_remove_fee_tier() {
    let protocol_fee = Percentage::new(0);
    let initial_amount = 10u128.pow(10);
    let mut app = MockApp::new(&[("alice", &[coin(initial_amount, "orai")])]);
    app.set_token_contract(Box::new(crate::create_entry_points_testing!(cw20_base)));
    let dex = app.create_dex("alice", protocol_fee).unwrap();

    let fee_tier = FeeTier::new(Percentage::from_scale(2, 4), 1).unwrap();
    add_fee_tier!(app, dex, fee_tier, "alice").unwrap();

    let fee_tier = FeeTier::new(Percentage::from_scale(2, 4), 2).unwrap();
    add_fee_tier!(app, dex, fee_tier, "alice").unwrap();

    remove_fee_tier!(app, dex, fee_tier, "alice").unwrap();
    let exist = fee_tier_exist!(
        app,
        dex,
        FeeTier::new(Percentage::from_scale(2, 4), 2).unwrap()
    );
    assert!(!exist);
}

#[test]
fn test_remove_not_existing_fee_tier() {
    let protocol_fee = Percentage::new(0);
    let initial_amount = 10u128.pow(10);
    let mut app = MockApp::new(&[("alice", &[coin(initial_amount, "orai")])]);
    app.set_token_contract(Box::new(crate::create_entry_points_testing!(cw20_base)));
    let dex = app.create_dex("alice", protocol_fee).unwrap();

    let fee_tier = FeeTier::new(Percentage::from_scale(2, 4), 1).unwrap();
    add_fee_tier!(app, dex, fee_tier, "alice").unwrap();

    let fee_tier = FeeTier::new(Percentage::from_scale(2, 4), 2).unwrap();
    remove_fee_tier!(app, dex, fee_tier, "alice").unwrap_err();
}

#[test]
fn test_remove_fee_tier_not_admin() {
    let protocol_fee = Percentage::new(0);
    let initial_amount = 10u128.pow(10);
    let mut app = MockApp::new(&[("alice", &[coin(initial_amount, "orai")])]);
    app.set_token_contract(Box::new(crate::create_entry_points_testing!(cw20_base)));
    let dex = app.create_dex("alice", protocol_fee).unwrap();

    let fee_tier = FeeTier::new(Percentage::from_scale(2, 4), 1).unwrap();
    add_fee_tier!(app, dex, fee_tier, "alice").unwrap();

    let fee_tier = FeeTier::new(Percentage::from_scale(2, 4), 2).unwrap();
    add_fee_tier!(app, dex, fee_tier, "alice").unwrap();

    remove_fee_tier!(app, dex, fee_tier, "bob").unwrap_err();
}
