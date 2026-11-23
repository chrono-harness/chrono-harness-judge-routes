#![allow(unexpected_cfgs)]
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {

pub open spec fn source_pipeline_result<S, P, C, N, T, R, E, O, V>(
    initial: S, previous: P, candidate: C, paths: N, observe: O, validate: V,
    out: (S, Result<R, E>),
) -> bool
    where O: FnOnce(S, P, C, N) -> (S, Result<T, E>),
        V: FnOnce(S, T) -> (S, Result<R, E>),
{
    exists|observed: (S, Result<T, E>)|
        observe.ensures((initial, previous, candidate, paths), observed)
        && match observed.1 {
            Err(error) => out.0 == observed.0 && out.1 == Err::<R, E>(error),
            Ok(evidence) => validate.ensures((observed.0, evidence), out),
        }
}

/// 原两端和所选路径只观察一次；验证只接收该次观察的完整原结果。
pub fn with_source_continuity<S, P, C, N, T, R, E, O, V>(
    state: S, previous: P, candidate: C, paths: N, observe: O, validate: V,
) -> (out: (S, Result<R, E>))
    where O: FnOnce(S, P, C, N) -> (S, Result<T, E>),
        V: FnOnce(S, T) -> (S, Result<R, E>),
    requires forall|s: S, p: P, c: C, n: N| observe.requires((s, p, c, n)),
        forall|s: S, evidence: T| validate.requires((s, evidence)),
    ensures source_pipeline_result(state, previous, candidate, paths, observe, validate, out),
{
    let ghost initial = state;
    let ghost p = previous;
    let ghost c = candidate;
    let ghost n = paths;
    let observed = observe(state, previous, candidate, paths);
    let ghost observation = observed;
    let (state, result) = observed;
    let out = match result {
        Err(error) => (state, Err(error)),
        Ok(evidence) => validate(state, evidence),
    };
    proof {
        assert(observe.ensures((initial, p, c, n), observation));
        assert(exists|observed: (S, Result<T, E>)|
            observe.ensures((initial, p, c, n), observed)
            && match observed.1 {
                Err(error) => out.0 == observed.0 && out.1 == Err::<R, E>(error),
                Ok(evidence) => validate.ensures((observed.0, evidence), out),
            });
    }
    out
}

} // verus!
