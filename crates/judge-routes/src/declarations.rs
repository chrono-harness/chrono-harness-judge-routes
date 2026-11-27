//! Execution declarations as consumed by the route planner and retained plan reader.
#[cfg(not(verus_keep_ghost))]
use crate::verified_bounds::valid_execution_bounds;
#[cfg(not(verus_keep_ghost))]
use serde::{Deserialize, Serialize};
#[cfg(not(verus_keep_ghost))]
use std::{
    collections::{BTreeMap, BTreeSet},
    num::NonZeroUsize,
    path::Path,
};

#[cfg(not(verus_keep_ghost))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Bounds {
    #[serde(with = "crate::declarations::timeout_wire")]
    pub timeout_seconds: u64,
    pub output_limit_bytes: usize,
}
#[cfg(not(verus_keep_ghost))]
impl Bounds {
    fn admissible(self) -> bool {
        valid_execution_bounds(self.timeout_seconds, self.output_limit_bytes)
    }
}

#[cfg(not(verus_keep_ghost))]
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Plan {
    pub operations: Vec<String>,
    #[serde(with = "crate::declarations::timeout_wire")]
    pub timeout_seconds: u64,
    pub output_limit_bytes: usize,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub operation_bounds: BTreeMap<String, Bounds>,
}

#[cfg(not(verus_keep_ghost))]
impl Plan {
    /// Resolve the bounds for each step before looking up a method or observing a tool.
    pub(super) fn steps(&self) -> Result<Vec<(&str, Bounds)>, &'static str> {
        let error = "E_EXECUTION_PLAN: invalid operations or bounds";
        let default = Bounds {
            timeout_seconds: self.timeout_seconds,
            output_limit_bytes: self.output_limit_bytes,
        };
        if self.operations.is_empty() || !default.admissible() {
            return Err(error);
        }
        let mut remaining = self.operation_bounds.len();
        let mut seen = BTreeSet::new();
        let mut steps = Vec::with_capacity(self.operations.len());
        for name in &self.operations {
            if name.is_empty() || !crate::verified_admission::insert_once(&mut seen, name) {
                return Err(error);
            }
            let bounds = match self.operation_bounds.get(name) {
                Some(bounds) if bounds.admissible() => {
                    remaining -= 1;
                    *bounds
                }
                Some(_) => return Err(error),
                None => default,
            };
            steps.push((name.as_str(), bounds));
        }
        if remaining != 0 {
            return Err(error);
        }
        Ok(steps)
    }
}

#[cfg(not(verus_keep_ghost))]
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Claims {
    pub resources: Vec<String>,
    pub outputs: Vec<String>,
}

#[cfg(not(verus_keep_ghost))]
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Scheduling {
    pub max_running: usize,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub priority: Vec<String>,
    pub resources: Vec<String>,
    pub claims: BTreeMap<String, Claims>,
}

#[cfg(not(verus_keep_ghost))]
#[derive(Serialize)]
pub struct Dispatch {
    pub max_running: NonZeroUsize,
    pub order: Vec<usize>,
}

#[cfg(not(verus_keep_ghost))]
fn named<'a>(names: &'a [String]) -> Option<BTreeSet<&'a str>> {
    let mut result = BTreeSet::new();
    for name in names {
        if name.is_empty() || name.contains('\0') || !result.insert(name.as_str()) {
            return None;
        }
    }
    Some(result)
}

