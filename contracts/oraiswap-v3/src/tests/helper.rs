use cosmwasm_schema::serde::de::DeserializeOwned;
use cosmwasm_schema::serde::Serialize;
use cosmwasm_std::{
    Addr, AllBalanceResponse, Attribute, BalanceResponse, BankQuery, Coin, Empty, QuerierWrapper,
    QueryRequest, StdResult, Uint128,
};
use cw20::TokenInfoResponse;
use decimal::num_traits::Zero;
use std::collections::HashMap;

use cw_multi_test::{next_block, App, AppResponse, Contract, Executor};

use crate::{
    interface::SwapHop,
    liquidity::Liquidity,
    msg::{self, QuoteResult},
    percentage::Percentage,
    sqrt_price::SqrtPrice,
    token_amount::TokenAmount,
    FeeTier, Pool, PoolKey, Tick,
};

#[macro_export]
macro_rules! create_entry_points_testing {
    ($contract:ident) => {
        cw_multi_test::ContractWrapper::new(
            $contract::contract::execute,
            $contract::contract::instantiate,
            $contract::contract::query,
        )
    };
}

pub trait AttributeUtil {
    fn get_attributes(&self, index: usize) -> Vec<Attribute>;
}

impl AttributeUtil for AppResponse {
    fn get_attributes(&self, index: usize) -> Vec<Attribute> {
        self.events[index].attributes[1..].to_vec()
    }
}

pub struct MockApp {
    app: App,
    token_map: HashMap<String, Addr>, // map token name to address
    pub token_id: u64,
    pub oracle_addr: Addr,
    pub factory_addr: Addr,
    pub router_addr: Addr,
}

#[allow(dead_code)]
impl MockApp {
    pub fn new(init_balances: &[(&str, &[Coin])]) -> Self {
        let app = App::new(|router, _, storage| {
            for (owner, init_funds) in init_balances.iter() {
                router
                    .bank
                    .init_balance(
                        storage,
                        &Addr::unchecked(owner.to_owned()),
                        init_funds.to_vec(),
                    )
                    .unwrap();
            }
        });

        MockApp {
            app,
            token_id: 0,
            oracle_addr: Addr::unchecked(""),
            factory_addr: Addr::unchecked(""),
            router_addr: Addr::unchecked(""),
            token_map: HashMap::new(),
        }
    }

    pub fn set_token_contract(&mut self, code: Box<dyn Contract<Empty>>) {
        self.token_id = self.upload(code);
    }

    pub fn upload(&mut self, code: Box<dyn Contract<Empty>>) -> u64 {
        let code_id = self.app.store_code(code);
        self.app.update_block(next_block);
        code_id
    }

    pub fn instantiate<T: Serialize>(
        &mut self,
        code_id: u64,
        sender: Addr,
        init_msg: &T,
        send_funds: &[Coin],
        label: &str,
    ) -> Result<Addr, String> {
        let contract_addr = self
            .app
            .instantiate_contract(code_id, sender, init_msg, send_funds, label, None)
            .map_err(|err| err.to_string())?;
        self.app.update_block(next_block);
        Ok(contract_addr)
    }

    pub fn execute<T: Serialize + std::fmt::Debug>(
        &mut self,
        sender: Addr,
        contract_addr: Addr,
        msg: &T,
        send_funds: &[Coin],
    ) -> Result<AppResponse, String> {
        let response = self
            .app
            .execute_contract(sender, contract_addr, msg, send_funds)
            .map_err(|err| err.to_string())?;

        self.app.update_block(next_block);

        Ok(response)
    }

    pub fn query<T: DeserializeOwned, U: Serialize>(
        &self,
        contract_addr: Addr,
        msg: &U,
    ) -> StdResult<T> {
        self.app.wrap().query_wasm_smart(contract_addr, msg)
    }

    pub fn query_balance(&self, account_addr: Addr, denom: String) -> StdResult<Uint128> {
        // load price form the oracle
        let balance: BalanceResponse =
            self.app
                .wrap()
                .query(&QueryRequest::Bank(BankQuery::Balance {
                    address: account_addr.to_string(),
                    denom,
                }))?;
        Ok(balance.amount.amount)
    }

    pub fn query_all_balances(&self, account_addr: Addr) -> StdResult<Vec<Coin>> {
        // load price form the oracle
        let all_balances: AllBalanceResponse =
            self.app
                .wrap()
                .query(&QueryRequest::Bank(BankQuery::AllBalances {
                    address: account_addr.to_string(),
                }))?;
        Ok(all_balances.amount)
    }

    pub fn register_token(&mut self, contract_addr: Addr) -> StdResult<String> {
        let res: cw20::TokenInfoResponse =
            self.query(contract_addr.clone(), &cw20::Cw20QueryMsg::TokenInfo {})?;
        self.token_map.insert(res.symbol.clone(), contract_addr);
        Ok(res.symbol)
    }

    pub fn query_token_balance(
        &self,
        contract_addr: &str,
        account_addr: &str,
    ) -> StdResult<Uint128> {
        let res: cw20::BalanceResponse = self.query(
            Addr::unchecked(contract_addr),
            &cw20::Cw20QueryMsg::Balance {
                address: account_addr.to_string(),
            },
        )?;
        Ok(res.balance)
    }

    pub fn query_token_info(&self, contract_addr: Addr) -> StdResult<TokenInfoResponse> {
        self.query(contract_addr, &cw20::Cw20QueryMsg::TokenInfo {})
    }

