//! Release acceptance for the release binary JSON result and original invalid-input response.
#![allow(unexpected_cfgs)]
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {
pub fn expected_text() -> (value: &'static str)
    ensures value@ == "test:example"@,
{ "test:example" }

pub fn accepts(exit: i32, error_bytes: u64, count: Option<usize>, first_matches: bool) -> (accepted: bool)
    ensures accepted == (exit == 2 && error_bytes > 0 && count == Some(1usize) && first_matches),
{
    exit == 2 && error_bytes > 0 && count == Some(1usize) && first_matches
}
}

#[cfg(not(verus_keep_ghost))]
fn main() -> Result<(), String> {
    let arguments: Vec<_> = std::env::args().skip(1).collect();
    let [exit, error_file, output_file] = arguments.as_slice() else {
        return Err("usage: release_observation EXIT ERROR_FILE OUTPUT_FILE".into());
    };
    let exit = exit.parse::<i32>().map_err(|e| e.to_string())?;
    let error_bytes = std::fs::metadata(error_file)
        .map_err(|e| e.to_string())?
        .len();
    let output = std::fs::read(output_file).map_err(|e| e.to_string())?;
    let mut results =
        serde_json::Deserializer::from_slice(&output).into_iter::<serde_json::Value>();
    let mut admitted = false;
    for result in &mut results {
        let result = result.map_err(|e| e.to_string())?;
        let selected = result.as_array();
        let count = selected.map(Vec::len);
        let first = selected
            .and_then(|items| items.first())
            .and_then(serde_json::Value::as_str);
        admitted = accepts(exit, error_bytes, count, first == Some(expected_text()));
    }
    if admitted {
        Ok(())
    } else {
        Err(format!("unexpected release observation for {arguments:?}"))
    }
}
