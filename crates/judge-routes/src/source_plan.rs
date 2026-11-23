//! Check proof obligations using the original registration views and fixed Git endpoints.
use crate::{Method, Operation, validate_proof_plan};
use chrono_harness::{facts, verified_verifier::sources};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{collections::BTreeMap, io::Read, path::Path};

impl crate::verified_source_pipeline::InventoryPath for sources::SourceFile {
    fn path(&self) -> &str {
        &self.path
    }
}

pub(super) type EndpointData<'a> = (&'a str, &'a Value, &'a Value, Vec<String>);

pub(super) fn validate<'a>(
    previous: Option<EndpointData<'a>>,
    current: EndpointData<'a>,
    affected: &Vec<String>,
    methods: &BTreeMap<String, Vec<Method>>,
    operations: &[Operation],
    observe: impl FnOnce(
        Option<EndpointData<'a>>,
        EndpointData<'a>,
        &Vec<String>,
    ) -> Result<sources::continuity::Continuity, String>,
) -> Result<(), String> {
    let baseline = previous
        .as_ref()
        .map(|endpoint| serde_json::from_value(endpoint.1.clone()))
        .transpose()
        .map_err(|error| error.to_string());
    let registry: sources::SourceRegistry =
        serde_json::from_value(current.1.clone()).map_err(|error| error.to_string())?;
    let paths = crate::verified_source_pipeline::continuity_paths(
        &registry.files.entries,
        registry.files.require_new_file_classification,
        affected,
    );
    crate::verified_source_pipeline::with_source_continuity(
        (),
        previous,
        current,
        &paths,
        |state, previous, current, paths| (state, observe(previous, current, paths)),
        |state, continuity| {
            (
                state,
                validate_proof_plan(
                    &registry,
                    baseline
                        .as_ref()
                        .map(Option::as_ref)
                        .map_err(String::as_str),
                    &affected.iter().cloned().collect(),
                    methods,
                    operations,
                    &continuity,
                ),
            )
        },
    )
    .1
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Endpoint {
    commit: String,
    filemap: Value,
    projects: Value,
    artifacts: Vec<String>,
}

impl Endpoint {
    fn view(&self) -> EndpointData<'_> {
        (
            &self.commit,
            &self.filemap,
            &self.projects,
            self.artifacts.clone(),
        )
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct GitBinding {
    config: String,
    git_facts: Value,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Query {
    previous: Option<Endpoint>,
    candidate: Endpoint,
    affected: Vec<String>,
    methods: BTreeMap<String, Vec<Method>>,
    operations: Vec<Operation>,
    git: Option<GitBinding>,
}

pub fn query(root: &Path, input: impl Read) -> Result<Value, String> {
    let q: Query = serde_json::from_reader(input).map_err(|error| error.to_string())?;
    facts::full_oid(&q.candidate.commit)?;
    if let Some(previous) = &q.previous {
        facts::full_oid(&previous.commit)?;
    }
    let mut observation = Value::Null;
    let result = validate(
        q.previous.as_ref().map(Endpoint::view),
        q.candidate.view(),
        &q.affected,
        &q.methods,
        &q.operations,
        |previous, candidate, paths| {
            if paths.is_empty() {
                return Ok(sources::continuity::Continuity::empty());
            }
            let reader = match &q.git {
                None => facts::Reader::legacy(),
                Some(binding) => facts::Reader::from_observations(
                    root,
                    &binding.config,
                    candidate.0,
                    &json!({"git_facts":binding.git_facts,"environment":binding.git_facts["environment"]}),
                )?,
            };
            let result = sources::continuity::observe(&reader, root, previous, candidate, paths);
            observation = reader.observation();
            result
        },
    );
    Ok(
        json!({"previous": q.previous.as_ref().map(|e| &e.commit), "candidate":q.candidate.commit,
        "git_facts":observation, "error":result.err()}),
    )
}
