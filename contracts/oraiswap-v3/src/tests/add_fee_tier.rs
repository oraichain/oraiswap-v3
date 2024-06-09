use crate::math::types::percentage::Percentage;
use crate::msg::QueryMsg;
use crate::tests::helper::MockApp;
use crate::FeeTier;
use decimal::Decimal;

#[test]
fn test_add_multiple_fee_tiers() {
    let mut mock_app = MockApp::new(&[("admin", &[])]);
    let owner = "admin";

    let protocol_fee = Percentage::new(0);
    let dex_addr = mock_app.create_dex(owner, protocol_fee).unwrap();

    let first_fee_tier = FeeTier::new(Percentage::new(1), 1).unwrap();
    let second_fee_tier = FeeTier::new(Percentage::new(1), 2).unwrap();
    let third_fee_tier = FeeTier::new(Percentage::new(1), 4).unwrap();

    mock_app.add_fee_tier(owner, dex_addr.as_str(), first_fee_tier.clone()).unwrap();
    mock_app.add_fee_tier(owner, dex_addr.as_str(), second_fee_tier.clone()).unwrap();
    mock_app.add_fee_tier(owner, dex_addr.as_str(), third_fee_tier.clone()).unwrap();

    let fee_tiers: Vec<FeeTier> = mock_app.query(dex_addr.clone(), &QueryMsg::FeeTiers {}).unwrap();
    assert!(fee_tiers.contains(&first_fee_tier));
    assert!(fee_tiers.contains(&second_fee_tier));
    assert!(fee_tiers.contains(&third_fee_tier));
    assert_eq!(fee_tiers.len(), 3);
}

#[test]
fn test_add_fee_tier_not_admin() {
    let mut mock_app = MockApp::new(&[("admin", &[]), ("user", &[])]);
    let admin = "admin";
    let user = "user";

    let protocol_fee = Percentage::new(0);
    let dex_addr = mock_app.create_dex(admin, protocol_fee).unwrap();

    let fee_tier = FeeTier::new(Percentage::new(1), 1).unwrap();
    let result = mock_app.add_fee_tier(user, dex_addr.as_str(), fee_tier).unwrap_err();
    assert!(result.contains("error executing WasmMsg"));
}

#[test]
fn test_add_fee_tier_zero_fee() {
    let mut mock_app = MockApp::new(&[("admin", &[])]);
    let admin = "admin";

    let protocol_fee = Percentage::new(0);
    let dex_addr = mock_app.create_dex(admin, protocol_fee).unwrap();

    let fee_tier = FeeTier::new(Percentage::new(0), 10).unwrap();
    let result = mock_app.add_fee_tier(admin, dex_addr.as_str(), fee_tier);
    assert!(result.is_ok());
}

#[test]
fn test_add_fee_tier_tick_spacing_zero() {
    let mut mock_app = MockApp::new(&[("admin", &[])]);
    let admin = "admin";

    let protocol_fee = Percentage::new(0);
    let dex_addr = mock_app.create_dex(admin, protocol_fee).unwrap();

    let fee_tier = FeeTier {
        fee: Percentage::new(1),
        tick_spacing: 0,
    };
    let result = mock_app.add_fee_tier(admin, dex_addr.as_str(), fee_tier).unwrap_err();

    assert!(result.contains("error executing WasmMsg"));
}

#[test]
fn test_add_fee_tier_over_upper_bound_tick_spacing() {
    let mut mock_app = MockApp::new(&[("admin", &[])]);
    let admin = "admin";

    let protocol_fee = Percentage::new(0);
    let dex_addr = mock_app.create_dex(admin, protocol_fee).unwrap();

    let fee_tier = FeeTier {
        fee: Percentage::new(1),
        tick_spacing: 101,
    };
    let result = mock_app.add_fee_tier(admin, dex_addr.as_str(), fee_tier).unwrap_err();

    assert!(result.contains("error executing WasmMsg"));
}

#[test]
fn test_add_fee_tier_fee_above_limit() {
    let mut mock_app = MockApp::new(&[("admin", &[])]);
    let admin = "admin";

    let protocol_fee = Percentage::new(0);
    let dex_addr = mock_app.create_dex(admin, protocol_fee).unwrap();

    let fee_tier = FeeTier {
        fee: Percentage::new(1000000000000),
        tick_spacing: 10,
    };
    let result = mock_app.add_fee_tier(admin, dex_addr.as_str(), fee_tier).unwrap_err();

    assert!(result.contains("error executing WasmMsg"));
}
