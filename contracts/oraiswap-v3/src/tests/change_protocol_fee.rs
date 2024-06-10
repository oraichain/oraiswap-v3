use crate::msg::{ExecuteMsg, QueryMsg};
use crate::percentage::Percentage;
use crate::tests::helper::MockApp;
use cosmwasm_std::Addr;
use decimal::Decimal;

#[test]
fn test_change_protocol_fee() {
    let mut mock_app = MockApp::new(&[]);

    let protocol_fee = Percentage::new(0);
    let dex_addr = mock_app.create_dex("alice", protocol_fee).unwrap();

    let query_msg = QueryMsg::ProtocolFee {};
    let protocol_fee: Percentage = mock_app.query(dex_addr.clone(), &query_msg).unwrap();
    assert_eq!(protocol_fee, Percentage::new(0));

    let execute_msg = ExecuteMsg::ChangeProtocolFee {
        protocol_fee: Percentage::new(1),
    };
    let result = mock_app.execute(
        Addr::unchecked("alice"),
        Addr::unchecked(dex_addr.clone()),
        &execute_msg,
        &[],
    );
    assert!(result.is_ok());

    let protocol_fee: Percentage = mock_app.query(dex_addr.clone(), &query_msg).unwrap();
    assert_eq!(protocol_fee, Percentage::new(1));
}

#[test]
fn test_change_protocol_fee_not_admin() {
    let mut mock_app = MockApp::new(&[]);
    let admin = "admin";
    let user = "user";

    let protocol_fee = Percentage::new(0);
    let dex_addr = mock_app.create_dex(admin, protocol_fee).unwrap();

    let execute_msg = ExecuteMsg::ChangeProtocolFee {
        protocol_fee: Percentage::new(1),
    };
    let result = mock_app
        .execute(
            Addr::unchecked(user),
            Addr::unchecked(dex_addr.clone()),
            &execute_msg,
            &[],
        )
        .unwrap_err();

    assert!(result.contains("error executing WasmMsg"));
}
