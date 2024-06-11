use crate::math::types::percentage::Percentage;
use crate::math::types::sqrt_price::calculate_sqrt_price;
use crate::msg::ExecuteMsg;
use crate::tests::helper::{macros::*, MockApp};
use crate::{FeeTier, PoolKey};
use cosmwasm_std::Addr;
use decimal::Decimal;

#[test]
fn test_change_fee_reciever() {
    let admin = "admin";
    let new_fee_receiver = "alice";
    let mut app: MockApp = MockApp::new(&[(admin, &[])]);
    let dex = create_dex!(app, Percentage::new(0));

    let (token_x, token_y) = create_tokens!(app, 0, 0);
    let fee_tier = FeeTier::new(Percentage::new(1), 1).unwrap();
    let init_tick = 0;
    let init_sqrt_price = calculate_sqrt_price(init_tick).unwrap();

    let result = add_fee_tier!(app, dex, fee_tier, admin);
    assert!(result.is_ok());

    let result = create_pool!(
        app,
        dex,
        token_x,
        token_y,
        fee_tier.clone(),
        init_sqrt_price,
        init_tick,
        admin
    );
    assert!(result.is_ok());

    let pool_key = PoolKey::new(token_x.clone(), token_y.clone(), fee_tier.clone()).unwrap();
    let result = app.execute(
        Addr::unchecked(admin),
        Addr::unchecked(dex.clone()),
        &ExecuteMsg::ChangeFeeReceiver {
            pool_key: pool_key.clone(),
            fee_receiver: Addr::unchecked(new_fee_receiver),
        },
        &[],
    );
    assert!(result.is_ok());

    let pool = get_pool!(app, dex, token_x, token_y, fee_tier).unwrap();
    assert_eq!(pool.fee_receiver, Addr::unchecked(new_fee_receiver));
}

#[test]
fn test_not_admin_change_fee_reciever() {
    let mut mock_app = MockApp::new(&[("admin", &[]), ("user", &[]), ("bob", &[])]);
    let admin = "admin";
    let user = "user";

    let protocol_fee = Percentage::new(0);
    let dex_addr = mock_app.create_dex(admin, protocol_fee).unwrap();

    let token_x = Addr::unchecked("token_x");
    let token_y = Addr::unchecked("token_y");
    let fee_tier = FeeTier::new(Percentage::new(1), 100).unwrap();
    let init_tick = 0;
    let init_sqrt_price = calculate_sqrt_price(init_tick).unwrap();

    let result = mock_app.add_fee_tier(admin, dex_addr.as_str(), fee_tier.clone());
    assert!(result.is_ok());

    let result = mock_app.create_pool(
        admin,
        dex_addr.as_str(),
        token_x.as_str(),
        token_y.as_str(),
        fee_tier.clone(),
        init_sqrt_price,
        init_tick,
    );
    assert!(result.is_ok());

    let pool_key = PoolKey::new(token_x.clone(), token_y.clone(), fee_tier.clone()).unwrap();
    let result = mock_app
        .execute(
            Addr::unchecked(user),
            Addr::unchecked(dex_addr.clone()),
            &ExecuteMsg::ChangeFeeReceiver {
                pool_key,
                fee_receiver: Addr::unchecked("bob"),
            },
            &[],
        )
        .unwrap_err();
    assert!(result.contains("error executing WasmMsg"));
}