    pub fn query_token_balances(&self, account_addr: &str) -> StdResult<Vec<Coin>> {
        let mut balances = vec![];
        for (denom, contract_addr) in self.token_map.iter() {
            let res: cw20::BalanceResponse = self.query(
                contract_addr.clone(),
                &cw20::Cw20QueryMsg::Balance {
                    address: account_addr.to_string(),
                },
            )?;
            balances.push(Coin {
                denom: denom.clone(),
                amount: res.balance,
            });
        }
        Ok(balances)
    }

    pub fn as_querier(&self) -> QuerierWrapper {
        self.app.wrap()
    }

    pub fn get_token_addr(&self, token: &str) -> Option<Addr> {
        self.token_map.get(token).cloned()
    }

    pub fn create_token(&mut self, owner: &str, token: &str, initial_amount: u128) -> Addr {
        let addr = self
            .instantiate(
                self.token_id,
                Addr::unchecked(owner),
                &cw20_base::msg::InstantiateMsg {
                    name: token.to_string(),
                    symbol: token.to_string(),
                    decimals: 6,
                    initial_balances: vec![cw20::Cw20Coin {
                        address: owner.to_string(),
                        amount: initial_amount.into(),
                    }],
                    mint: Some(cw20::MinterResponse {
                        minter: owner.to_string(),
                        cap: None,
                    }),
                    marketing: None,
                },
                &[],
                "cw20",
            )
            .unwrap();
        self.token_map.insert(token.to_string(), addr.clone());
        addr
    }

    pub fn set_balances_from(
        &mut self,
        sender: Addr,
        balances: &[(&String, &[(&String, &Uint128)])],
    ) {
        for (denom, balances) in balances.iter() {
            // send for each recipient
            for (recipient, &amount) in balances.iter() {
                self.app
                    .send_tokens(
                        sender.clone(),
                        Addr::unchecked(recipient.as_str()),
                        &[Coin {
                            denom: denom.to_string(),
                            amount,
                        }],
                    )
                    .unwrap();
            }
        }
    }

    pub fn mint_token(
        &mut self,
        sender: &str,
        recipient: &str,
        cw20_addr: &str,
        amount: u128,
    ) -> Result<AppResponse, String> {
        self.execute(
            Addr::unchecked(sender),
            Addr::unchecked(cw20_addr),
            &cw20::Cw20ExecuteMsg::Mint {
                recipient: recipient.to_string(),
                amount: amount.into(),
            },
            &[],
        )
    }

    pub fn set_token_balances_from(
        &mut self,
        sender: &str,
        balances: &[(&str, &[(&str, u128)])],
    ) -> Result<Vec<Addr>, String> {
        let mut contract_addrs = vec![];
        for (token, balances) in balances {
            let contract_addr = match self.token_map.get(*token) {
                None => self.create_token(sender, token, 0),
                Some(addr) => addr.clone(),
            };
            contract_addrs.push(contract_addr.clone());

            // mint for each recipient
            for (recipient, amount) in balances.iter() {
                if !amount.is_zero() {
                    self.mint_token(sender, recipient, contract_addr.as_str(), *amount)?;
                }
            }
        }
        Ok(contract_addrs)
    }

    pub fn set_balances(&mut self, owner: &str, balances: &[(&String, &[(&String, &Uint128)])]) {
        self.set_balances_from(Addr::unchecked(owner), balances)
    }

    // configure the mint whitelist mock querier
    pub fn set_token_balances(
        &mut self,
        owner: &str,
        balances: &[(&str, &[(&str, u128)])],
    ) -> Result<Vec<Addr>, String> {
        self.set_token_balances_from(owner, balances)
    }

    pub fn approve_token(
        &mut self,
        token: &str,
        approver: &str,
        spender: &str,
        amount: u128,
    ) -> Result<AppResponse, String> {
        let token_addr = match self.token_map.get(token) {
            Some(v) => v.to_owned(),
            None => Addr::unchecked(token),
        };

        self.execute(
            Addr::unchecked(approver),
            token_addr,
            &cw20::Cw20ExecuteMsg::IncreaseAllowance {
                spender: spender.to_string(),
                amount: amount.into(),
                expires: None,
            },
            &[],
        )
    }

    /// external method

    pub fn create_dex(&mut self, owner: &str, protocol_fee: Percentage) -> Result<Addr, String> {
        let dex_code_id = self.upload(Box::new(create_entry_points_testing!(crate)));
        self.instantiate(
            dex_code_id,
            Addr::unchecked(owner),
            &msg::InstantiateMsg { protocol_fee },
            &[],
            "oraiswap_v3",
        )
    }

    pub fn add_fee_tier(
        &mut self,
        sender: &str,
        clmm_addr: &str,
        fee_tier: FeeTier,
    ) -> Result<AppResponse, String> {
        self.execute(
            Addr::unchecked(sender),
            Addr::unchecked(clmm_addr),
            &msg::ExecuteMsg::AddFeeTier { fee_tier },
            &[],
        )
    }

    pub fn remove_fee_tier(
        &mut self,
        sender: &str,
        clmm_addr: &str,
        fee_tier: FeeTier,
    ) -> Result<AppResponse, String> {
        self.execute(
            Addr::unchecked(sender),
            Addr::unchecked(clmm_addr),
            &msg::ExecuteMsg::RemoveFeeTier { fee_tier },
            &[],
        )
    }

