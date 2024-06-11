use crate::math::types::percentage::Percentage;
use crate::msg::QueryMsg;
use crate::tests::helper::{macros::*, MockApp};
use crate::FeeTier;
use decimal::Decimal;

#[test]
fn test_add_multiple_fee_tiers() {
    let mut app: MockApp = MockApp::new(&[("alice", &[])]);
    let dex = create_dex!(app, Percentage::new(0));

    let first_fee_tier = FeeTier::new(Percentage::new(1), 1).unwrap();
    let second_fee_tier = FeeTier::new(Percentage::new(1), 2).unwrap();
    let third_fee_tier = FeeTier::new(Percentage::new(1), 4).unwrap();

    add_fee_tier!(app, dex, first_fee_tier, "alice").unwrap();
    add_fee_tier!(app, dex, second_fee_tier, "alice").unwrap();
    add_fee_tier!(app, dex, third_fee_tier, "alice").unwrap();

    let fee_tiers: Vec<FeeTier> = app.query(dex.clone(), &QueryMsg::FeeTiers {}).unwrap();
    assert!(fee_tiers.contains(&first_fee_tier));
    assert!(fee_tiers.contains(&second_fee_tier));
    assert!(fee_tiers.contains(&third_fee_tier));
    assert_eq!(fee_tiers.len(), 3);
}

#[test]
fn test_add_fee_tier_not_admin() {
    let admin = "alice";
    let not_admin = "user";
    let mut app = MockApp::new(&[(admin, &[]), ("user", &[])]);
    let dex = create_dex!(app, Percentage::new(0));
    let result = add_fee_tier!(
        app,
        dex,
        FeeTier::new(Percentage::new(1), 1).unwrap(),
        not_admin
    );
    assert!(result.is_err());
}

#[test]
fn test_add_fee_tier_zero_fee() {
    let mut app = MockApp::new(&[("alice", &[])]);
    let dex = create_dex!(app, Percentage::new(0));
    let result = add_fee_tier!(
        app,
        dex,
        FeeTier::new(Percentage::new(0), 10).unwrap(),
        "alice"
    );
    assert!(result.is_ok());
}

#[test]
fn test_add_fee_tier_tick_spacing_zero() {
    let mut app = MockApp::new(&[("alice", &[])]);
    let dex = create_dex!(app, Percentage::new(0));
    let fee_tier = FeeTier {
        fee: Percentage::new(1),
        tick_spacing: 0,
    };
    let result = add_fee_tier!(app, dex, fee_tier, "alice");
    assert!(result.is_err());
}

#[test]
fn test_add_fee_tier_over_upper_bound_tick_spacing() {
    let mut app = MockApp::new(&[("alice", &[])]);
    let dex = create_dex!(app, Percentage::new(0));
    let fee_tier = FeeTier {
        fee: Percentage::new(1),
        tick_spacing: 101,
    };
    let result = add_fee_tier!(app, dex, fee_tier, "alice");
    assert!(result.is_err());
}

#[test]
fn test_add_fee_tier_fee_above_limit() {
    let mut app = MockApp::new(&[("alice", &[])]);
    let dex = create_dex!(app, Percentage::new(0));
    let fee_tier = FeeTier {
        fee: Percentage::new(1000000000000),
        tick_spacing: 10,
    };
    let result = add_fee_tier!(app, dex, fee_tier, "alice");
    assert!(result.is_err());
}
