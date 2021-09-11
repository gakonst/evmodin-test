use ethers::{
    abi::{self, FunctionExt},
    types::*,
    utils::{id, Solc},
};
use evmodin::{
    tracing::{NoopTracer, StdoutTracer, Tracer},
    util::mocked_host::MockedHost,
    AnalyzedCode, CallKind, Message, Revision, StatusCode,
};

#[tokio::main]
async fn main() -> eyre::Result<()> {
    if std::env::var("TRACE").is_ok() {
        run(StdoutTracer::default())
    } else {
        run(NoopTracer)
    }
}

fn run<T: Tracer>(mut tracer: T) -> eyre::Result<()> {
    // compile the contracts
    let compiled = Solc::new("./*.sol").build()?;
    let compiled = compiled.get("Greet").expect("could not find contract");

    // get the contract bytecode (no constructor args)
    let bytecode = compiled.runtime_bytecode.clone().to_vec();

    // setup the contract
    let contract = AnalyzedCode::analyze(bytecode);

    // Note: This host does not support x-contract calls. How are we going to handle it?!
    let mut host = MockedHost::default();
    // first make a call to `setUp()`, as done in DappTools
    let setup_id = id("setUp()").to_vec();
    let gas = 10_000_000;
    let msg = Message {
        kind: CallKind::Call,
        is_static: false,
        depth: 0,
        gas,
        destination: Address::zero(),
        sender: Address::zero(),
        input_data: setup_id.into(),
        value: U256::zero(),
    };
    // call the setup function
    let output = contract.execute(
        &mut host,
        &mut tracer,
        None,
        msg.clone(),
        Revision::latest(),
    );
    assert_eq!(output.status_code, StatusCode::Success);

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
            StatusCode::Revert
        } else {
            StatusCode::Success
        };

        // set the selector
        let mut msg = msg.clone();
        msg.input_data = func.selector().to_vec().into();

        // execute the call
        let output = contract.execute(&mut host, &mut tracer, None, msg, Revision::latest());

        // print the revert reason if Reverted
        if output.status_code == StatusCode::Revert {
            let revert_reason =
                abi::decode(&[abi::ParamType::String], &output.output_data[4..])?[0].to_string();
            println!("{} failed. Revert reason: \"{}\"", func.name, revert_reason);
        }

        // ensure it worked
        assert_eq!(output.status_code, expected);
        println!("{}: {}", func.name, output.status_code);
    }

    Ok(())
}