    pub fn create_pool(
        &mut self,
        sender: &str,
        clmm_addr: &str,
        token_x: &str,
        token_y: &str,
        fee_tier: FeeTier,
        init_sqrt_price: SqrtPrice,
        init_tick: i32,
    ) -> Result<AppResponse, String> {
        self.execute(
            Addr::unchecked(sender),
            Addr::unchecked(clmm_addr),
            &msg::ExecuteMsg::CreatePool {
                token_0: Addr::unchecked(token_x),
                token_1: Addr::unchecked(token_y),
                fee_tier,
                init_sqrt_price,
                init_tick,
            },
            &[],
        )
    }

    pub fn withdraw_protocol_fee(
        &mut self,
        sender: &str,
        clmm_addr: &str,
        pool_key: &PoolKey,
    ) -> Result<AppResponse, String> {
        self.execute(
            Addr::unchecked(sender),
            Addr::unchecked(clmm_addr),
            &msg::ExecuteMsg::WithdrawProtocolFee {
                pool_key: pool_key.clone(),
            },
            &[],
        )
    }

    pub fn change_fee_receiver(
        &mut self,
        sender: &str,
        clmm_addr: &str,
        pool_key: &PoolKey,
        fee_recevier: &str,
    ) -> Result<AppResponse, String> {
        self.execute(
            Addr::unchecked(sender),
            Addr::unchecked(clmm_addr),
            &msg::ExecuteMsg::ChangeFeeReceiver {
                pool_key: pool_key.clone(),
                fee_receiver: Addr::unchecked(fee_recevier),
            },
            &[],
        )
    }

    pub fn create_position(
        &mut self,
        sender: &str,
        clmm_addr: &str,
        pool_key: &PoolKey,
        lower_tick: i32,
        upper_tick: i32,
        liquidity_delta: Liquidity,
        slippage_limit_lower: SqrtPrice,
        slippage_limit_upper: SqrtPrice,
    ) -> Result<AppResponse, String> {
        self.execute(
            Addr::unchecked(sender),
            Addr::unchecked(clmm_addr),
            &msg::ExecuteMsg::CreatePosition {
                pool_key: pool_key.clone(),
                lower_tick,
                upper_tick,
                liquidity_delta,
                slippage_limit_lower,
                slippage_limit_upper,
            },
            &[],
        )
    }

    pub fn swap_route(
        &mut self,
        sender: &str,
        clmm_addr: &str,
        amount_in: TokenAmount,
        expected_amount_out: TokenAmount,
        slippage: Percentage,
        swaps: Vec<SwapHop>,
    ) -> Result<AppResponse, String> {
        self.execute(
            Addr::unchecked(sender),
            Addr::unchecked(clmm_addr),
            &msg::ExecuteMsg::SwapRoute {
                amount_in,
                expected_amount_out,
                slippage,
                swaps,
            },
            &[],
        )
    }

    pub fn swap(
        &mut self,
        sender: &str,
        clmm_addr: &str,
        pool_key: &PoolKey,
        x_to_y: bool,
        amount: TokenAmount,
        by_amount_in: bool,
        sqrt_price_limit: SqrtPrice,
    ) -> Result<AppResponse, String> {
        self.execute(
            Addr::unchecked(sender),
            Addr::unchecked(clmm_addr),
            &msg::ExecuteMsg::Swap {
                pool_key: pool_key.clone(),
                x_to_y,
                amount,
                by_amount_in,
                sqrt_price_limit,
            },
            &[],
        )
    }

    pub fn claim_fee(
        &mut self,
        sender: &str,
        clmm_addr: &str,
        index: u32,
    ) -> Result<AppResponse, String> {
        self.execute(
            Addr::unchecked(sender),
            Addr::unchecked(clmm_addr),
            &msg::ExecuteMsg::ClaimFee { index },
            &[],
        )
    }

    pub fn quote_route(
        &mut self,
        sender: &str,
        clmm_addr: &str,
        amount_in: TokenAmount,
        swaps: Vec<SwapHop>,
    ) -> Result<AppResponse, String> {
        self.execute(
            Addr::unchecked(sender),
            Addr::unchecked(clmm_addr),
            &msg::ExecuteMsg::QuoteRoute { amount_in, swaps },
            &[],
        )
    }

    pub fn quote(
        &mut self,
        clmm_addr: &str,
        pool_key: &PoolKey,
        x_to_y: bool,
        amount: TokenAmount,
        by_amount_in: bool,
        sqrt_price_limit: SqrtPrice,
    ) -> StdResult<QuoteResult> {
        self.query(
            Addr::unchecked(clmm_addr),
            &msg::QueryMsg::Quote {
                pool_key: pool_key.clone(),
                x_to_y,
                amount,
                by_amount_in,
                sqrt_price_limit,
            },
        )
    }

    pub fn get_pool(
        &self,
        clmm_addr: &str,
        token_x: &str,
        token_y: &str,
        fee_tier: FeeTier,
    ) -> StdResult<Pool> {
        self.query(
            Addr::unchecked(clmm_addr),
            &msg::QueryMsg::Pool {
                token_0: Addr::unchecked(token_x),
                token_1: Addr::unchecked(token_y),
                fee_tier,
            },
        )
    }

    pub fn fee_tier_exist(&self, clmm_addr: &str, fee_tier: FeeTier) -> StdResult<bool> {
        self.query(
            Addr::unchecked(clmm_addr),
            &msg::QueryMsg::FeeTierExist { fee_tier },
        )
    }

    pub fn get_tick(&self, clmm_addr: &str, pool_key: &PoolKey, index: i32) -> StdResult<Tick> {
        self.query(
            Addr::unchecked(clmm_addr),
            &msg::QueryMsg::Tick {
                key: pool_key.clone(),
                index,
            },
        )
    }