#[cfg(not(verus_keep_ghost))]
impl Scheduling {
    pub(super) fn indices(
        &self,
        operations: &[super::Operation],
    ) -> Result<(NonZeroUsize, BTreeMap<&str, usize>), String> {
        let capacity_error =
            "E_SCHEDULING: positive max_running and unique resource names required";
        let cap = NonZeroUsize::new(self.max_running).ok_or(capacity_error)?;
        let resources = named(&self.resources).ok_or(capacity_error)?;
        let mut ranks = BTreeMap::new();
        for (rank, name) in self.priority.iter().enumerate() {
            if !crate::verified_admission::record_priority(
                &mut ranks,
                name.as_str(),
                name.as_bytes(),
                self.claims.contains_key(name),
                rank,
            ) {
                return Err(
                    "E_SCHEDULING: priority must list distinct registered operation IDs".into(),
                );
            }
        }
        for (op, claim) in &self.claims {
            let error = || format!("E_SCHEDULING: invalid claims for {op}");
            let requested = named(&claim.resources).ok_or_else(error)?;
            if op.is_empty() || !requested.is_subset(&resources) {
                return Err(error());
            }
            let mut paths = BTreeSet::new();
            for output in &claim.outputs {
                if relative_path(output.trim_end_matches('/')).is_err()
                    || !paths.insert(Path::new(output))
                {
                    return Err(error());
                }
            }
        }

        for op in operations {
            if !self.claims.contains_key(&op.method.operation) {
                return Err(format!(
                    "E_SCHEDULING: missing claims for {}",
                    op.method.operation
                ));
            }
        }
        Ok((cap, ranks))
    }
}

#[cfg(not(verus_keep_ghost))]
impl Dispatch {
    pub(super) fn for_plan(plan: &super::Execution) -> Result<Self, String> {
        let mut order: Vec<_> = (0..plan.operations.len()).collect();
        let Some(policy) = &plan.scheduling else {
            return Ok(Self {
                max_running: NonZeroUsize::MIN,
                order,
            });
        };
        let (max_running, ranks) = policy.indices(&plan.operations)?;
        if !ranks.is_empty() {
            order.sort_by_key(|i| {
                crate::verified_admission::priority_key(
                    &ranks,
                    &plan.operations[*i].method.operation.as_str(),
                )
            });
        }
        Ok(Self { max_running, order })
    }
}

#[cfg(not(verus_keep_ghost))]
pub(super) fn relative_path(value: &str) -> Result<(), String> {
    if value.is_empty()
        || value.starts_with('/')
        || value == "."
        || value.starts_with("./")
        || value.contains(['\0', '\n', '\r', '\\'])
        || value.split('/').any(|part| part == "..")
    {
        return Err(format!("invalid relative path: {value:?}"));
    }
    Ok(())
}

/// Public execution bounds use JSON numbers in the interoperable range and
/// canonical decimal strings above it so a v1 JCS transport preserves them.
#[cfg(not(verus_keep_ghost))]
pub(crate) mod timeout_wire {
    use serde::{Deserialize, Deserializer, Serializer, de::Error};

    pub fn serialize<S: Serializer>(seconds: &u64, s: S) -> Result<S::Ok, S::Error> {
        if !super::decimal_timeout(*seconds) {
            s.serialize_u64(*seconds)
        } else {
            s.collect_str(seconds)
        }
    }

    pub fn observation<S: Serializer>(
        process: &crate::ProcessResult,
        s: S,
    ) -> Result<S::Ok, S::Error> {
        use serde::Serialize;
        if let Some(seconds) = crate::evidence::exact_observation_timeout(process.timeout_seconds) {
            let mut value = serde_json::to_value(process).map_err(serde::ser::Error::custom)?;
            value["timeout_seconds"] = serde_json::Value::String(seconds.to_string());
            value.serialize(s)
        } else {
            process.serialize(s)
        }
    }

    pub mod optional {
        use super::*;
        #[derive(Deserialize)]
        struct Wrapped(#[serde(deserialize_with = "super::deserialize")] u64);

        pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<u64>, D::Error> {
            Ok(Option::<Wrapped>::deserialize(d)?.map(|wrapped| wrapped.0))
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<u64, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Timeout {
            Number(u64),
            Decimal(String),
        }
        match Timeout::deserialize(d)? {
            Timeout::Number(value) => Ok(value),
            Timeout::Decimal(text) => text
                .parse::<u64>()
                .ok()
                .filter(|n| super::decimal_timeout(*n) && n.to_string() == text)
                .ok_or_else(|| D::Error::custom("invalid exact timeout")),
        }
    }
}

#[cfg(not(verus_keep_ghost))]
use vstd::prelude::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {
/// Only JSON's interoperable integer range can use a number token on the v2 wire.
pub fn decimal_timeout(seconds: u64) -> (use_text: bool)
    ensures use_text == (seconds as int >= 9007199254740992),
{ seconds >= 9007199254740992u64 }
}
