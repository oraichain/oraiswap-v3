use crate::contract::{execute, instantiate, query};
use crate::error::ContractError;
use crate::math::types::percentage::Percentage;
use crate::math::types::sqrt_price::calculate_sqrt_price;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::{FeeTier, Pool, PoolKey};
use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
use cosmwasm_std::{from_binary, Addr};
use decimal::Decimal;

#[test]
fn test_change_fee_reciever() -> Result<(), ContractError> {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let admin = "admin";

    let instantiate_msg = InstantiateMsg {
        protocol_fee: Percentage::new(0),
    };

    let info = mock_info(admin, &[]);
    instantiate(deps.as_mut(), env.clone(), info.clone(), instantiate_msg)?;

    let token_x = Addr::unchecked("token_x");
    let token_y = Addr::unchecked("token_y");
    let fee_tier = FeeTier::new(Percentage::new(1), 1).unwrap();
    let init_tick = 0;
    let init_sqrt_price = calculate_sqrt_price(init_tick).unwrap();

    let alice = Addr::unchecked("alice");

    let execute_msg = ExecuteMsg::AddFeeTier {
        fee_tier: fee_tier.clone(),
    };
    execute(deps.as_mut(), env.clone(), info.clone(), execute_msg)?;

    let execute_msg = ExecuteMsg::CreatePool {
        token_0: token_x.clone(),
        token_1: token_y.clone(),
        fee_tier: fee_tier.clone(),
        init_sqrt_price,
        init_tick,
    };
    let result = execute(deps.as_mut(), env.clone(), info.clone(), execute_msg);
    assert!(result.is_ok());

    let pool_key =
        PoolKey::new(token_x.clone(), token_y.clone(), fee_tier.clone()).unwrap();
    let execute_msg = ExecuteMsg::ChangeFeeReceiver {
        pool_key: pool_key.clone(),
        fee_receiver: alice.clone(),
    };
    let result = execute(deps.as_mut(), env.clone(), info.clone(), execute_msg);
    assert!(result.is_ok());

    let query_msg = QueryMsg::Pool {
        token_0: token_x,
        token_1: token_y,
        fee_tier: fee_tier.clone(),
    };
    let pool: Pool = from_binary(&query(deps.as_ref(), env.clone(), query_msg)?)?;
    assert_eq!(pool.fee_receiver, alice);

    Ok(())
}

#[test]
fn test_not_admin_change_fee_reciever() -> Result<(), ContractError> {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let admin = "admin";
    let user = "user";

    let instantiate_msg = InstantiateMsg {
        protocol_fee: Percentage::new(0),
    };

    let info = mock_info(admin, &[]);
    instantiate(deps.as_mut(), env.clone(), info.clone(), instantiate_msg)?;

    let token_x = Addr::unchecked("token_x");
    let token_y = Addr::unchecked("token_y");
    let fee_tier = FeeTier::new(Percentage::new(1), 100).unwrap();
    let init_tick = 0;
    let init_sqrt_price = calculate_sqrt_price(init_tick).unwrap();

    let execute_msg = ExecuteMsg::AddFeeTier {
        fee_tier: fee_tier.clone(),
    };
    execute(deps.as_mut(), env.clone(), info.clone(), execute_msg)?;

    let execute_msg = ExecuteMsg::CreatePool {
        token_0: token_x.clone(),
        token_1: token_y.clone(),
        fee_tier: fee_tier.clone(),
        init_sqrt_price,
        init_tick,
    };
    let result = execute(deps.as_mut(), env.clone(), info.clone(), execute_msg);
    assert!(result.is_ok());

    let info = mock_info(user, &[]);
    let bob = Addr::unchecked("bob");
    let pool_key =
        PoolKey::new(token_x.clone(), token_y.clone(), fee_tier.clone()).unwrap();
    let execute_msg = ExecuteMsg::ChangeFeeReceiver {
        pool_key,
        fee_receiver: bob.clone(),
    };
    let result = execute(deps.as_mut(), env.clone(), info, execute_msg).unwrap_err();
    assert!(matches!(result, ContractError::Unauthorized {}));

    Ok(())
}
