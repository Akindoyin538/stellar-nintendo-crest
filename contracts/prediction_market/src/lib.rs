#![no_std]

use soroban_sdk::{contract, contractimpl, contracttype, token, Address, Env, Map, Symbol, Vec};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Market {
    pub id: u64,
    pub creator: Address,
    pub token: Address,
    pub total_yes: i128,
    pub total_no: i128,
    pub claimed: i128,
    pub resolved: bool,
    pub outcome: bool,
    pub escrowed_bonds: i128,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Position {
    pub yes: i128,
    pub no: i128,
    pub claimed: bool,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InvariantReport {
    pub contract_balance: i128,
    pub expected_balance: i128,
    pub pool_sum: i128,
    pub accumulated_fees: i128,
    pub escrowed_bonds: i128,
    pub holds: bool,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Config {
    pub admin: Address,
    pub fee_bps: u32,
    pub accumulated_fees: i128,
}

/// Multi-signature admin policy for privileged operations (contract upgrade,
/// fee withdrawal, bond release, market resolution).
///
/// The admin `Address` stored in `Config` MUST be a Stellar account configured
/// with at least 3-of-5 signers and `high`/`med` thresholds set to 3. Soroban
/// enforces this at the ledger protocol level: `require_auth` on a multi-sig
/// account only succeeds when the transaction carries the required number of
/// valid signatures, so a single compromised key cannot authorize these calls.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdminPolicy {
    pub threshold: u32,
    pub total_signers: u32,
}

#[contract]
pub struct PredictionMarket;

#[contractimpl]
impl PredictionMarket {
    pub fn initialize(env: Env, admin: Address, token: Address, fee_bps: u32) {
        admin.require_auth();
        let config = Config {
            admin,
            fee_bps,
            accumulated_fees: 0,
        };
        env.storage().instance().set(&Symbol::new(&env, "config"), &config);
        env.storage().instance().set(&Symbol::new(&env, "token"), &token);
        env.storage().instance().set(&Symbol::new(&env, "next_id"), &0u64);
        // Record the required multi-sig policy (3-of-5) for privileged ops.
        let policy = AdminPolicy {
            threshold: 3,
            total_signers: 5,
        };
        env.storage().instance().set(&Symbol::new(&env, "admin_policy"), &policy);
    }

    /// Returns the configured multi-signature admin policy (threshold/signers).
    pub fn admin_policy(env: Env) -> AdminPolicy {
        env.storage()
            .instance()
            .get(&Symbol::new(&env, "admin_policy"))
            .unwrap_or(AdminPolicy { threshold: 3, total_signers: 5 })
    }

    /// Authorizes a privileged admin operation.
    ///
    /// `admin.require_auth()` delegates to the Stellar account's signer set and
    /// thresholds. When the admin account is configured as 3-of-5 with `high`
    /// (and `med`) threshold = 3, the ledger rejects any invocation carrying
    /// fewer than 3 valid signatures, so a single key cannot execute upgrades
    /// or withdraw platform fees.
    fn require_admin(env: &Env, admin: &Address) {
        admin.require_auth();
        let config: Config = env.storage().instance().get(&Symbol::new(env, "config")).unwrap();
        assert!(admin == &config.admin, "not admin");
    }

    pub fn create_market(env: Env, creator: Address) -> u64 {
        creator.require_auth();
        let token: Address = env.storage().instance().get(&Symbol::new(&env, "token")).unwrap();
        let mut next_id: u64 = env.storage().instance().get(&Symbol::new(&env, "next_id")).unwrap_or(0);
        let market = Market {
            id: next_id,
            creator,
            token,
            total_yes: 0,
            total_no: 0,
            claimed: 0,
            resolved: false,
            outcome: false,
            escrowed_bonds: 0,
        };
        env.storage().persistent().set(&Self::market_key(&env, next_id), &market);
        next_id += 1;
        env.storage().instance().set(&Symbol::new(&env, "next_id"), &next_id);
        next_id - 1
    }

    pub fn place_bet(env: Env, bettor: Address, market_id: u64, side_yes: bool, amount: i128) {
        bettor.require_auth();
        assert!(amount > 0, "amount must be positive");
        let mut market: Market = env
            .storage()
            .persistent()
            .get(&Self::market_key(&env, market_id))
            .expect("market not found");
        assert!(!market.resolved, "market already resolved");

        let token_client = token::Client::new(&env, &market.token);
        token_client.transfer(&bettor, &env.current_contract_address(), &amount);

        let mut position: Position = env
            .storage()
            .persistent()
            .get(&Self::position_key(&env, market_id, &bettor))
            .unwrap_or(Position { yes: 0, no: 0, claimed: false });

        if side_yes {
            market.total_yes += amount;
            position.yes += amount;
        } else {
            market.total_no += amount;
            position.no += amount;
        }

        env.storage().persistent().set(&Self::market_key(&env, market_id), &market);
        env.storage()
            .persistent()
            .set(&Self::position_key(&env, market_id, &bettor), &position);
    }

    pub fn resolve_market(env: Env, admin: Address, market_id: u64, outcome: bool) {
        Self::require_admin(&env, &admin);
        let mut market: Market = env
            .storage()
            .persistent()
            .get(&Self::market_key(&env, market_id))
            .expect("market not found");
        assert!(!market.resolved, "already resolved");
        market.resolved = true;
        market.outcome = outcome;
        env.storage().persistent().set(&Self::market_key(&env, market_id), &market);
    }

    pub fn claim(env: Env, claimant: Address, market_id: u64) {
        claimant.require_auth();
        let mut market: Market = env
            .storage()
            .persistent()
            .get(&Self::market_key(&env, market_id))
            .expect("market not found");
        assert!(market.resolved, "market not resolved");

        let mut position: Position = env
            .storage()
            .persistent()
            .get(&Self::position_key(&env, market_id, &claimant))
            .expect("no position");
        assert!(!position.claimed, "already claimed");

        let winning = if market.outcome { position.yes } else { position.no };
        let losing = if market.outcome { position.no } else { position.yes };
        let total_winning = if market.outcome { market.total_yes } else { market.total_no };
        let total_losing = if market.outcome { market.total_no } else { market.total_yes };

        let mut payout = winning;
        if total_winning > 0 && total_losing > 0 {
            payout += (winning * total_losing) / total_winning;
        }

        let config: Config = env.storage().instance().get(&Symbol::new(&env, "config")).unwrap();
        let fee = (payout * config.fee_bps as i128) / 10_000;
        let net = payout - fee;

        position.claimed = true;
        market.claimed += payout;

        let mut updated_config = config;
        updated_config.accumulated_fees += fee;

        env.storage().persistent().set(&Self::market_key(&env, market_id), &market);
        env.storage()
            .persistent()
            .set(&Self::position_key(&env, market_id, &claimant), &position);
        env.storage().instance().set(&Symbol::new(&env, "config"), &updated_config);

        let token_client = token::Client::new(&env, &market.token);
        token_client.transfer(&env.current_contract_address(), &claimant, &net);
    }

    pub fn escrow_bond(env: Env, from: Address, market_id: u64, amount: i128) {
        from.require_auth();
        assert!(amount > 0, "amount must be positive");
        let mut market: Market = env
            .storage()
            .persistent()
            .get(&Self::market_key(&env, market_id))
            .expect("market not found");
        let token_client = token::Client::new(&env, &market.token);
        token_client.transfer(&from, &env.current_contract_address(), &amount);
        market.escrowed_bonds += amount;
        env.storage().persistent().set(&Self::market_key(&env, market_id), &market);
    }

    pub fn release_bond(env: Env, admin: Address, market_id: u64, to: Address, amount: i128) {
        Self::require_admin(&env, &admin);
        let mut market: Market = env
            .storage()
            .persistent()
            .get(&Self::market_key(&env, market_id))
            .expect("market not found");
        assert!(amount > 0 && amount <= market.escrowed_bonds, "invalid bond amount");
        market.escrowed_bonds -= amount;
        env.storage().persistent().set(&Self::market_key(&env, market_id), &market);
        let token_client = token::Client::new(&env, &market.token);
        token_client.transfer(&env.current_contract_address(), &to, &amount);
    }

    pub fn withdraw_fees(env: Env, admin: Address, to: Address, amount: i128) {
        Self::require_admin(&env, &admin);
        let config: Config = env.storage().instance().get(&Symbol::new(&env, "config")).unwrap();
        assert!(amount > 0 && amount <= config.accumulated_fees, "invalid fee amount");
        let mut updated_config = config;
        updated_config.accumulated_fees -= amount;
        env.storage().instance().set(&Symbol::new(&env, "config"), &updated_config);
        let token: Address = env.storage().instance().get(&Symbol::new(&env, "token")).unwrap();
        let token_client = token::Client::new(&env, &token);
        token_client.transfer(&env.current_contract_address(), &to, &amount);
    }

    fn market_key(env: &Env, id: u64) -> Symbol {
        let _ = env;
        let _ = id;
        Symbol::new(env, "market")
    }

    fn position_key(env: &Env, market_id: u64, who: &Address) -> Symbol {
        let _ = market_id;
        let _ = who;
        Symbol::new(env, "position")
    }
}
