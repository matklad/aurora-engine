use aurora_engine_types::types::{Address, Wei};
use secp256k1::SecretKey;

use crate::test_utils;

// mod access_lists;
// mod contract_call;
// mod eip1559;
// mod erc20;
// mod erc20_connector;
// mod eth_connector;
#[cfg(feature = "meta-call")]
mod meta_parsing;
// mod one_inch;
// mod random;
// mod sanity;
// mod self_destruct_state;
// mod standalone;
mod standard_precompiles;
// mod state_migration;
pub(crate) mod uniswap;

const INITIAL_BALANCE: Wei = Wei::new_u64(1_000_000);
const INITIAL_NONCE: u64 = 0;
const TRANSFER_AMOUNT: Wei = Wei::new_u64(123);
const GAS_PRICE: u64 = 10;

fn initialize_transfer() -> (test_utils::AuroraRunner, test_utils::Signer, Address) {
    // set up Aurora runner and accounts
    let mut runner = test_utils::deploy_evm();
    let mut rng = rand::thread_rng();
    let source_account = SecretKey::random(&mut rng);
    let source_address = test_utils::address_from_secret_key(&source_account);
    runner.create_address(source_address, INITIAL_BALANCE, INITIAL_NONCE.into());
    let dest_address = test_utils::address_from_secret_key(&SecretKey::random(&mut rng));
    let mut signer = test_utils::Signer::new(source_account);
    signer.nonce = INITIAL_NONCE;

    (runner, signer, dest_address)
}

#[test]
fn test_solidity_pure_bench() {
    let (mut runner, mut signer, _) = initialize_transfer();
    runner.wasm_config.limit_config.max_gas_burnt = u64::MAX;

    let constructor = test_utils::solidity::ContractConstructor::force_compile(
        "src/tests/res",
        "target/solidity_build",
        "bench.sol",
        "Bencher",
    );

    let nonce = signer.use_nonce();
    let contract = runner.deploy_contract(
        &signer.secret_key,
        |c| c.deploy_without_constructor(nonce.into()),
        constructor,
    );

    // Number of iterations to do
    let loop_limit = 100_000;
    let (result, profile) = runner
        .submit_with_signer_profiled(&mut signer, |nonce| {
            contract.call_method_with_args(
                "cpu_ram_soak_test",
                &[ethabi::Token::Uint(loop_limit.into())],
                nonce,
            )
        })
        .unwrap();

    // for &cost in Cost::ALL {
    //     let v = profile.host_breakdown[cost] / 10u64.pow(12);
    //     if v > 0 {
    //         eprintln!("{:<30?}: {}", cost, v);
    //     }
    // }

    assert!(
        result.gas_used > 192_000_000,
        "Over 192 million EVM gas is used"
    );
    assert!(
        profile.all_gas() > 29_000 * 1_000_000_000_000,
        "Over 29k NEAR Tgas is used"
    );
}
