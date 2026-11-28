//! Execution declarations as consumed by the route planner and retained plan reader.
#[cfg(verus_keep_ghost)]
use super::{execution_bounds_valid, valid_execution_bounds};
#[cfg(not(verus_keep_ghost))]
use crate::verified_bounds::valid_execution_bounds;
#[cfg(not(verus_keep_ghost))]
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
#[cfg(not(verus_keep_ghost))]
use std::{num::NonZeroUsize, path::Path};

verus! {
#[derive(Clone, Copy)]
#[cfg_attr(not(verus_keep_ghost), derive(Debug, PartialEq, Eq, Deserialize, Serialize))]
#[cfg_attr(not(verus_keep_ghost), serde(deny_unknown_fields))]
pub struct Bounds {
    #[cfg_attr(not(verus_keep_ghost), serde(with = "crate::declarations::timeout_wire"))]
    pub timeout_seconds: u64,
    pub output_limit_bytes: usize,
}

impl Bounds {
    pub open spec fn valid(self) -> bool {
        execution_bounds_valid(self.timeout_seconds, self.output_limit_bytes)
    }

    fn admissible(self) -> (valid: bool)
        ensures valid == self.valid(),
    {
        valid_execution_bounds(self.timeout_seconds, self.output_limit_bytes)
    }
}

#[cfg_attr(not(verus_keep_ghost), derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize))]
#[cfg_attr(not(verus_keep_ghost), serde(deny_unknown_fields))]
pub struct Plan {
    pub operations: Vec<String>,
    #[cfg_attr(not(verus_keep_ghost), serde(with = "crate::declarations::timeout_wire"))]
    pub timeout_seconds: u64,
    pub output_limit_bytes: usize,
    #[cfg_attr(not(verus_keep_ghost), serde(default, skip_serializing_if = "BTreeMap::is_empty"))]
    pub operation_bounds: BTreeMap<String, Bounds>,
}

impl Plan {
    pub open spec fn default_bounds(&self) -> Bounds {
        Bounds { timeout_seconds: self.timeout_seconds, output_limit_bytes: self.output_limit_bytes }
    }

    pub open spec fn valid_steps(&self) -> bool {
        let operations = self.operations@;
        &&& operations.len() > 0
        &&& self.default_bounds().valid()
        &&& operations.no_duplicates()
        &&& forall|i: int| 0 <= i < operations.len() ==> (#[trigger] operations[i])@.len() > 0
        &&& forall|id: String| #[trigger] self.operation_bounds@.contains_key(id)
            ==> operations.contains(id) && self.operation_bounds@[id].valid()
    }

    pub open spec fn bounds_for(&self, name: String) -> Bounds {
        if self.operation_bounds@.contains_key(name) {
            self.operation_bounds@[name]
        } else {
            self.default_bounds()
        }
    }

    /// Resolve every bound before the planner looks up any methods or observes tools.
    pub fn steps(&self) -> (result: Result<Vec<(&String, Bounds)>, &'static str>)
        requires vstd::laws_cmp::obeys_cmp::<String>(), vstd::laws_cmp::obeys_cmp::<&String>(),
        ensures
            result.is_ok() == self.valid_steps(),
            result matches Ok(steps) ==> steps@.len() == self.operations@.len()
                && (forall|i: int| 0 <= i < steps@.len() ==>
                    *(#[trigger] steps@[i]).0 == self.operations@[i]
                    && steps@[i].1 == self.bounds_for(self.operations@[i])),
            result matches Err(reason) ==> reason@ == "E_EXECUTION_PLAN: invalid operations or bounds"@,
    {
        broadcast use vstd::std_specs::btree::axiom_set_contains_deref_key;
        broadcast use vstd::std_specs::btree::axiom_maps_deref_key_to_value;
        let error = "E_EXECUTION_PLAN: invalid operations or bounds";
        let default = Bounds {
            timeout_seconds: self.timeout_seconds,
            output_limit_bytes: self.output_limit_bytes,
        };
        if self.operations.len() == 0 || !default.admissible() {
            return Err(error);
        }
        let mut seen: BTreeSet<&String> = BTreeSet::new();
        let mut steps: Vec<(&String, Bounds)> = Vec::with_capacity(self.operations.len());
        let mut i = 0;
        while i < self.operations.len()
            invariant
                error@ == "E_EXECUTION_PLAN: invalid operations or bounds"@,
                i <= self.operations@.len(),
                self.operations@.len() > 0,
                default == self.default_bounds(),
                default.valid(),
                vstd::laws_cmp::obeys_cmp::<String>(),
                vstd::laws_cmp::obeys_cmp::<&String>(),
                steps@.len() == i,
                forall|j: int| 0 <= j < i ==> *(#[trigger] steps@[j]).0 == self.operations@[j]
                    && steps@[j].1 == self.bounds_for(self.operations@[j]) && steps@[j].1.valid(),
                forall|j: int| 0 <= j < i ==> (#[trigger] self.operations@[j])@.len() > 0,
                self.operations@.take(i as int).no_duplicates(),
                forall|name: &String| #[trigger] seen@.contains(name) ==
                    self.operations@.take(i as int).contains(*name),
            decreases self.operations@.len() - i,
        {
            let name = &self.operations[i];
            if name.as_str().is_empty() {
                assert(self.operations@[i as int]@.len() == 0);
                return Err(error);
            }
            let ghost before = seen@;
            if !seen.insert(name) {
                assert(self.operations@.take(i as int).contains(*name));
                assert(!self.operations@.no_duplicates());
                return Err(error);
            }
            let bounds = match self.operation_bounds.get(name) {
                Some(bounds) => {
                    if !bounds.admissible() {
                        assert(self.operation_bounds@[*name] == *bounds);
                        return Err(error);
                    }
                    *bounds
                },
                None => default,
            };
            assert(self.operations@.take((i + 1) as int) =~=
                self.operations@.take(i as int).push(*name));
            proof {
                let prefix = self.operations@.take(i as int);
                prefix.lemma_push_to_set_commute(*name);
                let next_prefix = self.operations@.take(i as int + 1);
                assert forall|key: &String| #[trigger] seen@.contains(key) ==
                    next_prefix.contains(*key) by {
                    assert(before.contains(key) == prefix.contains(*key));
                    assert((key == name) == (*key == *name));
                    assert(seen@.contains(key) == (before.contains(key) || key == name));
                    assert(next_prefix.to_set().contains(*key) ==
                        (prefix.to_set().contains(*key) || *key == *name));
                    assert(prefix.to_set().contains(*key) == prefix.contains(*key));
                    assert(next_prefix.to_set().contains(*key) == next_prefix.contains(*key));
                }
            }
            steps.push((name, bounds));
            i += 1;
        }
        assert(self.operations@.take(i as int) =~= self.operations@);
        let mut overrides = self.operation_bounds.iter();
        let ghost entries = overrides@.1;
        loop
            invariant
                error@ == "E_EXECUTION_PLAN: invalid operations or bounds"@,
                self.operations@.len() > 0,
                default == self.default_bounds(),
                default.valid(),
                vstd::laws_cmp::obeys_cmp::<String>(),
                vstd::laws_cmp::obeys_cmp::<&String>(),
                self.operations@.no_duplicates(),
                steps@.len() == self.operations@.len(),
                forall|j: int| 0 <= j < steps@.len() ==>
                    *(#[trigger] steps@[j]).0 == self.operations@[j]
                    && steps@[j].1 == self.bounds_for(self.operations@[j]) && steps@[j].1.valid(),
                forall|j: int| 0 <= j < self.operations@.len() ==> (#[trigger] self.operations@[j])@.len() > 0,
                forall|name: &String| #[trigger] seen@.contains(name) == self.operations@.contains(*name),
                0 <= overrides@.0 <= entries.len(),
                overrides@.1 == entries,
                entries.to_set() == self.operation_bounds@.kv_pairs(),
                forall|j: int| 0 <= j < overrides@.0 ==> self.operations@.contains((#[trigger] entries[j]).0),
            decreases entries.len() - overrides@.0 + 1,
        {
            match overrides.next() {
                Some((name, _)) => {
                    if !seen.contains(&name) {
                        assert(self.operation_bounds@.contains_key(*name));
                        return Err(error);
                    }
                },
                None => {
                    assert forall|name: String| #[trigger] self.operation_bounds@.contains_key(name)
                        implies self.operations@.contains(name) && self.operation_bounds@[name].valid() by {
                        let entry = (name, self.operation_bounds@[name]);
                        assert(entries.to_set().contains(entry));
                        let j = choose|j: int| 0 <= j < entries.len() && entries[j] == entry;
                        let k = choose|k: int| 0 <= k < self.operations@.len() && self.operations@[k] == name;
                        assert(steps@[k].1.valid());
                    }
                    return Ok(steps);
                },
            }
        }
    }
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
