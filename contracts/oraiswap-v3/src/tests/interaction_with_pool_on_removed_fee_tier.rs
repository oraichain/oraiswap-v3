use crate::tests::helper::extract_amount;
use crate::token_amount::TokenAmount;
use crate::{
    percentage::Percentage,
    sqrt_price::calculate_sqrt_price,
    tests::helper::{macros::*, MockApp},
    FeeTier, PoolKey,
};
use cosmwasm_std::Addr;
use decimal::{Decimal, Factories};

#[test]
fn test_interaction_with_pool_on_removed_fee_tier() {
    let mut app = MockApp::new(&[]);
    let (dex, token_x, token_y) = init_dex_and_tokens!(app);
    init_basic_pool!(app, dex, token_x, token_y);
    let fee_tier = FeeTier::new(Percentage::from_scale(6, 3), 10).unwrap();
    let pool_key = PoolKey::new(token_x.clone(), token_y.clone(), fee_tier).unwrap();
    // Remove Fee Tier
    {
        remove_fee_tier!(app, dex, fee_tier, "alice").unwrap();
        let exist = fee_tier_exist!(app, dex, fee_tier);
        assert!(!exist);
    }
    // Attempt to create same pool again
    {
        let init_tick = 0;
        let init_sqrt_price = calculate_sqrt_price(init_tick).unwrap();
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
        assert!(result.is_err());
    }
    // Init  position
    {
        init_basic_position!(app, dex, token_x, token_y);
    }

    // Init swap
    {
        init_basic_swap!(app, dex, token_x, token_y);
    }

    // Claim fee
    {
        let app_res = claim_fee!(app, dex, 0, "alice").unwrap();
        let claimed_x = extract_amount(&app_res.events, "amount_x").unwrap();
        let claimed_y = extract_amount(&app_res.events, "amount_y").unwrap();
        assert_eq!(claimed_x, TokenAmount(5));
        assert_eq!(claimed_y, TokenAmount(0));
    }
    // Change fee receiver
    {
        change_fee_receiver!(app, dex, pool_key, "bob", "alice").unwrap();
    }
    // Withdraw protocol fee
    {
        withdraw_protocol_fee!(app, dex, pool_key, "bob").unwrap();
    }
    // Close position
    {
        remove_position!(app, dex, 0, "alice").unwrap();
    }
    // Get pool
    {
        get_pool!(app, dex, token_x, token_y, fee_tier).unwrap();
    }
    // Get Pools
    {
        let pools = get_pools!(app, dex, Some(1), None);
        assert_eq!(pools.len(), 1);
        assert_eq!(pools[0].pool_key, pool_key);
    }
    // Transfer position
    {
        init_basic_position!(app, dex, token_x, token_y);
        let transferred_index = 0;
        let position_owner = "alice";
        let recipient = "bob";
        let recipient_address = Addr::unchecked("bob");
        let owner_list_before = get_all_positions!(app, dex, position_owner);
        let recipient_list_before = get_all_positions!(app, dex, recipient);
        let removed_position = get_position!(app, dex, transferred_index, position_owner).unwrap();

        transfer_position!(
            app,
            dex,
            transferred_index,
            recipient_address,
            position_owner
        )
        .unwrap();

        let recipient_position = get_position!(app, dex, transferred_index, recipient).unwrap();
        let owner_list_after = get_all_positions!(app, dex, position_owner);
        let recipient_list_after = get_all_positions!(app, dex, recipient);

        assert_eq!(recipient_list_after.len(), recipient_list_before.len() + 1);
        assert_eq!(owner_list_before.len() - 1, owner_list_after.len());
        assert_eq!(owner_list_after.len(), 0);

        // Equals fields of transferred position
        positions_equals!(recipient_position, removed_position);
    }
    // Readd fee tier and create same pool
    {
        let deployer = "alice";
        add_fee_tier!(app, dex, fee_tier, deployer).unwrap();
        let init_tick = 0;
        let init_sqrt_price = calculate_sqrt_price(init_tick).unwrap();
        let result = create_pool!(
            app,
            dex,
            token_x,
            token_y,
            fee_tier,
            init_sqrt_price,
            init_tick,
            deployer
        );
        assert!(result.is_err());
    }
}