    pub fn is_tick_initialized(
        &self,
        clmm_addr: &str,
        pool_key: &PoolKey,
        index: i32,
    ) -> StdResult<bool> {
        self.query(
            Addr::unchecked(clmm_addr),
            &msg::QueryMsg::IsTickInitialized {
                key: pool_key.clone(),
                index,
            },
        )
    }

    pub fn assert_fail(&self, res: Result<AppResponse, String>) {
        // new version of cosmwasm does not return detail error
        match res.err() {
            Some(msg) => assert!(msg.contains("error executing WasmMsg")),
            None => panic!("Must return generic error"),
        }
    }
}

pub mod macros {
    macro_rules! extract_amount {
        ($res:ident, $key: tt) => {{
            $res.events
                .into_iter()
                .filter(|e| e.ty == "wasm")
                .flat_map(|e| e.attributes)
                .find(|a| a.key == $key)
                .unwrap()
                .value
                .parse::<u128>()
                .map(TokenAmount)
        }};
    }
    pub(crate) use extract_amount;

    macro_rules! create_tokens {
        ($app:ident, $token_x_supply:expr, $token_y_supply:expr, $owner: tt) => {{
            let token_x = $app.create_token($owner, "tokenx", $token_x_supply);
            let token_y = $app.create_token($owner, "tokeny", $token_y_supply);
            (token_x, token_y)
        }};
        ($app:ident, $token_x_supply:expr, $token_y_supply:expr) => {{
            create_tokens!($app, $token_x_supply, $token_y_supply, "alice")
        }};
    }

    pub(crate) use create_tokens;

    macro_rules! create_3_tokens {
        ($app:ident, $token_x_supply:expr, $token_y_supply:expr,$token_z_supply:expr, $owner: tt) => {{
            let token_x = $app.create_token($owner, "tokenx", $token_x_supply);
            let token_y = $app.create_token($owner, "tokeny", $token_y_supply);
            let token_z = $app.create_token($owner, "tokenz", $token_y_supply);
            (token_x, token_y, token_z)
        }};
        ($app:ident, $token_x_supply:expr, $token_y_supply:expr,$token_z_supply:expr) => {{
            create_3_tokens!(
                $app,
                $token_x_supply,
                $token_y_supply,
                $token_z_supply,
                "alice"
            )
        }};
    }
    pub(crate) use create_3_tokens;

    macro_rules! create_pool {
        ($app:ident, $dex_address:expr, $token_0:expr, $token_1:expr, $fee_tier:expr, $init_sqrt_price:expr, $init_tick:expr, $caller:tt) => {{
            $app.create_pool(
                $caller,
                $dex_address.as_str(),
                $token_0.as_str(),
                $token_1.as_str(),
                $fee_tier,
                $init_sqrt_price,
                $init_tick,
            )
        }};
    }
    pub(crate) use create_pool;

    macro_rules! add_fee_tier {
        ($app:ident, $dex_address:expr, $fee_tier:expr, $caller:tt) => {{
            $app.add_fee_tier($caller, $dex_address.as_str(), $fee_tier)
        }};
    }
    pub(crate) use add_fee_tier;

    macro_rules! remove_fee_tier {
        ($app:ident, $dex_address:expr, $fee_tier:expr, $caller:tt) => {{
            $app.remove_fee_tier($caller, $dex_address.as_str(), $fee_tier)
        }};
    }
    pub(crate) use remove_fee_tier;

    macro_rules! approve {
        ($app:ident, $token_address:expr, $spender:expr, $value:expr, $caller:tt) => {{
            $app.approve_token($token_address.as_str(), $caller, $spender.as_str(), $value)
        }};
    }
    pub(crate) use approve;

    macro_rules! fee_tier_exist {
        ($app:ident, $dex_address:expr, $fee_tier:expr) => {{
            $app.fee_tier_exist($dex_address.as_str(), $fee_tier)
                .unwrap()
        }};
    }
    pub(crate) use fee_tier_exist;

    macro_rules! create_position {
        ($app:ident, $dex_address:expr, $pool_key:expr, $lower_tick:expr, $upper_tick:expr, $liquidity_delta:expr, $slippage_limit_lower:expr, $slippage_limit_upper:expr, $caller:tt) => {{
            $app.create_position(
                $caller,
                $dex_address.as_str(),
                &$pool_key,
                $lower_tick,
                $upper_tick,
                $liquidity_delta,
                $slippage_limit_lower,
                $slippage_limit_upper,
            )
        }};
    }
    pub(crate) use create_position;

    macro_rules! get_pool {
        ($app:ident, $dex_address:expr, $token_0:expr, $token_1:expr, $fee_tier:expr) => {{
            $app.get_pool(
                $dex_address.as_str(),
                $token_0.as_str(),
                $token_1.as_str(),
                $fee_tier,
            )
        }};
    }
    pub(crate) use get_pool;

    macro_rules! get_tick {
        ($app:ident, $dex_address:expr, $key:expr, $index:expr) => {{
            $app.get_tick($dex_address.as_str(), &$key, $index)
        }};
    }
    pub(crate) use get_tick;

    macro_rules! is_tick_initialized {
        ($app:ident, $dex_address:expr, $key:expr, $index:expr) => {{
            $app.is_tick_initialized($dex_address.as_str(), &$key, $index)
                .unwrap()
        }};
    }
    pub(crate) use is_tick_initialized;

    macro_rules! mint {
        ($app:ident, $token_address:expr, $to:tt, $value:expr, $caller:tt) => {{
            $app.mint_token($caller, $to, $token_address.as_str(), $value)
        }};
    }
    pub(crate) use mint;

