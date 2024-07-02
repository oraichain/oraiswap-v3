use cosmwasm_std::Addr;
use decimal::*;

use crate::{
    fee_growth::FeeGrowth,
    interface::{NftInfoResponse, TokensResponse},
    liquidity::Liquidity,
    msg,
    percentage::Percentage,
    sqrt_price::{calculate_sqrt_price, SqrtPrice},
    state,
    tests::helper::{macros::*, MockApp},
    token_amount::TokenAmount,
    FeeTier, PoolKey, MIN_SQRT_PRICE,
};

#[test]
fn test_mint_nft() {
    let mut app = MockApp::new(&[]);
    let dex = create_dex!(app, Percentage::new(0));
    let (token_x, token_y) = create_tokens!(app, 500, 500);

    let fee_tier = FeeTier::new(Percentage::new(0), 1).unwrap();

    add_fee_tier!(app, dex, fee_tier, "alice").unwrap();

    let init_tick = 10;
    let init_sqrt_price = calculate_sqrt_price(init_tick).unwrap();
    create_pool!(
        app,
        dex,
        token_x,
        token_y,
        fee_tier,
        init_sqrt_price,
        init_tick,
        "alice"
    )
    .unwrap();

    approve!(app, token_x, dex, 500, "alice").unwrap();
    approve!(app, token_y, dex, 500, "alice").unwrap();

    let pool_key = PoolKey::new(token_x.to_string(), token_y.to_string(), fee_tier).unwrap();

    app.execute(
        Addr::unchecked("alice"),
        dex.clone(),
        &msg::ExecuteMsg::Mint {
            extension: msg::NftExtensionMsg {
                pool_key,
                lower_tick: -10,
                upper_tick: 10,
                liquidity_delta: Liquidity::new(10),
                slippage_limit_lower: SqrtPrice::new(0),
                slippage_limit_upper: SqrtPrice::max_instance(),
            },
        },
        &[],
    )
    .unwrap();
}

#[test]
fn test_query_nft() {
    let mut app = MockApp::new(&[]);
    let dex = create_dex!(app, Percentage::new(0));
    let (token_x, token_y) = create_tokens!(app, 500, 500);

    let fee_tier = FeeTier::new(Percentage::new(0), 1).unwrap();

    add_fee_tier!(app, dex, fee_tier, "alice").unwrap();

    let init_tick = 10;
    let init_sqrt_price = calculate_sqrt_price(init_tick).unwrap();
    create_pool!(
        app,
        dex,
        token_x,
        token_y,
        fee_tier,
        init_sqrt_price,
        init_tick,
        "alice"
    )
    .unwrap();

    approve!(app, token_x, dex, 500, "alice").unwrap();
    approve!(app, token_y, dex, 500, "alice").unwrap();

    let pool_key = PoolKey::new(token_x.to_string(), token_y.to_string(), fee_tier).unwrap();

    app.execute(
        Addr::unchecked("alice"),
        dex.clone(),
        &msg::ExecuteMsg::Mint {
            extension: msg::NftExtensionMsg {
                pool_key: pool_key.clone(),
                lower_tick: -10,
                upper_tick: 10,
                liquidity_delta: Liquidity::new(10),
                slippage_limit_lower: SqrtPrice::new(0),
                slippage_limit_upper: SqrtPrice::max_instance(),
            },
        },
        &[],
    )
    .unwrap();

    let token_id = state::position_key(&Addr::unchecked("alice"), 0).into();

    let nft_info: NftInfoResponse = app
        .query(dex.clone(), &msg::QueryMsg::NftInfo { token_id })
        .unwrap();

    assert_eq!(nft_info.extension.pool_key, pool_key);

    let TokensResponse { tokens } = app
        .query(
            dex.clone(),
            &msg::QueryMsg::Tokens {
                owner: Addr::unchecked("alice"),
                start_after: None,
                limit: None,
            },
        )
        .unwrap();
    assert_eq!(tokens.len(), 1)
}

