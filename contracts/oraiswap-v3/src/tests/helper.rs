use cosmwasm_std::{Addr, Coin};
use cw20::{Cw20Coin, Cw20Contract, MinterResponse};

use decimal::Factories;
use osmosis_test_tube::{Module, OraichainTestApp, Wasm};
use test_tube::cosmrs::proto::cosmwasm::wasm::v1::MsgExecuteContractResponse;
use test_tube::Account;
use test_tube::RunnerExecuteResult;
use test_tube::SigningAccount;

use crate::msg::ExecuteMsg;
use crate::msg::InstantiateMsg;
use crate::percentage::Percentage;
use crate::FeeTier;

static CW20_BYTES: &[u8] = include_bytes!("testdata/cw20-base.wasm");
static CLMM_BYTES: &[u8] = include_bytes!("testdata/oraiswap-v3.wasm");

pub struct TestTubeScenario {
    pub router: OraichainTestApp,
    /// [owner, alice, bob, david]
    pub accounts: Vec<SigningAccount>,
    pub clmm_addr: String,
    pub token_x: Cw20Contract,
    pub token_y: Cw20Contract,
}

impl Default for TestTubeScenario {
    fn default() -> Self {
        Self::new(Percentage::from_scale(6, 3))
    }
}

impl TestTubeScenario {
    pub fn new(protocol_fee: Percentage) -> Self {
        let router = OraichainTestApp::default();

        let init_funds = [Coin::new(5_000_000_000_000u128, "orai")];

        let accounts = router.init_accounts(&init_funds, 4).unwrap();
        // let owner = &accounts[0];

        let (owner, alice, bob, david) = (&accounts[0], &accounts[1], &accounts[2], &accounts[3]);

        let wasm = Wasm::new(&router);
        let clmm_id = wasm
            .store_code(CLMM_BYTES, None, owner)
            .unwrap()
            .data
            .code_id;

        let token_id = wasm
            .store_code(CW20_BYTES, None, owner)
            .unwrap()
            .data
            .code_id;

        let clmm_addr = wasm
            .instantiate(
                clmm_id,
                &InstantiateMsg { protocol_fee },
                Some(&owner.address()),
                Some("oraiswap_v3"),
                &[],
                owner,
            )
            .unwrap()
            .data
            .address;

        let initial_amount = 10u128.pow(10);

        let [token_x_addr, token_y_addr] =
            [("Token X", "tokenx"), ("Token Y", "tokeny")].map(|(name, symbol)| {
                wasm.instantiate(
                    token_id,
                    &cw20_base::msg::InstantiateMsg {
                        name: name.to_string(),
                        symbol: symbol.to_string(),
                        decimals: 6, //see here
                        initial_balances: vec![
                            Cw20Coin {
                                address: alice.address(),
                                amount: initial_amount.into(),
                            },
                            Cw20Coin {
                                address: bob.address(),
                                amount: initial_amount.into(),
                            },
                            Cw20Coin {
                                address: david.address(),
                                amount: initial_amount.into(),
                            },
                        ],
                        mint: Some(MinterResponse {
                            minter: owner.address(),
                            cap: None,
                        }),
                        marketing: None,
                    },
                    Some(&owner.address()),
                    Some("cw20"),
                    &[],
                    owner,
                )
                .unwrap()
                .data
                .address
            });

        let token_x = Cw20Contract(Addr::unchecked(token_x_addr));
        let token_y = Cw20Contract(Addr::unchecked(token_y_addr));

        Self {
            router,
            accounts,
            token_x,
            token_y,
            clmm_addr,
        }
    }

    pub fn add_fee_tier(
        &mut self,
        acc_idx: usize,
        fee_tier: FeeTier,
    ) -> RunnerExecuteResult<MsgExecuteContractResponse> {
        let alice = &self.accounts[acc_idx];

        let wasm = Wasm::new(&self.router);

        wasm.execute(
            self.clmm_addr.as_str(),
            &ExecuteMsg::AddFeeTier { fee_tier },
            &[],
            alice,
        )
    }
}
