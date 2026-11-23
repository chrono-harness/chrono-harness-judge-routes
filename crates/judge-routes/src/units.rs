//! Global assignment and scoped selection for route plans.
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Unit {
    pub tests: Vec<String>,
    pub report_path: String,
}

/// Validate and select the explicitly registered full execution units.  This
/// is deliberately a pure map operation shared by the full route and the
/// scoped adapter; it never discovers plans or derives ownership from paths.
pub fn assignments(
    units: &BTreeMap<String, Unit>,
    plans: &BTreeSet<String>,
    shared: &BTreeMap<String, Vec<String>>,
    operation_sets: &BTreeMap<String, BTreeSet<String>>,
) -> Result<(), String> {
    let mut owners = BTreeMap::<String, String>::new();
    for (unit, definition) in units {
        id(unit)?;
        for test in &definition.tests {
            if !plans.contains(test) {
                return Err(format!("unknown CI unit plan: {test}"));
            }
            if owners.insert(test.clone(), unit.clone()).is_some() {
                return Err(format!("duplicate CI unit plan assignment: {test}"));
            }
        }
    }
    if owners.keys().collect::<BTreeSet<_>>() != plans.iter().collect::<BTreeSet<_>>() {
        return Err("missing CI unit plan assignment".into());
    }
    let mut expected = BTreeMap::<String, BTreeSet<String>>::new();
    for (operation, names) in shared {
        let set: BTreeSet<_> = names.iter().cloned().collect();
        if set.len() != names.len() || set.len() < 2 || names.iter().any(|u| !units.contains_key(u))
        {
            return Err(format!("invalid shared operation assignment: {operation}"));
        }
        expected.insert(operation.clone(), set);
    }
    let actual: BTreeMap<_, _> = operation_sets
        .iter()
        .filter(|(_, names)| names.len() > 1)
        .map(|(operation, names)| (operation.clone(), names.clone()))
        .collect();
    if actual != expected {
        return Err(
            "shared operations must explicitly name exactly the units that repeat them".into(),
        );
    }
    Ok(())
}

pub fn select(
    units: &BTreeMap<String, Unit>,
    selected: &BTreeSet<String>,
    unit: &str,
) -> Result<BTreeSet<String>, String> {
    id(unit)?;
    Ok(units
        .get(unit)
        .ok_or("unregistered CI unit")?
        .tests
        .iter()
        .filter(|test| selected.contains(*test))
        .cloned()
        .collect())
}

pub fn required(units: &BTreeMap<String, Unit>, selected: &BTreeSet<String>) -> Vec<String> {
    units
        .iter()
        .filter(|(_, definition)| definition.tests.iter().any(|test| selected.contains(test)))
        .map(|(id, _)| id.clone())
        .collect()
}

fn id(id: &str) -> Result<(), String> {
    if id.is_empty()
        || !id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
    {
        return Err(format!("invalid CI unit ID: {id:?}"));
    }
    Ok(())
}
