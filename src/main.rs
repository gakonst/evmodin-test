use ethers::{
    abi::{self, FunctionExt},
    types::*,
    utils::{id, Solc},
};
use evm::backend::{MemoryAccount, MemoryBackend, MemoryVicinity};
use evm::executor::{MemoryStackState, StackExecutor, StackSubstateMetadata};
use evm::Config;
use evm::{ExitReason, ExitRevert, ExitSucceed};
use std::collections::BTreeMap;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    // compile the contracts
    let compiled = Solc::new("./*.sol").build()?;
    let compiled = compiled.get("Greet").expect("could not find contract");

    let config = Config::istanbul();

    let vicinity = MemoryVicinity {
        gas_price: U256::zero(),
        origin: H160::default(),
        block_hashes: Vec::new(),
        block_number: Default::default(),
        block_coinbase: Default::default(),
        block_timestamp: Default::default(),
        block_difficulty: Default::default(),
        block_gas_limit: Default::default(),
        chain_id: U256::one(),
    };
    let mut state = BTreeMap::new();

    // Deploy the contract
    let bytecode = compiled.runtime_bytecode.clone().to_vec();
    let contract_address: Address = "0x1000000000000000000000000000000000000000"
        .parse()
        .unwrap();
    state.insert(
        contract_address,
        MemoryAccount {
            nonce: U256::one(),
            balance: U256::from(10000000),
            storage: BTreeMap::new(),
            code: bytecode,
        },
    );

    // setup memory backend w/ initial state
    let backend = MemoryBackend::new(&vicinity, state);
    let mut executor = {
        // setup gasometer
        let gas_limit = 15_000_000;
        let metadata = StackSubstateMetadata::new(gas_limit, &config);
        // setup state
        let state = MemoryStackState::new(metadata, &backend);
        // setup executor
        StackExecutor::new(state, &config)
    };

    // first make a call to `setUp()`, as done in DappTools
    let data = id("setUp()").to_vec();
    // call the setup function
    let from = Address::zero();
    let to = contract_address;
    let value = 0.into();
    let gas_limit = 10_000_000;
    let (reason, _) = executor.transact_call(from, to, value, data, gas_limit);
    assert!(matches!(reason, ExitReason::Succeed(_)));

    // get all the test functions
    let test_fns = compiled
        .abi
        .functions()
        .into_iter()
        .filter(|func| func.name.starts_with("test"));

    // call all the test functions
    for func in test_fns {
        // the expected result depends on the function name
        let expected = if func.name.contains("testFail") {
            ExitReason::Revert(ExitRevert::Reverted)
        } else {
            ExitReason::Succeed(ExitSucceed::Stopped)
        };

        // set the selector & execute the call
        let data = func.selector().to_vec().into();
        let (result, output) = executor.transact_call(from, to, value, data, gas_limit);

        // print the revert reason if Reverted
        if matches!(result, ExitReason::Revert(_)) {
            let revert_reason =
                abi::decode(&[abi::ParamType::String], &output[4..])?[0].to_string();
            println!("{} failed. Revert reason: \"{}\"", func.name, revert_reason);
        }

        // ensure it worked
        assert_eq!(result, expected);
        println!("{}: {:?}", func.name, result);
    }

    Ok(())
}
