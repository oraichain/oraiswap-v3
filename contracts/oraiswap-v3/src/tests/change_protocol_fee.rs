use crate::contract::{execute, instantiate, query};
use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::percentage::Percentage;
use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
use cosmwasm_std::from_binary;
use decimal::Decimal;

#[test]
fn test_change_protocol_fee() -> Result<(), ContractError> {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let admin = "admin";

    let instantiate_msg = InstantiateMsg {
        protocol_fee: Percentage::new(0),
    };

    let info = mock_info(admin, &[]);
    instantiate(deps.as_mut(), env.clone(), info.clone(), instantiate_msg)?;

    let query_msg = QueryMsg::ProtocolFee {};
    let protocol_fee: Percentage = from_binary(&query(deps.as_ref(), env.clone(), query_msg.clone())?)?;
    assert_eq!(protocol_fee, Percentage::new(0));

    let execute_msg = ExecuteMsg::ChangeProtocolFee {
        protocol_fee: Percentage::new(1),
    };
    execute(deps.as_mut(), env.clone(), info.clone(), execute_msg)?;

    let protocol_fee: Percentage = from_binary(&query(deps.as_ref(), env, query_msg)?)?;
    assert_eq!(protocol_fee, Percentage::new(1));

    Ok(())
}

#[test]
fn test_change_protocol_fee_not_admin() -> Result<(), ContractError> {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let admin = "admin";
    let user = "user";

    let instantiate_msg = InstantiateMsg {
        protocol_fee: Percentage::new(0),
    };

    let info = mock_info(admin, &[]);
    instantiate(deps.as_mut(), env.clone(), info.clone(), instantiate_msg)?;

    let info = mock_info(user, &[]);
    let execute_msg = ExecuteMsg::ChangeProtocolFee {
        protocol_fee: Percentage::new(1),
    };
    let result = execute(deps.as_mut(), env, info, execute_msg).unwrap_err();
    assert!(matches!(result, ContractError::Unauthorized {}));

    Ok(())
}