#[test]
fn test_burn_nft() {
    let fee_tier = FeeTier::new(Percentage::from_scale(6, 3), 10).unwrap();

    let init_tick = 0;
    let init_sqrt_price = calculate_sqrt_price(init_tick).unwrap();
    let remove_position_index = 0;

    let initial_mint = 10u128.pow(10);

    let mut app = MockApp::new(&[]);
    let dex = create_dex!(app, Percentage::from_scale(1, 2));

    let (token_x, token_y) = create_tokens!(app, initial_mint, initial_mint);

    let pool_key = PoolKey::new(token_x.to_string(), token_y.to_string(), fee_tier).unwrap();

    add_fee_tier!(app, dex, fee_tier, "alice").unwrap();

    create_pool!(
        app,
        dex,
        token_x,
        token_y,
        fee_tier,
        init_sqrt_price,
        init_tick,
        "alice"
    )
    .unwrap();

    let lower_tick_index = -20;
    let upper_tick_index = 10;
    let liquidity_delta = Liquidity::from_integer(1_000_000);

    approve!(app, token_x, dex, initial_mint, "alice").unwrap();
    approve!(app, token_y, dex, initial_mint, "alice").unwrap();

    let pool_state = get_pool!(app, dex, token_x, token_y, fee_tier).unwrap();

    app.execute(
        Addr::unchecked("alice"),
        dex.clone(),
        &msg::ExecuteMsg::Mint {
            extension: msg::NftExtensionMsg {
                pool_key: pool_key.clone(),
                lower_tick: lower_tick_index,
                upper_tick: upper_tick_index,
                liquidity_delta,
                slippage_limit_lower: pool_state.sqrt_price,
                slippage_limit_upper: pool_state.sqrt_price,
            },
        },
        &[],
    )
    .unwrap();

    let pool_state = get_pool!(app, dex, token_x, token_y, fee_tier).unwrap();

    assert_eq!(pool_state.liquidity, liquidity_delta);

    let liquidity_delta = Liquidity::new(liquidity_delta.get() * 1_000_000);

    let incorrect_lower_tick_index = lower_tick_index - 50;
    let incorrect_upper_tick_index = upper_tick_index + 50;

    approve!(app, token_x, dex, liquidity_delta.0, "alice").unwrap();
    approve!(app, token_y, dex, liquidity_delta.0, "alice").unwrap();

    app.execute(
        Addr::unchecked("alice"),
        dex.clone(),
        &msg::ExecuteMsg::Mint {
            extension: msg::NftExtensionMsg {
                pool_key: pool_key.clone(),
                lower_tick: incorrect_lower_tick_index,
                upper_tick: incorrect_upper_tick_index,
                liquidity_delta,
                slippage_limit_lower: pool_state.sqrt_price,
                slippage_limit_upper: pool_state.sqrt_price,
            },
        },
        &[],
    )
    .unwrap();

    let token_id = state::position_key(&Addr::unchecked("alice"), 1).into();
    let NftInfoResponse {
        extension: position_state,
    } = app
        .query(dex.clone(), &msg::QueryMsg::NftInfo { token_id })
        .unwrap();

    // Check position
    assert!(position_state.lower_tick_index == incorrect_lower_tick_index);
    assert!(position_state.upper_tick_index == incorrect_upper_tick_index);

    let amount = 1000;
    mint!(app, token_x, "bob", amount, "alice").unwrap();
    let amount_x = balance_of!(app, token_x, "bob");
    assert_eq!(amount_x, amount);

    approve!(app, token_x, dex, amount, "bob").unwrap();

    let pool_state_before = get_pool!(app, dex, token_x, token_y, fee_tier).unwrap();

    let swap_amount = TokenAmount::new(amount);
    let slippage = SqrtPrice::new(MIN_SQRT_PRICE);
    swap!(app, dex, pool_key, true, swap_amount, true, slippage, "bob").unwrap();

    let pool_state_after = get_pool!(app, dex, token_x, token_y, fee_tier).unwrap();
    assert_eq!(
        pool_state_after.fee_growth_global_x,
        FeeGrowth::new(49999950000049999)
    );
    assert_eq!(pool_state_after.fee_protocol_token_x, TokenAmount(1));
    assert_eq!(pool_state_after.fee_protocol_token_y, TokenAmount(0));

    assert!(pool_state_after
        .sqrt_price
        .lt(&pool_state_before.sqrt_price));

    assert_eq!(pool_state_after.liquidity, pool_state_before.liquidity);
    assert_eq!(pool_state_after.current_tick_index, -10);
    assert_ne!(pool_state_after.sqrt_price, pool_state_before.sqrt_price);

    let amount_x = balance_of!(app, token_x, "bob");
    let amount_y = balance_of!(app, token_y, "bob");
    assert_eq!(amount_x, 0);
    assert_eq!(amount_y, 993);

    // pre load dex balances
    let dex_x_before_remove = balance_of!(app, token_x, dex);
    let dex_y_before_remove = balance_of!(app, token_y, dex);

    // Remove position
    let sender = Addr::unchecked("alice");
    let token_id = state::position_key(&sender, remove_position_index).into();
    // remove_position!(app, dex, remove_position_index, "alice").unwrap();
    app.execute(
        sender,
        dex.clone(),
        &msg::ExecuteMsg::Burn { token_id },
        &[],
    )
    .unwrap();

    // Load states
    let pool_state = get_pool!(app, dex, token_x, token_y, fee_tier).unwrap();
    // Check ticks
    get_tick!(app, dex, pool_key, lower_tick_index).unwrap_err();
    get_tick!(app, dex, pool_key, upper_tick_index).unwrap_err();
    let lower_tick_bit = is_tick_initialized!(app, dex, pool_key, lower_tick_index);

    let upper_tick_bit = is_tick_initialized!(app, dex, pool_key, upper_tick_index);
    let dex_x = balance_of!(app, token_x, dex);
    let dex_y = balance_of!(app, token_y, dex);
    let expected_withdrawn_x = 499;
    let expected_withdrawn_y = 999;
    let expected_fee_x = 0;

    assert_eq!(
        dex_x_before_remove - dex_x,
        expected_withdrawn_x + expected_fee_x
    );
    assert_eq!(dex_y_before_remove - dex_y, expected_withdrawn_y);

    // Check tickmap
    assert!(!lower_tick_bit);
    assert!(!upper_tick_bit);

    // Check pool
    assert!(pool_state.liquidity == liquidity_delta);
    assert!(pool_state.current_tick_index == -10);
}

