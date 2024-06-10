use decimal::{Decimal, Factories};

use crate::{
    percentage::Percentage,
    tests::helper::{macros::*, MockApp},
    token_amount::TokenAmount,
    FeeTier,
};

#[test]
fn test_claim() {
    let mut app = MockApp::new(&[]);
    let (dex, token_x, token_y) = init_dex_and_tokens!(app);
    init_basic_pool!(app, dex, token_x, token_y);
    init_basic_position!(app, dex, token_x, token_y);
    init_basic_swap!(app, dex, token_x, token_y);

    let fee_tier = FeeTier::new(Percentage::from_scale(6, 3), 10).unwrap();

    let pool = get_pool!(app, dex, token_x, token_y, fee_tier).unwrap();
    let user_amount_before_claim = balance_of!(app, token_x, "alice");
    let dex_amount_before_claim = balance_of!(app, token_x, dex);

    claim_fee!(app, dex, 0, "alice").unwrap();

    let user_amount_after_claim = balance_of!(app, token_x, "alice");
    let dex_amount_after_claim = balance_of!(app, token_x, dex);
    let position = get_position!(app, dex, 0, "alice").unwrap();
    let expected_tokens_claimed = 5;

    assert_eq!(
        user_amount_after_claim - expected_tokens_claimed,
        user_amount_before_claim
    );
    assert_eq!(
        dex_amount_after_claim + expected_tokens_claimed,
        dex_amount_before_claim
    );
    assert_eq!(position.fee_growth_inside_x, pool.fee_growth_global_x);
    assert_eq!(position.tokens_owed_x, TokenAmount(0));
}

#[test]
fn test_claim_not_owner() {
    let mut app = MockApp::new(&[]);
    let (dex, token_x, token_y) = init_dex_and_tokens!(app);
    init_basic_pool!(app, dex, token_x, token_y);
    init_basic_position!(app, dex, token_x, token_y);
    init_basic_swap!(app, dex, token_x, token_y);

    claim_fee!(app, dex, 0, "bob").unwrap_err();
}
