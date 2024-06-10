use decimal::*;

use crate::{
    percentage::Percentage,
    tests::helper::{macros::*, MockApp},
    FeeTier,
};

#[test]
fn test_remove_fee_tier() {
    let protocol_fee = Percentage::new(0);
    let (mut app, dex) = create_dex!(protocol_fee);

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
    let (mut app, dex) = create_dex!(protocol_fee);

    let fee_tier = FeeTier::new(Percentage::from_scale(2, 4), 1).unwrap();
    add_fee_tier!(app, dex, fee_tier, "alice").unwrap();

    let fee_tier = FeeTier::new(Percentage::from_scale(2, 4), 2).unwrap();
    remove_fee_tier!(app, dex, fee_tier, "alice").unwrap_err();
}

#[test]
fn test_remove_fee_tier_not_admin() {
    let protocol_fee = Percentage::new(0);
    let (mut app, dex) = create_dex!(protocol_fee);

    let fee_tier = FeeTier::new(Percentage::from_scale(2, 4), 1).unwrap();
    add_fee_tier!(app, dex, fee_tier, "alice").unwrap();

    let fee_tier = FeeTier::new(Percentage::from_scale(2, 4), 2).unwrap();
    add_fee_tier!(app, dex, fee_tier, "alice").unwrap();

    remove_fee_tier!(app, dex, fee_tier, "bob").unwrap_err();
}