    macro_rules! quote {
        ($app:ident,  $dex_address:expr, $pool_key:expr, $x_to_y:expr, $amount:expr, $by_amount_in:expr, $sqrt_price_limit:expr) => {{
            $app.quote(
                $dex_address.as_str(),
                &$pool_key,
                $x_to_y,
                $amount,
                $by_amount_in,
                $sqrt_price_limit,
            )
        }};
    }
    pub(crate) use quote;

    macro_rules! balance_of {
        // any type that can converted to string
        ($app:ident, $token_address:expr, $owner:expr) => {{
            $app.query_token_balance($token_address.as_str(), &$owner.to_string())
                .unwrap()
                .u128()
        }};
        ($app:ident, $token_address:expr, $owner:tt) => {{
            $app.query_token_balance($token_address.as_str(), $owner)
                .unwrap()
                .u128()
        }};
    }
    pub(crate) use balance_of;

    macro_rules! swap {
        ($app:ident, $dex_address:expr, $pool_key:expr, $x_to_y:expr, $amount:expr, $by_amount_in:expr, $sqrt_price_limit:expr, $caller:tt) => {{
            $app.swap(
                $caller,
                $dex_address.as_str(),
                &$pool_key,
                $x_to_y,
                $amount,
                $by_amount_in,
                $sqrt_price_limit,
            )
        }};
    }
    pub(crate) use swap;

    macro_rules! quote_route {
        ($app:ident, $dex_address:expr, $amount_in:expr, $swaps:expr, $caller: tt) => {{
            let res = $app
                .quote_route($caller, $dex_address.as_str(), $amount_in, $swaps)
                .unwrap();
            extract_amount!(res, "amount_out")
        }};
        ($app:ident, $dex_address:expr, $amount_in:expr, $swaps:expr) => {{
            quote_route!($app, $dex_address, $amount_in, $swaps, "alice")
        }};
    }
    pub(crate) use quote_route;

    macro_rules! swap_route {
        ($app:ident, $dex_address:expr, $amount_in:expr, $expected_amount_out:expr, $slippage:expr, $swaps:expr, $caller:tt) => {{
            $app.swap_route(
                $caller,
                $dex_address.as_str(),
                $amount_in,
                $expected_amount_out,
                $slippage,
                $swaps,
            )
        }};
    }
    pub(crate) use swap_route;

    macro_rules! claim_fee {
        ($app:ident, $dex_address:expr, $index:expr, $caller:tt) => {{
            $app.claim_fee($caller, $dex_address.as_str(), $index)
        }};
    }
    pub(crate) use claim_fee;

    macro_rules! init_slippage_pool_with_liquidity {
        ($app:ident, $dex_address:ident, $token_x_address:ident, $token_y_address:ident) => {{
            let fee_tier = FeeTier {
                fee: Percentage::from_scale(6, 3),
                tick_spacing: 10,
            };
            add_fee_tier!($app, $dex_address, fee_tier, "alice").unwrap();

            let init_tick = 0;
            let init_sqrt_price = calculate_sqrt_price(init_tick).unwrap();
            create_pool!(
                $app,
                $dex_address,
                $token_x_address,
                $token_y_address,
                fee_tier,
                init_sqrt_price,
                init_tick,
                "alice"
            )
            .unwrap();

            let mint_amount = 10u128.pow(10);
            approve!($app, $token_x_address, $dex_address, mint_amount, "alice").unwrap();
            approve!($app, $token_y_address, $dex_address, mint_amount, "alice").unwrap();

            let pool_key =
                PoolKey::new($token_x_address.clone(), $token_y_address.clone(), fee_tier).unwrap();
            let lower_tick = -1000;
            let upper_tick = 1000;
            let liquidity = Liquidity::from_integer(10_000_000_000u128);

            let pool_before = get_pool!(
                $app,
                $dex_address,
                $token_x_address,
                $token_y_address,
                fee_tier
            )
            .unwrap();
            let slippage_limit_lower = pool_before.sqrt_price;
            let slippage_limit_upper = pool_before.sqrt_price;
            create_position!(
                $app,
                $dex_address,
                pool_key,
                lower_tick,
                upper_tick,
                liquidity,
                slippage_limit_lower,
                slippage_limit_upper,
                "alice"
            )
            .unwrap();

            let pool_after = get_pool!(
                $app,
                $dex_address,
                $token_x_address,
                $token_y_address,
                fee_tier
            )
            .unwrap();

            assert_eq!(pool_after.liquidity, liquidity);

            pool_key
        }};
    }
    pub(crate) use init_slippage_pool_with_liquidity;

    macro_rules! init_basic_pool {
        ($app:ident, $dex_address:ident, $token_x_address:ident, $token_y_address:ident) => {{
            let fee_tier = FeeTier {
                fee: Percentage::from_scale(6, 3),
                tick_spacing: 10,
            };

            add_fee_tier!($app, $dex_address, fee_tier, "alice").unwrap();

            let init_tick = 0;
            let init_sqrt_price = crate::sqrt_price::calculate_sqrt_price(init_tick).unwrap();
            create_pool!(
                $app,
                $dex_address,
                $token_x_address,
                $token_y_address,
                fee_tier,
                init_sqrt_price,
                init_tick,
                "alice"
            )
            .unwrap();
        }};
    }
    pub(crate) use init_basic_pool;

