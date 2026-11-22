use chrono_judge_registration::Registrations;
use serde::Deserialize;
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    io::{Read, Write},
};

/// Shared global obligations; callers select a transport scope only after this succeeds.
pub fn global_selection(
    old: &Registrations,
    r: &Registrations,
    tests: &[String],
    retired_tests: &[String],
    deferred: bool,
) -> Result<BTreeSet<String>, String> {
    let mut selected: BTreeSet<_> = tests.iter().cloned().collect();
    for test in retired_tests {
        match chrono_judge_registration::validate_test_retirement(old, r, test)? {
            Some(replacement) => {
                selected.insert(replacement);
            }
            // The downstream validator approves joint removal after consuming
            // current execution results; historical methods remain input data.
            None if deferred => {}
            None => return Err(format!("E_REQUIRED_TEST_REMOVED: {test}")),
        }
    }
    Ok(selected)
}

#[derive(Deserialize)]
struct SelectionRequest {
    config: String,
    base: BTreeMap<String, Value>,
    candidate: BTreeMap<String, Value>,
    tests: Vec<String>,
    retired_tests: Vec<String>,
}

pub fn select_from_registries(input: impl Read, output: impl Write) -> Result<(), String> {
    let request: SelectionRequest = serde_json::from_reader(input).map_err(|e| e.to_string())?;
    let old = Registrations::load(&request.base, &request.config)?;
    let current = Registrations::load(&request.candidate, &request.config)?;
    let selected = global_selection(
        &old,
        &current,
        &request.tests,
        &request.retired_tests,
        chrono_judge_registration::downstream_validator(&current, "routes"),
    )?;
    serde_json::to_writer(output, &selected).map_err(|e| e.to_string())
}