#[test]
fn test_transfer_nft() {
    let init_tick = -23028;
    let init_sqrt_price = calculate_sqrt_price(init_tick).unwrap();

    let mut app = MockApp::new(&[]);
    let dex = create_dex!(app, Percentage::new(0));
    let initial_balance = 10u128.pow(10);

    let (token_x, token_y) = create_tokens!(app, initial_balance, initial_balance);

    let fee_tier = FeeTier::new(Percentage::from_scale(2, 4), 3).unwrap();

    add_fee_tier!(app, dex, fee_tier, "alice").unwrap();

    create_pool!(
        app,
        dex,
        token_x,
        token_y,
        fee_tier,
        init_sqrt_price,
        init_tick,
        "alice"
    )
    .unwrap();

    approve!(app, token_x, dex, initial_balance, "alice").unwrap();
    approve!(app, token_y, dex, initial_balance, "alice").unwrap();

    let pool_key = PoolKey::new(token_x.to_string(), token_y.to_string(), fee_tier).unwrap();
    let tick_indexes = [-9780, -42, 0, 9, 276, 32343, -50001];
    let liquidity_delta = Liquidity::from_integer(1_000_000);
    let pool_state = get_pool!(app, dex, token_x, token_y, fee_tier).unwrap();
    {
        app.execute(
            Addr::unchecked("alice"),
            dex.clone(),
            &msg::ExecuteMsg::Mint {
                extension: msg::NftExtensionMsg {
                    pool_key: pool_key.clone(),
                    lower_tick: tick_indexes[0],
                    upper_tick: tick_indexes[1],
                    liquidity_delta,
                    slippage_limit_lower: pool_state.sqrt_price,
                    slippage_limit_upper: SqrtPrice::max_instance(),
                },
            },
            &[],
        )
        .unwrap();

        let TokensResponse { tokens } = app
            .query(
                dex.clone(),
                &msg::QueryMsg::Tokens {
                    owner: Addr::unchecked("alice"),
                    start_after: None,
                    limit: None,
                },
            )
            .unwrap();

        assert_eq!(tokens.len(), 1)
    }

    // Open  additional positions
    {
        app.execute(
            Addr::unchecked("alice"),
            dex.clone(),
            &msg::ExecuteMsg::Mint {
                extension: msg::NftExtensionMsg {
                    pool_key: pool_key.clone(),
                    lower_tick: tick_indexes[0],
                    upper_tick: tick_indexes[1],
                    liquidity_delta,
                    slippage_limit_lower: pool_state.sqrt_price,
                    slippage_limit_upper: SqrtPrice::max_instance(),
                },
            },
            &[],
        )
        .unwrap();
        app.execute(
            Addr::unchecked("alice"),
            dex.clone(),
            &msg::ExecuteMsg::Mint {
                extension: msg::NftExtensionMsg {
                    pool_key: pool_key.clone(),
                    lower_tick: tick_indexes[1],
                    upper_tick: tick_indexes[2],
                    liquidity_delta,
                    slippage_limit_lower: pool_state.sqrt_price,
                    slippage_limit_upper: SqrtPrice::max_instance(),
                },
            },
            &[],
        )
        .unwrap();
        app.execute(
            Addr::unchecked("alice"),
            dex.clone(),
            &msg::ExecuteMsg::Mint {
                extension: msg::NftExtensionMsg {
                    pool_key: pool_key.clone(),
                    lower_tick: tick_indexes[2],
                    upper_tick: tick_indexes[3],
                    liquidity_delta,
                    slippage_limit_lower: pool_state.sqrt_price,
                    slippage_limit_upper: SqrtPrice::max_instance(),
                },
            },
            &[],
        )
        .unwrap();
    }
    // Transfer first position
    {
        let transferred_index = 0;
        let TokensResponse {
            tokens: owner_list_before,
        } = app
            .query(
                dex.clone(),
                &msg::QueryMsg::Tokens {
                    owner: Addr::unchecked("alice"),
                    start_after: None,
                    limit: None,
                },
            )
            .unwrap();
        let TokensResponse {
            tokens: recipient_list_before,
        } = app
            .query(
                dex.clone(),
                &msg::QueryMsg::Tokens {
                    owner: Addr::unchecked("bob"),
                    start_after: None,
                    limit: None,
                },
            )
            .unwrap();
        let token_id = state::position_key(&Addr::unchecked("alice"), transferred_index).into();
        let NftInfoResponse {
            extension: removed_position,
        } = app
            .query(dex.clone(), &msg::QueryMsg::NftInfo { token_id })
            .unwrap();

        let NftInfoResponse {
            extension: last_position_before,
        } = app
            .query(
                dex.clone(),
                &msg::QueryMsg::NftInfo {
                    token_id: owner_list_before[owner_list_before.len() - 1].clone(),
                },
            )
            .unwrap();

        let token_id = state::position_key(&Addr::unchecked("alice"), transferred_index).into();
        app.execute(
            Addr::unchecked("alice"),
            dex.clone(),
            &msg::ExecuteMsg::TransferNft {
                recipient: Addr::unchecked("bob"),
                token_id,
            },
            &[],
        )
        .unwrap();

        let token_id = state::position_key(&Addr::unchecked("bob"), transferred_index).into();
        let NftInfoResponse {
            extension: recipient_position,
        } = app
            .query(dex.clone(), &msg::QueryMsg::NftInfo { token_id })
            .unwrap();
        let TokensResponse {
            tokens: owner_list_after,
        } = app
            .query(
                dex.clone(),
                &msg::QueryMsg::Tokens {
                    owner: Addr::unchecked("alice"),
                    start_after: None,
                    limit: None,
                },
            )
            .unwrap();
        let TokensResponse {
            tokens: recipient_list_after,
        } = app
            .query(
                dex.clone(),
                &msg::QueryMsg::Tokens {
                    owner: Addr::unchecked("bob"),
                    start_after: None,
                    limit: None,
                },
            )
            .unwrap();
        let token_id = state::position_key(&Addr::unchecked("alice"), transferred_index).into();
        let NftInfoResponse {
            extension: owner_first_position_after,
        } = app
            .query(dex.clone(), &msg::QueryMsg::NftInfo { token_id })
            .unwrap();

        assert_eq!(recipient_list_after.len(), recipient_list_before.len() + 1);
        assert_eq!(owner_list_before.len() - 1, owner_list_after.len());

        // move last position
        positions_equals!(owner_first_position_after, last_position_before);

        // Equals fields od transferred position
        positions_equals!(recipient_position, removed_position);
    }

    // Transfer middle position
    {
        let transferred_index = 1;
        let TokensResponse {
            tokens: owner_list_before,
        } = app
            .query(
                dex.clone(),
                &msg::QueryMsg::Tokens {
                    owner: Addr::unchecked("alice"),
                    start_after: None,
                    limit: None,
                },
            )
            .unwrap();
        let TokensResponse {
            tokens: recipient_list_before,
        } = app
            .query(
                dex.clone(),
                &msg::QueryMsg::Tokens {
                    owner: Addr::unchecked("bob"),
                    start_after: None,
                    limit: None,
                },
            )
            .unwrap();

        let NftInfoResponse {
            extension: last_position_before,
        } = app
            .query(
                dex.clone(),
                &msg::QueryMsg::NftInfo {
                    token_id: owner_list_before[owner_list_before.len() - 1].clone(),
                },
            )
            .unwrap();

        let token_id = state::position_key(&Addr::unchecked("alice"), transferred_index).into();
        app.execute(
            Addr::unchecked("alice"),
            dex.clone(),
            &msg::ExecuteMsg::TransferNft {
                recipient: Addr::unchecked("bob"),
                token_id,
            },
            &[],
        )
        .unwrap();

        let TokensResponse {
            tokens: owner_list_after,
        } = app
            .query(
                dex.clone(),
                &msg::QueryMsg::Tokens {
                    owner: Addr::unchecked("alice"),
                    start_after: None,
                    limit: None,
                },
            )
            .unwrap();
        let TokensResponse {
            tokens: recipient_list_after,
        } = app
            .query(
                dex.clone(),
                &msg::QueryMsg::Tokens {
                    owner: Addr::unchecked("bob"),
                    start_after: None,
                    limit: None,
                },
            )
            .unwrap();
        let token_id = state::position_key(&Addr::unchecked("alice"), transferred_index).into();
        let NftInfoResponse {
            extension: owner_first_position_after,
        } = app
            .query(dex.clone(), &msg::QueryMsg::NftInfo { token_id })
            .unwrap();

        assert_eq!(recipient_list_after.len(), recipient_list_before.len() + 1);
        assert_eq!(owner_list_before.len() - 1, owner_list_after.len());

        // move last position
        positions_equals!(owner_first_position_after, last_position_before);
    }
    // Transfer last position
    {
        let TokensResponse {
            tokens: owner_list_before,
        } = app
            .query(
                dex.clone(),
                &msg::QueryMsg::Tokens {
                    owner: Addr::unchecked("alice"),
                    start_after: None,
                    limit: None,
                },
            )
            .unwrap();
        let transferred_index = (owner_list_before.len() - 1) as u32;
        let token_id = state::position_key(&Addr::unchecked("alice"), transferred_index).into();
        let NftInfoResponse {
            extension: removed_position,
        } = app
            .query(dex.clone(), &msg::QueryMsg::NftInfo { token_id })
            .unwrap();

        let token_id = state::position_key(&Addr::unchecked("alice"), transferred_index).into();
        app.execute(
            Addr::unchecked("alice"),
            dex.clone(),
            &msg::ExecuteMsg::TransferNft {
                recipient: Addr::unchecked("bob"),
                token_id,
            },
            &[],
        )
        .unwrap();
        let TokensResponse {
            tokens: recipient_list_after,
        } = app
            .query(
                dex.clone(),
                &msg::QueryMsg::Tokens {
                    owner: Addr::unchecked("bob"),
                    start_after: None,
                    limit: None,
                },
            )
            .unwrap();
        let recipient_position_index = (recipient_list_after.len() - 1) as u32;
        let token_id =
            state::position_key(&Addr::unchecked("bob"), recipient_position_index).into();
        let NftInfoResponse {
            extension: recipient_position,
        } = app
            .query(dex.clone(), &msg::QueryMsg::NftInfo { token_id })
            .unwrap();

        positions_equals!(removed_position, recipient_position);
    }

    // Clear position
    {
        let transferred_index = 0;
        let TokensResponse {
            tokens: recipient_list_before,
        } = app
            .query(
                dex.clone(),
                &msg::QueryMsg::Tokens {
                    owner: Addr::unchecked("bob"),
                    start_after: None,
                    limit: None,
                },
            )
            .unwrap();
        let token_id = state::position_key(&Addr::unchecked("alice"), transferred_index).into();
        let NftInfoResponse {
            extension: removed_position,
        } = app
            .query(dex.clone(), &msg::QueryMsg::NftInfo { token_id })
            .unwrap();

        let token_id = state::position_key(&Addr::unchecked("alice"), transferred_index).into();
        app.execute(
            Addr::unchecked("alice"),
            dex.clone(),
            &msg::ExecuteMsg::TransferNft {
                recipient: Addr::unchecked("bob"),
                token_id,
            },
            &[],
        )
        .unwrap();

        let TokensResponse {
            tokens: recipient_list_after,
        } = app
            .query(
                dex.clone(),
                &msg::QueryMsg::Tokens {
                    owner: Addr::unchecked("bob"),
                    start_after: None,
                    limit: None,
                },
            )
            .unwrap();
        let recipient_position_index = (recipient_list_after.len() - 1) as u32;
        let token_id =
            state::position_key(&Addr::unchecked("bob"), recipient_position_index).into();
        let NftInfoResponse {
            extension: recipient_position,
        } = app
            .query(dex.clone(), &msg::QueryMsg::NftInfo { token_id })
            .unwrap();
        let TokensResponse {
            tokens: owner_list_after,
        } = app
            .query(
                dex.clone(),
                &msg::QueryMsg::Tokens {
                    owner: Addr::unchecked("alice"),
                    start_after: None,
                    limit: None,
                },
            )
            .unwrap();

        assert_eq!(recipient_list_after.len(), recipient_list_before.len() + 1);
        assert_eq!(0, owner_list_after.len());

        // Equals fields od transferred position
        positions_equals!(recipient_position, removed_position);
    }

    // Get back position
    {
        let transferred_index = 0;
        let TokensResponse {
            tokens: owner_list_before,
        } = app
            .query(
                dex.clone(),
                &msg::QueryMsg::Tokens {
                    owner: Addr::unchecked("alice"),
                    start_after: None,
                    limit: None,
                },
            )
            .unwrap();
        let TokensResponse {
            tokens: recipient_list_before,
        } = app
            .query(
                dex.clone(),
                &msg::QueryMsg::Tokens {
                    owner: Addr::unchecked("bob"),
                    start_after: None,
                    limit: None,
                },
            )
            .unwrap();
        let token_id = state::position_key(&Addr::unchecked("bob"), transferred_index).into();
        let NftInfoResponse {
            extension: removed_position,
        } = app
            .query(dex.clone(), &msg::QueryMsg::NftInfo { token_id })
            .unwrap();

        let NftInfoResponse {
            extension: last_position_before,
        } = app
            .query(
                dex.clone(),
                &msg::QueryMsg::NftInfo {
                    token_id: recipient_list_before[recipient_list_before.len() - 1].clone(),
                },
            )
            .unwrap();

        let token_id = state::position_key(&Addr::unchecked("bob"), transferred_index).into();
        app.execute(
            Addr::unchecked("bob"),
            dex.clone(),
            &msg::ExecuteMsg::TransferNft {
                recipient: Addr::unchecked("alice"),
                token_id,
            },
            &[],
        )
        .unwrap();

        let TokensResponse {
            tokens: owner_list_after,
        } = app
            .query(
                dex.clone(),
                &msg::QueryMsg::Tokens {
                    owner: Addr::unchecked("alice"),
                    start_after: None,
                    limit: None,
                },
            )
            .unwrap();
        let TokensResponse {
            tokens: recipient_list_after,
        } = app
            .query(
                dex.clone(),
                &msg::QueryMsg::Tokens {
                    owner: Addr::unchecked("bob"),
                    start_after: None,
                    limit: None,
                },
            )
            .unwrap();
        let token_id = state::position_key(&Addr::unchecked("bob"), transferred_index).into();
        let NftInfoResponse {
            extension: recipient_first_position_after,
        } = app
            .query(dex.clone(), &msg::QueryMsg::NftInfo { token_id })
            .unwrap();

        let token_id = state::position_key(&Addr::unchecked("alice"), transferred_index).into();
        let NftInfoResponse {
            extension: owner_new_position,
        } = app
            .query(dex.clone(), &msg::QueryMsg::NftInfo { token_id })
            .unwrap();

        assert_eq!(recipient_list_after.len(), recipient_list_before.len() - 1);
        assert_eq!(owner_list_before.len() + 1, owner_list_after.len());

        // move last position
        positions_equals!(last_position_before, recipient_first_position_after);

        // Equals fields od transferred position
        positions_equals!(owner_new_position, removed_position);
    }
}

