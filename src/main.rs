use ethers::{
    abi::FunctionExt,
    types::*,
    utils::{id, Solc},
};
use evmodin::{
    host::DummyHost, tracing::NoopTracer, AnalyzedCode, CallKind, Message, Revision, StatusCode,
};

// Features:
// 1. Specify contract name (moduel)
// 2. Pattern on test name functions
#[tokio::main]
async fn main() -> eyre::Result<()> {
    let compiled = Solc::new("./*.sol").build()?;
    let contract = compiled.get("Greet").expect("could not find contract");

    // get all the test functions
    let test_fns = contract
        .abi
        .functions()
        .into_iter()
        .filter(|func| func.name.starts_with("test"));

    // get the contract bytecode (no constructor args)
    let bytecode = contract.bytecode.clone().to_vec();

    // setup the contract
    let contract = AnalyzedCode::analyze(bytecode);

    // setup the host
    // 1. balances
    // 2. initial state (block timestamp etc.)

    let mut host = DummyHost;
    let mut tracer = NoopTracer;

    let setup_id = id("setUp()").to_vec();
    let gas = 10_000_000;
    let msg = Message {
        kind: CallKind::Call,
        is_static: true,
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

    for func in test_fns {
        println!("Testing {}", func.name);
        let mut msg = msg.clone();
        msg.input_data = func.selector().to_vec().into();

        dbg!(&msg);

        let expected = if func.name.contains("testFail") {
            StatusCode::Failure
        } else {
            StatusCode::Success
        };

        let output = contract.execute(&mut host, &mut tracer, None, msg, Revision::latest());
        dbg!(&output);
        assert_eq!(output.status_code, expected);
    }

    Ok(())
}
