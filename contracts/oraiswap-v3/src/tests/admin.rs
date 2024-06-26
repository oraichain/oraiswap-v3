use crate::msg::{ExecuteMsg, QueryMsg};
use crate::percentage::Percentage;
use crate::tests::helper::macros::*;
use crate::tests::helper::MockApp;
use cosmwasm_std::Addr;
use decimal::Decimal;

#[test]
fn test_change_admin() {
    let mut app = MockApp::new(&[]);
    let dex = create_dex!(app, Percentage::new(0));

    let query_msg = QueryMsg::Admin {};
    let admin: Addr = app.query(dex.clone(), &query_msg).unwrap();
    assert_eq!(admin, Addr::unchecked("alice"));

    let execute_msg = ExecuteMsg::ChangeAdmin {
        new_admin: Addr::unchecked("bob"),
    };

    let result = app.execute(
        Addr::unchecked("alice"),
        Addr::unchecked(dex.clone()),
        &execute_msg,
        &[],
    );
    assert!(result.is_ok());

    let admin: Addr = app.query(dex.clone(), &query_msg).unwrap();
    assert_eq!(admin, Addr::unchecked("bob"));
}

#[test]
fn test_change_admin_not_admin() {
    let mut app = MockApp::new(&[]);
    let dex = create_dex!(app, Percentage::new(0));

    let execute_msg = ExecuteMsg::ChangeAdmin {
        new_admin: Addr::unchecked("bob"),
    };
    let result = app
        .execute(
            Addr::unchecked("bob"),
            Addr::unchecked(dex.clone()),
            &execute_msg,
            &[],
        )
        .unwrap_err();

    assert!(result.contains("error executing WasmMsg"));
}