#[test]
fn test_only_owner_can_transfer_nft() {
    let init_tick = -23028;
    let init_sqrt_price = calculate_sqrt_price(init_tick).unwrap();

    let mut app = MockApp::new(&[]);
    let dex = create_dex!(app, Percentage::new(0));
    let initial_balance = 10u128.pow(10);

    let (token_x, token_y) = create_tokens!(app, initial_balance, initial_balance);

    let fee_tier = FeeTier::new(Percentage::from_scale(2, 4), 3).unwrap();

    add_fee_tier!(app, dex, fee_tier, "alice").unwrap();

    create_pool!(
        app,
        dex,
        token_x,
        token_y,
        fee_tier,
        init_sqrt_price,
        init_tick,
        "alice"
    )
    .unwrap();

    approve!(app, token_x, dex, initial_balance, "alice").unwrap();
    approve!(app, token_y, dex, initial_balance, "alice").unwrap();

    let pool_key = PoolKey::new(token_x.to_string(), token_y.to_string(), fee_tier).unwrap();
    let tick_indexes = [-9780, -42, 0, 9, 276, 32343, -50001];
    let liquidity_delta = Liquidity::from_integer(1_000_000);
    let pool_state = get_pool!(app, dex, token_x, token_y, fee_tier).unwrap();
    {
        app.execute(
            Addr::unchecked("alice"),
            dex.clone(),
            &msg::ExecuteMsg::Mint {
                extension: msg::NftExtensionMsg {
                    pool_key: pool_key.clone(),
                    lower_tick: tick_indexes[0],
                    upper_tick: tick_indexes[1],
                    liquidity_delta,
                    slippage_limit_lower: pool_state.sqrt_price,
                    slippage_limit_upper: SqrtPrice::max_instance(),
                },
            },
            &[],
        )
        .unwrap();

        let TokensResponse { tokens } = app
            .query(
                dex.clone(),
                &msg::QueryMsg::Tokens {
                    owner: Addr::unchecked("alice"),
                    start_after: None,
                    limit: None,
                },
            )
            .unwrap();

        assert_eq!(tokens.len(), 1)
    }

    // Open  additional positions
    {
        app.execute(
            Addr::unchecked("alice"),
            dex.clone(),
            &msg::ExecuteMsg::Mint {
                extension: msg::NftExtensionMsg {
                    pool_key: pool_key.clone(),
                    lower_tick: tick_indexes[0],
                    upper_tick: tick_indexes[1],
                    liquidity_delta,
                    slippage_limit_lower: pool_state.sqrt_price,
                    slippage_limit_upper: SqrtPrice::max_instance(),
                },
            },
            &[],
        )
        .unwrap();
        app.execute(
            Addr::unchecked("alice"),
            dex.clone(),
            &msg::ExecuteMsg::Mint {
                extension: msg::NftExtensionMsg {
                    pool_key: pool_key.clone(),
                    lower_tick: tick_indexes[1],
                    upper_tick: tick_indexes[2],
                    liquidity_delta,
                    slippage_limit_lower: pool_state.sqrt_price,
                    slippage_limit_upper: SqrtPrice::max_instance(),
                },
            },
            &[],
        )
        .unwrap();
        app.execute(
            Addr::unchecked("alice"),
            dex.clone(),
            &msg::ExecuteMsg::Mint {
                extension: msg::NftExtensionMsg {
                    pool_key: pool_key.clone(),
                    lower_tick: tick_indexes[2],
                    upper_tick: tick_indexes[3],
                    liquidity_delta,
                    slippage_limit_lower: pool_state.sqrt_price,
                    slippage_limit_upper: SqrtPrice::max_instance(),
                },
            },
            &[],
        )
        .unwrap();
    }
    // Transfer first position
    {
        let transferred_index = 0;
        let token_id = state::position_key(&Addr::unchecked("bob"), transferred_index).into();
        app.execute(
            Addr::unchecked("bob"),
            dex.clone(),
            &msg::ExecuteMsg::TransferNft {
                recipient: Addr::unchecked("alice"),
                token_id,
            },
            &[],
        )
        .unwrap_err();
    }
}