    macro_rules! init_basic_position {
        ($app:ident, $dex_address:ident, $token_x_address:ident, $token_y_address:ident) => {{
            let fee_tier = FeeTier {
                fee: Percentage::from_scale(6, 3),
                tick_spacing: 10,
            };

            let mint_amount = 10u128.pow(10);
            approve!($app, $token_x_address, $dex_address, mint_amount, "alice").unwrap();
            approve!($app, $token_y_address, $dex_address, mint_amount, "alice").unwrap();

            let pool_key =
                PoolKey::new($token_x_address.clone(), $token_y_address.clone(), fee_tier).unwrap();
            let lower_tick = -20;
            let upper_tick = 10;
            let liquidity = crate::math::types::liquidity::Liquidity::from_integer(1000000);

            let pool_before = get_pool!(
                $app,
                $dex_address,
                $token_x_address,
                $token_y_address,
                fee_tier
            )
            .unwrap();
            let slippage_limit_lower = pool_before.sqrt_price;
            let slippage_limit_upper = pool_before.sqrt_price;
            create_position!(
                $app,
                $dex_address,
                pool_key,
                lower_tick,
                upper_tick,
                liquidity,
                slippage_limit_lower,
                slippage_limit_upper,
                "alice"
            )
            .unwrap();

            let pool_after = get_pool!(
                $app,
                $dex_address,
                $token_x_address,
                $token_y_address,
                fee_tier
            )
            .unwrap();

            assert_eq!(pool_after.liquidity, liquidity);
        }};
    }
    pub(crate) use init_basic_position;

    macro_rules! init_cross_position {
        ($app:ident, $dex_address:ident, $token_x_address:ident, $token_y_address:ident) => {{
            let fee_tier = FeeTier {
                fee: Percentage::from_scale(6, 3),
                tick_spacing: 10,
            };

            let mint_amount = 10u128.pow(10);
            approve!($app, $token_x_address, $dex_address, mint_amount, "alice").unwrap();
            approve!($app, $token_y_address, $dex_address, mint_amount, "alice").unwrap();

            let pool_key =
                PoolKey::new($token_x_address.clone(), $token_y_address.clone(), fee_tier).unwrap();
            let lower_tick = -40;
            let upper_tick = -10;
            let liquidity = Liquidity::from_integer(1000000);

            let pool_before = get_pool!(
                $app,
                $dex_address,
                $token_x_address,
                $token_y_address,
                fee_tier
            )
            .unwrap();
            let slippage_limit_lower = pool_before.sqrt_price;
            let slippage_limit_upper = pool_before.sqrt_price;
            create_position!(
                $app,
                $dex_address,
                pool_key,
                lower_tick,
                upper_tick,
                liquidity,
                slippage_limit_lower,
                slippage_limit_upper,
                "alice"
            )
            .unwrap();

            let pool_after = get_pool!(
                $app,
                $dex_address,
                $token_x_address,
                $token_y_address,
                fee_tier
            )
            .unwrap();

            assert_eq!(pool_after.liquidity, liquidity);
        }};
    }
    pub(crate) use init_cross_position;

    macro_rules! swap_exact_limit {
        ($app:ident, $dex_address:ident, $pool_key:expr, $x_to_y:expr, $amount:expr, $by_amount_in:expr, $caller:tt) => {{
            let sqrt_price_limit = if $x_to_y {
                SqrtPrice::new(crate::MIN_SQRT_PRICE)
            } else {
                SqrtPrice::new(crate::MAX_SQRT_PRICE)
            };

            let quote_result = quote!(
                $app,
                $dex_address,
                $pool_key,
                $x_to_y,
                $amount,
                $by_amount_in,
                sqrt_price_limit
            )
            .unwrap();
            swap!(
                $app,
                $dex_address,
                $pool_key,
                $x_to_y,
                $amount,
                $by_amount_in,
                quote_result.target_sqrt_price,
                $caller
            )
            .unwrap();
        }};
    }
    pub(crate) use swap_exact_limit;

    macro_rules! init_dex_and_tokens {
        ($app:ident) => {{
            let mint_amount = 10u128.pow(10);
            let (token_x, token_y) = create_tokens!($app, mint_amount, mint_amount);

            let protocol_fee = Percentage::from_scale(1, 2);
            let dex = $app.create_dex("alice", protocol_fee).unwrap();
            (dex, token_x, token_y)
        }};
    }
    pub(crate) use init_dex_and_tokens;

