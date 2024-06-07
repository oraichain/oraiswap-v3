use crate::contract::{execute, instantiate};
use crate::error::ContractError;
use crate::math::types::percentage::Percentage;
use crate::msg::{ExecuteMsg, InstantiateMsg};
use crate::state::CONFIG;
use crate::{Config, FeeTier};
use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
use decimal::Decimal;

#[test]
fn test_add_multiple_fee_tiers() -> Result<(), ContractError> {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let admin = "admin";

    let instantiate_msg = InstantiateMsg {
        protocol_fee: Percentage::new(0),
    };

    let info = mock_info(admin, &[]);
    instantiate(deps.as_mut(), env.clone(), info.clone(), instantiate_msg)?;

    let first_fee_tier = FeeTier::new(Percentage::new(1), 1).unwrap();
    let execute_msg = ExecuteMsg::AddFeeTier {
        fee_tier: first_fee_tier,
    };
    execute(deps.as_mut(), env.clone(), info.clone(), execute_msg)?;

    let second_fee_tier = FeeTier::new(Percentage::new(1), 2).unwrap();
    let execute_msg = ExecuteMsg::AddFeeTier {
        fee_tier: second_fee_tier.clone(),
    };
    execute(deps.as_mut(), env.clone(), info.clone(), execute_msg)?;

    let third_fee_tier = FeeTier::new(Percentage::new(1), 4).unwrap();
    let execute_msg = ExecuteMsg::AddFeeTier {
        fee_tier: third_fee_tier.clone(),
    };
    execute(deps.as_mut(), env.clone(), info.clone(), execute_msg)?;

    let config: Config = CONFIG.load(deps.as_ref().storage)?;
    assert!(config.fee_tiers.contains(&first_fee_tier));
    assert!(config.fee_tiers.contains(&second_fee_tier));
    assert!(config.fee_tiers.contains(&third_fee_tier));
    assert_eq!(config.fee_tiers.len(), 3);

    Ok(())
}

#[test]
fn test_add_fee_tier_not_admin() -> Result<(), ContractError> {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let admin = "admin";
    let user = "user";

    let instantiate_msg = InstantiateMsg {
        protocol_fee: Percentage::new(0),
    };

    let info = mock_info(admin, &[]);
    instantiate(deps.as_mut(), env.clone(), info.clone(), instantiate_msg)?;

    let fee_tier = FeeTier::new(Percentage::new(1), 1).unwrap();
    let info = mock_info(user, &[]);
    let execute_msg = ExecuteMsg::AddFeeTier { fee_tier };
    let result = execute(deps.as_mut(), env.clone(), info, execute_msg).unwrap_err();

    assert!(matches!(result, ContractError::Unauthorized {}));

    Ok(())
}

#[test]
fn test_add_fee_tier_zero_fee() -> Result<(), ContractError> {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let admin = "admin";

    let instantiate_msg = InstantiateMsg {
        protocol_fee: Percentage::new(0),
    };

    let info = mock_info(admin, &[]);
    instantiate(deps.as_mut(), env.clone(), info.clone(), instantiate_msg)?;

    let fee_tier = FeeTier::new(Percentage::new(0), 10).unwrap();
    let execute_msg = ExecuteMsg::AddFeeTier { fee_tier };
    execute(deps.as_mut(), env.clone(), info, execute_msg)?;
    Ok(())
}

#[test]
fn test_add_fee_tier_tick_spacing_zero() -> Result<(), ContractError> {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let admin = "admin";

    let instantiate_msg = InstantiateMsg {
        protocol_fee: Percentage::new(0),
    };

    let info = mock_info(admin, &[]);
    instantiate(deps.as_mut(), env.clone(), info.clone(), instantiate_msg)?;

    let fee_tier = FeeTier {
        fee: Percentage::new(1),
        tick_spacing: 0,
    };

    let execute_msg = ExecuteMsg::AddFeeTier { fee_tier };
    let result = execute(deps.as_mut(), env.clone(), info, execute_msg).unwrap_err();
    assert!(matches!(result, ContractError::InvalidTickSpacing {}));
    Ok(())
}

#[test]
fn test_add_fee_tier_over_upper_bound_tick_spacing() -> Result<(), ContractError> {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let admin = "admin";

    let instantiate_msg = InstantiateMsg {
        protocol_fee: Percentage::new(0),
    };

    let info = mock_info(admin, &[]);
    instantiate(deps.as_mut(), env.clone(), info.clone(), instantiate_msg)?;

    let fee_tier = FeeTier {
        fee: Percentage::new(1),
        tick_spacing: 101,
    };

    let execute_msg = ExecuteMsg::AddFeeTier { fee_tier };
    let result = execute(deps.as_mut(), env.clone(), info, execute_msg).unwrap_err();
    assert!(matches!(result, ContractError::InvalidTickSpacing {}));
    Ok(())
}

#[test]
fn test_add_fee_tier_fee_above_limit() -> Result<(), ContractError> {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let admin = "admin";

    let instantiate_msg = InstantiateMsg {
        protocol_fee: Percentage::new(0),
    };

    let info = mock_info(admin, &[]);
    instantiate(deps.as_mut(), env.clone(), info.clone(), instantiate_msg)?;

    let fee_tier = FeeTier {
        fee: Percentage::new(1000000000000),
        tick_spacing: 10,
    };

    let execute_msg = ExecuteMsg::AddFeeTier { fee_tier };
    let result = execute(deps.as_mut(), env.clone(), info, execute_msg).unwrap_err();

    assert!(matches!(result, ContractError::InvalidFee {}));
    Ok(())
}
