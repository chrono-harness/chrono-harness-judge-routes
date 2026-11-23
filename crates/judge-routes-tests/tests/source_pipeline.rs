#![allow(unexpected_cfgs)]
#[cfg(verus_keep_ghost)]
#[path = "../../judge-routes/src/verified_source_pipeline.rs"]
mod policy;
#[cfg(not(verus_keep_ghost))]
use chrono_judge_routes::verified_source_pipeline as policy;
#[cfg(verus_keep_ghost)]
use vstd::vpanic as panic;
use vstd::{contrib::exec_spec::ExecSpecEq, prelude::*};

verus! {
fn expect_continuity_bool(actual: bool)
    requires actual,
{ if !actual { panic!(); } }

#[cfg_attr(not(verus_keep_ghost), test)]
fn continuity_pipeline_preserves_both_endpoints_paths_and_observed_evidence() {
    let (trace, result) = policy::with_source_continuity(
        Vec::<u8>::new(), 11u8, 22u8, 33u8,
        |mut state: Vec<u8>, previous: u8, candidate: u8, paths: u8| -> (out: (Vec<u8>, Result<(u8,u8,u8), String>))
            ensures out.0@ == state@.push(1u8).push(previous).push(candidate).push(paths),
                out.1 == Ok::<(u8,u8,u8), String>((previous,candidate,paths)),
        {
            state.push(1); state.push(previous); state.push(candidate); state.push(paths);
            (state, Ok((previous,candidate,paths)))
        },
        |mut state: Vec<u8>, evidence: (u8,u8,u8)| -> (out: (Vec<u8>, Result<(u8,u8,u8), String>))
            ensures out.0@ == state@.push(2u8).push(evidence.0).push(evidence.1).push(evidence.2),
                out.1 == Ok::<(u8,u8,u8), String>(evidence),
        {
            state.push(2); state.push(evidence.0); state.push(evidence.1); state.push(evidence.2);
            (state, Ok(evidence))
        },
    );
    expect_continuity_bool(trace.len() == 8);
    expect_continuity_bool(trace[0] == 1 && trace[1] == 11 && trace[2] == 22 && trace[3] == 33);
    expect_continuity_bool(trace[4] == 2 && trace[5] == 11 && trace[6] == 22 && trace[7] == 33);
    let exact = match result { Ok(value) => value.0 == 11 && value.1 == 22 && value.2 == 33, Err(_) => false };
    expect_continuity_bool(exact);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn continuity_observation_failure_keeps_the_first_error_and_skips_validation() {
    let (trace, result) = policy::with_source_continuity(
        Vec::<u8>::new(), (), (), (),
        |mut state: Vec<u8>, _previous: (), _candidate: (), _paths: ()| -> (out: (Vec<u8>, Result<u8, String>))
            ensures out.0@ == state@.push(1u8),
                match out.1 { Err(error) => error@ == "原读取错误\0é"@, Ok(_) => false },
        { state.push(1); (state, Err("原读取错误\0é".to_string())) },
        |mut state: Vec<u8>, evidence: u8| -> (out: (Vec<u8>, Result<u8, String>))
            ensures out.0@ == state@.push(9u8), out.1 == Ok::<u8,String>(evidence),
        { state.push(9); (state, Ok(evidence)) },
    );
    expect_continuity_bool(trace.len() == 1 && trace[0] == 1);
    let exact = match result { Err(error) => <&str>::exec_eq(error.as_str(), "原读取错误\0é"), Ok(_) => false };
    expect_continuity_bool(exact);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn continuity_validation_failure_keeps_its_full_original_error() {
    let (trace, result) = policy::with_source_continuity(
        Vec::<u8>::new(), (), (), 7u8,
        |mut state: Vec<u8>, _previous: (), _candidate: (), paths: u8| -> (out: (Vec<u8>, Result<u8, String>))
            ensures out.0@ == state@.push(1u8), out.1 == Ok::<u8,String>(paths),
        { state.push(1); (state, Ok(paths)) },
        |mut state: Vec<u8>, evidence: u8| -> (out: (Vec<u8>, Result<u8, String>))
            ensures out.0@ == state@.push(evidence),
                match out.1 { Err(error) => error@ == "原判定错误\0é"@, Ok(_) => false },
        { state.push(evidence); (state, Err("原判定错误\0é".to_string())) },
    );
    expect_continuity_bool(trace.len() == 2 && trace[0] == 1 && trace[1] == 7);
    let exact = match result { Err(error) => <&str>::exec_eq(error.as_str(), "原判定错误\0é"), Ok(_) => false };
    expect_continuity_bool(exact);
}
}