    macro_rules! init_basic_swap {
        ($app:ident, $dex_address:ident, $token_x_address:ident, $token_y_address:ident) => {{
            let fee = Percentage::from_scale(6, 3);
            let tick_spacing = 10;
            let fee_tier = FeeTier { fee, tick_spacing };
            let pool_key =
                PoolKey::new($token_x_address.clone(), $token_y_address.clone(), fee_tier).unwrap();
            let lower_tick = -20;

            let amount = 1000;

            mint!($app, $token_x_address, "bob", amount, "alice").unwrap();
            let amount_x = balance_of!($app, $token_x_address, "bob");
            assert_eq!(amount_x, amount);
            approve!($app, $token_x_address, $dex_address, amount, "bob").unwrap();

            let amount_x = balance_of!($app, $token_x_address, $dex_address);
            let amount_y = balance_of!($app, $token_y_address, $dex_address);
            assert_eq!(amount_x, 500);
            assert_eq!(amount_y, 1000);

            let pool_before = get_pool!(
                $app,
                $dex_address,
                $token_x_address,
                $token_y_address,
                pool_key.fee_tier
            )
            .unwrap();

            let swap_amount = TokenAmount::new(amount);
            let slippage = crate::sqrt_price::SqrtPrice::new(crate::MIN_SQRT_PRICE);
            swap!(
                $app,
                $dex_address,
                pool_key,
                true,
                swap_amount,
                true,
                slippage,
                "bob"
            )
            .unwrap();

            let pool_after = get_pool!(
                $app,
                $dex_address,
                $token_x_address,
                $token_y_address,
                fee_tier
            )
            .unwrap();
            assert_eq!(pool_after.liquidity, pool_before.liquidity);
            assert_eq!(pool_after.current_tick_index, lower_tick);
            assert_ne!(pool_after.sqrt_price, pool_before.sqrt_price);

            let amount_x = balance_of!($app, $token_x_address, "bob");
            let amount_y = balance_of!($app, $token_y_address, "bob");
            assert_eq!(amount_x, 0);
            assert_eq!(amount_y, 993);

            let amount_x = balance_of!($app, $token_x_address, $dex_address);
            let amount_y = balance_of!($app, $token_y_address, $dex_address);
            assert_eq!(amount_x, 1500);
            assert_eq!(amount_y, 7);

            assert_eq!(
                pool_after.fee_growth_global_x,
                crate::fee_growth::FeeGrowth::new(50000000000000000000000)
            );
            assert_eq!(
                pool_after.fee_growth_global_y,
                crate::fee_growth::FeeGrowth::new(0)
            );

            assert_eq!(pool_after.fee_protocol_token_x, TokenAmount::new(1));
            assert_eq!(pool_after.fee_protocol_token_y, TokenAmount::new(0));
        }};
    }
    pub(crate) use init_basic_swap;

    macro_rules! withdraw_protocol_fee {
        ($app:ident, $dex_address:expr, $pool_key:expr, $caller:tt) => {{
            $app.withdraw_protocol_fee($caller, $dex_address.as_str(), &$pool_key)
        }};
    }
    pub(crate) use withdraw_protocol_fee;

    macro_rules! change_fee_receiver {
        ($app:ident,  $dex_address:expr, $pool_key:expr, $fee_receiver:tt, $caller:tt) => {{
            $app.change_fee_receiver($caller, $dex_address.as_str(), &$pool_key, $fee_receiver)
        }};
    }
    pub(crate) use change_fee_receiver;

    macro_rules! init_cross_swap {
        ($app:ident, $dex_address:ident, $token_x_address:expr, $token_y_address:expr) => {{
            let fee = Percentage::from_scale(6, 3);
            let tick_spacing = 10;
            let fee_tier = FeeTier { fee, tick_spacing };
            let pool_key = PoolKey::new($token_x_address, $token_y_address, fee_tier).unwrap();
            let lower_tick = -20;

            let amount = 1000;
            let bob = "bob";
            mint!($app, $token_x_address, "bob", amount, "alice").unwrap();
            let amount_x = balance_of!($app, $token_x_address, "bob");
            assert_eq!(amount_x, amount);
            approve!($app, $token_x_address, $dex_address, amount, bob).unwrap();

            let amount_x = balance_of!($app, $token_x_address, $dex_address);
            let amount_y = balance_of!($app, $token_y_address, $dex_address);
            assert_eq!(amount_x, 500);
            assert_eq!(amount_y, 2499);

            let pool_before = get_pool!(
                $app,
                $dex_address,
                $token_x_address,
                $token_y_address,
                fee_tier
            )
            .unwrap();

            let swap_amount = TokenAmount::new(amount);
            let slippage = SqrtPrice::new(MIN_SQRT_PRICE);
            swap!(
                $app,
                $dex_address,
                pool_key,
                true,
                swap_amount,
                true,
                slippage,
                bob
            )
            .unwrap();

            let pool_after = get_pool!(
                $app,
                $dex_address,
                $token_x_address,
                $token_y_address,
                fee_tier
            )
            .unwrap();
            let position_liquidity = Liquidity::from_integer(1000000);
            assert_eq!(
                pool_after.liquidity - position_liquidity,
                pool_before.liquidity
            );
            assert_eq!(pool_after.current_tick_index, lower_tick);
            assert_ne!(pool_after.sqrt_price, pool_before.sqrt_price);

            let amount_x = balance_of!($app, $token_x_address, "bob");
            let amount_y = balance_of!($app, $token_y_address, "bob");
            assert_eq!(amount_x, 0);
            assert_eq!(amount_y, 990);

            let amount_x = balance_of!($app, $token_x_address, $dex_address);
            let amount_y = balance_of!($app, $token_y_address, $dex_address);
            assert_eq!(amount_x, 1500);
            assert_eq!(amount_y, 1509);

            assert_eq!(
                pool_after.fee_growth_global_x,
                FeeGrowth::new(40000000000000000000000)
            );
            assert_eq!(pool_after.fee_growth_global_y, FeeGrowth::new(0));

            assert_eq!(pool_after.fee_protocol_token_x, TokenAmount::new(2));
            assert_eq!(pool_after.fee_protocol_token_y, TokenAmount::new(0));
        }};
    }
    pub(crate) use init_cross_swap;

    macro_rules! get_liquidity_ticks_amount {
        ($app:ident, $dex_address:expr, $pool_key:expr, $lower_tick:expr, $upper_tick:expr) => {{
            $app.query(
                Addr::unchecked($dex_address.as_str()),
                &msg::QueryMsg::LiquidityTicksAmount {
                    pool_key: $pool_key.clone(),
                    lower_tick: $lower_tick,
                    upper_tick: $upper_tick,
                },
            )
        }};
    }
    pub(crate) use get_liquidity_ticks_amount;

