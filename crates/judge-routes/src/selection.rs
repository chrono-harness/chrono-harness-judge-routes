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
    write_selection(
        &old,
        &current,
        &request.tests,
        &request.retired_tests,
        output,
    )
}

pub fn select_collected(
    root: &std::path::Path,
    input: impl Read,
    output: impl Write,
) -> Result<(), String> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct MigrationInputs {
        units: Vec<String>,
        originals: chrono_harness::units::FullManifest,
    }
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Query {
        request: chrono_harness::wire::Request,
        migration: Option<MigrationInputs>,
        tests: Vec<String>,
        retired_tests: Vec<String>,
    }
    let query: Query = serde_json::from_reader(input).map_err(|e| e.to_string())?;
    let req = &query.request;
    if req.candidate.root != root
        || !matches!(
            req.scope,
            Some(chrono_harness::units::Scope::Collect { .. })
        )
    {
        return Err("route collection requires this checkout and a collection scope".into());
    }
    let reader = chrono_harness::facts::Reader::for_request(req)?;
    let (old, current, _) =
        chrono_judge_registration::views_for_collection_inputs_with(req, &reader, &|units| {
            let bound = query
                .migration
                .as_ref()
                .filter(|m| m.units == units)
                .ok_or("missing original migration inputs for the selected units")?;
            Ok(bound.originals.clone())
        })?;
    write_selection(&old, &current, &query.tests, &query.retired_tests, output)
}

fn write_selection(
    old: &Registrations,
    current: &Registrations,
    tests: &[String],
    retired: &[String],
    mut output: impl Write,
) -> Result<(), String> {
    let selected = global_selection(
        old,
        current,
        tests,
        retired,
        chrono_judge_registration::downstream_validator(current, "routes"),
    )?;
    serde_json::to_writer(&mut output, &selected).map_err(|e| e.to_string())?;
    output.flush().map_err(|e| e.to_string())
}
