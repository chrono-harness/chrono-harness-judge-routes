//! Execution declarations as consumed by the route planner and retained plan reader.
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    num::NonZeroUsize,
    path::Path,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Bounds {
    pub timeout_seconds: u64,
    pub output_limit_bytes: usize,
}
impl Bounds {
    fn admissible(self) -> bool {
        chrono_harness::verified_limits::valid_execution_bounds(
            self.timeout_seconds,
            self.output_limit_bytes,
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Plan {
    pub operations: Vec<String>,
    pub timeout_seconds: u64,
    pub output_limit_bytes: usize,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub operation_bounds: BTreeMap<String, Bounds>,
}

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
            if name.is_empty() || !seen.insert(name) {
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

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Claims {
    pub resources: Vec<String>,
    pub outputs: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Scheduling {
    pub max_running: usize,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub priority: Vec<String>,
    pub resources: Vec<String>,
    pub claims: BTreeMap<String, Claims>,
}

#[derive(Serialize)]
pub struct Dispatch {
    pub max_running: NonZeroUsize,
    pub order: Vec<usize>,
}

fn named<'a>(names: &'a [String]) -> Option<BTreeSet<&'a str>> {
    let mut result = BTreeSet::new();
    for name in names {
        if name.is_empty() || name.contains('\0') || !result.insert(name.as_str()) {
            return None;
        }
    }
    Some(result)
}

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
            if name.is_empty()
                || name.contains('\0')
                || !self.claims.contains_key(name)
                || ranks.insert(name.as_str(), rank).is_some()
            {
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
                if chrono_harness::relative_path(output.trim_end_matches('/')).is_err()
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
                ranks
                    .get(plan.operations[*i].method.operation.as_str())
                    .copied()
                    .unwrap_or(usize::MAX)
            });
        }
        Ok(Self { max_running, order })
    }
}