    macro_rules! get_tickmap {
        ($app:ident, $dex_address:expr, $pool_key:expr, $lower_tick_index:expr, $upper_tick_index:expr , $x_to_y:expr, $caller:expr) => {{
            $app.query(
                Addr::unchecked($dex_address.as_str()),
                &msg::QueryMsg::TickMap {
                    pool_key: $pool_key.clone(),
                    lower_tick_index: $lower_tick_index,
                    upper_tick_index: $upper_tick_index,
                    x_to_y: $x_to_y,
                },
            )
        }};
    }
    pub(crate) use get_tickmap;

    macro_rules! get_liquidity_ticks {
        ($app:ident, $dex_address:expr, $pool_key:expr, $offset:expr) => {{
            $app.query(
                Addr::unchecked($dex_address.as_str()),
                &msg::QueryMsg::LiquidityTicks {
                    pool_key: $pool_key.clone(),
                    tick_indexes: $offset,
                },
            )
        }};
    }
    pub(crate) use get_liquidity_ticks;

    macro_rules! liquidity_tick_equals {
        ($a:expr, $b:expr) => {{
            assert_eq!($a.index, $b.index);
            assert_eq!($a.liquidity_change, $b.liquidity_change);
            assert_eq!($a.sign, $b.sign);
        }};
    }
    pub(crate) use liquidity_tick_equals;
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::{testing::MOCK_CONTRACT_ADDR, Addr, Coin, Uint128};

    use oraiswap::asset::AssetInfo;

    use super::MockApp;

    #[test]
    fn token_balance_querier() {
        let mut app = MockApp::new(&[]);

        app.set_token_contract(Box::new(crate::create_entry_points_testing!(cw20_base)));

        app.set_token_balances(
            "owner",
            &[(&"AIRI".to_string(), &[(MOCK_CONTRACT_ADDR, 123u128)])],
        )
        .unwrap();

        assert_eq!(
            Uint128::from(123u128),
            app.query_token_balance(
                app.get_token_addr("AIRI").unwrap().as_str(),
                MOCK_CONTRACT_ADDR,
            )
            .unwrap()
        );
    }

    #[test]
    fn balance_querier() {
        let app = MockApp::new(&[(
            &MOCK_CONTRACT_ADDR.to_string(),
            &[Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(200u128),
            }],
        )]);

        assert_eq!(
            app.query_balance(Addr::unchecked(MOCK_CONTRACT_ADDR), "uusd".to_string())
                .unwrap(),
            Uint128::from(200u128)
        );
    }

    #[test]
    fn all_balances_querier() {
        let app = MockApp::new(&[(
            &MOCK_CONTRACT_ADDR.to_string(),
            &[
                Coin {
                    denom: "uusd".to_string(),
                    amount: Uint128::from(200u128),
                },
                Coin {
                    denom: "ukrw".to_string(),
                    amount: Uint128::from(300u128),
                },
            ],
        )]);

        let mut balance1 = app
            .query_all_balances(Addr::unchecked(MOCK_CONTRACT_ADDR))
            .unwrap();
        balance1.sort_by(|a, b| a.denom.cmp(&b.denom));
        let mut balance2 = vec![
            Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(200u128),
            },
            Coin {
                denom: "ukrw".to_string(),
                amount: Uint128::from(300u128),
            },
        ];
        balance2.sort_by(|a, b| a.denom.cmp(&b.denom));
        assert_eq!(balance1, balance2);
    }

    #[test]
    fn supply_querier() {
        let mut app = MockApp::new(&[]);
        app.set_token_contract(Box::new(crate::create_entry_points_testing!(cw20_base)));
        app.set_token_balances(
            "owner",
            &[(
                &"LPA".to_string(),
                &[
                    (MOCK_CONTRACT_ADDR, 123u128),
                    (&"addr00000".to_string(), 123u128),
                    (&"addr00001".to_string(), 123u128),
                    (&"addr00002".to_string(), 123u128),
                ],
            )],
        )
        .unwrap();

        assert_eq!(
            app.query_token_info(app.get_token_addr("LPA").unwrap())
                .unwrap()
                .total_supply,
            Uint128::from(492u128)
        )
    }

    #[test]
    fn test_asset_info() {
        let mut app = MockApp::new(&[(
            &MOCK_CONTRACT_ADDR.to_string(),
            &[Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(123u128),
            }],
        )]);
        app.set_token_contract(Box::new(crate::create_entry_points_testing!(cw20_base)));

        app.set_token_balances(
            "owner",
            &[(
                &"ASSET".to_string(),
                &[
                    (MOCK_CONTRACT_ADDR, 123u128),
                    (&"addr00000".to_string(), 123u128),
                    (&"addr00001".to_string(), 123u128),
                    (&"addr00002".to_string(), 123u128),
                ],
            )],
        )
        .unwrap();

        let token_info = AssetInfo::Token {
            contract_addr: app.get_token_addr("ASSET").unwrap(),
        };
        let native_token_info = AssetInfo::NativeToken {
            denom: "uusd".to_string(),
        };

        assert!(!token_info.eq(&native_token_info));
        assert!(native_token_info.is_native_token());
        assert!(!token_info.is_native_token());

        assert_eq!(
            token_info
                .query_pool(&app.as_querier(), Addr::unchecked(MOCK_CONTRACT_ADDR))
                .unwrap(),
            Uint128::from(123u128)
        );
        assert_eq!(
            native_token_info
                .query_pool(&app.as_querier(), Addr::unchecked(MOCK_CONTRACT_ADDR))
                .unwrap(),
            Uint128::from(123u128)
        );
    }
}
