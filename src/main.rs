use ethers::{
    abi::{self, FunctionExt},
    types::*,
    utils::{id, Solc},
};
use evmodin::{
    util::mocked_host::MockedHost,
    continuation::resume_data::StateModifier,
    tracing::{NoopTracer, StdoutTracer, Tracer},
    AnalyzedCode, CallKind, ExecutionState, Message, Revision, StatusCode,
};
use std::sync::Arc;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    // Note: This host does not support x-contract calls. How are we going to handle it?!
    let host = MockedHost::default();

    if std::env::var("TRACE").is_ok() {
        let mut tracer = StdoutTracer::default();
        run(host, &mut tracer)
    } else {
        run(host, &mut NoopTracer)
    }
}

fn run<T: Tracer>(mut host: MockedHost, tracer: &mut T) -> eyre::Result<()> {
    // compile the contracts
    let compiled = Solc::new("./*.sol").build()?;
    let compiled = compiled.get("Greet").expect("could not find contract");

    // get the contract bytecode (no constructor args)
    let bytecode = compiled.runtime_bytecode.clone().to_vec();

    // setup the contract
    let contract = AnalyzedCode::analyze(bytecode);

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
    let output = contract.execute(&mut host, tracer, None, msg.clone(), Revision::latest());
    assert_eq!(output.status_code, StatusCode::Success);

    // get all the test functions
    let test_fns = compiled
        .abi
        .functions()
        .into_iter()
        .filter(|func| func.name.starts_with("test"));

    // call all the test functions
    for func in test_fns {
        // Reset the host's state in each test run
        let mut host = host.clone();
        // the expected result depends on the function name
        let expected = if func.name.contains("testFail") {
            StatusCode::Revert
        } else {
            StatusCode::Success
        };

        // set the selector
        let mut msg = msg.clone();
        msg.input_data = func.selector().to_vec().into();

        // Ideally, we should be able to hook on any message and override it with the
        // state modifier. We cannot test this currently because x-contract calls
        // are not implemented.
        let state_modifier: StateModifier = Some(Arc::new(|state: &mut ExecutionState| {
            let message = state.message_mut();
            #[allow(unused)]
            let hevm = "0x7109709ECfa91a80626fF3989D68f67F5b1DD12D"
                .parse()
                .unwrap();
            if message.destination == hevm {
                println!("Got call to HEVM");

                let sig = hex::encode(&message.input_data[0..4]);
                #[allow(unused)]
                let input = &message.input_data[4..];
                match sig {
                    // roll - sets block number
                    _ if sig == *"1f7b4f30" => {
                        // TODO: Figure out how to do this, we cannot right now because
                        // we are borrowing the host inside the closure while also mutably
                        // borrowing it later in the execute call, which Rust does not
                        // allow.
                        // host.set_block_number(U256::from_big_endian(&input[4..]));
                    }
                    _ => {
                        panic!("Unknown cheat code");
                    }
                };
            }
        }));

        // execute the call
        let output = contract.execute(&mut host, tracer, state_modifier, msg, Revision::latest());

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
