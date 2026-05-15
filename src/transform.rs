//! jq-compatible transform engine (powered by `jaq`).
//!
//! Each route compiles its filter expression once at startup; runtime hot path
//! is just `apply(payload)` per delivery.

use jaq_core::{load, Compiler, Ctx, Native, RcIter};
use jaq_json::Val;
use serde_json::Value as JsonValue;

use crate::error::{AppError, AppResult};

pub struct Transform {
    filter: jaq_core::Filter<Native<Val>>,
}

impl Transform {
    pub fn compile(src: &str) -> AppResult<Self> {
        let arena = load::Arena::default();
        let loader = load::Loader::new(jaq_std::defs());
        let file = load::File { code: src, path: () };
        let modules = loader
            .load(&arena, file)
            .map_err(|e| AppError::Transform(format!("load: {e:?}")))?;

        let filter = Compiler::<_, Native<Val>>::default()
            .with_funs(jaq_std::funs().chain(jaq_json::funs()))
            .compile(modules)
            .map_err(|e| AppError::Transform(format!("compile: {e:?}")))?;

        Ok(Self { filter })
    }

    pub fn apply(&self, payload: JsonValue) -> AppResult<JsonValue> {
        let val: Val = payload.into();
        let inputs = RcIter::new(core::iter::empty());
        let ctx = Ctx::new([], &inputs);

        let outputs: Vec<Val> = self
            .filter
            .run((ctx, val))
            .collect::<Result<_, _>>()
            .map_err(|e| AppError::Transform(format!("runtime: {e:?}")))?;

        match outputs.len() {
            0 => Err(AppError::Transform("filter produced no output".into())),
            1 => Ok(outputs.into_iter().next().unwrap().into()),
            n => Err(AppError::Transform(format!(
                "filter produced {n} outputs; expected exactly 1"
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn identity() {
        let t = Transform::compile(".").unwrap();
        let out = t.apply(json!({"a": 1})).unwrap();
        assert_eq!(out, json!({"a": 1}));
    }

    #[test]
    fn projection() {
        let t = Transform::compile("{x: .a, y: .b}").unwrap();
        let out = t.apply(json!({"a": 1, "b": 2, "c": 3})).unwrap();
        assert_eq!(out, json!({"x": 1, "y": 2}));
    }
}
