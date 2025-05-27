use napi::bindgen_prelude::*;
use napi_derive::napi;
use std::path::{Path, PathBuf};
use globset::{Glob, GlobSet, GlobSetBuilder};
use ignore::WalkBuilder;

#[napi(object)]
#[derive(Clone)]
pub struct GlobOptions {
    pub exclude: Option<Vec<String>>,
    pub cwd: Option<String>,
}

fn build_globset(patterns: &[String]) -> Result<GlobSet> {
    let mut builder = GlobSetBuilder::new();
    for pat in patterns {
        builder.add(Glob::new(pat).map_err(|e| {
            Error::new(Status::InvalidArg, format!("Invalid glob pattern '{}': {}", pat, e))
        })?);
    }
    builder.build()
        .map_err(|e| Error::new(Status::InvalidArg, format!("GlobSet build error: {}", e)))
}

fn walk_and_filter(cwd: &Path, include: &GlobSet, exclude: &GlobSet) -> Result<Vec<String>> {
    let walker = WalkBuilder::new(cwd)
        .standard_filters(true)
        .build_parallel();

    let results = std::sync::Mutex::new(Vec::new());

    walker.run(|| {
        let results = &results;
        Box::new(move |result| {
            let entry = match result {
                Ok(e) => e,
                Err(_) => return ignore::WalkState::Continue,
            };

            let path = entry.path();
            let relative_path = path.strip_prefix(cwd).unwrap_or(path);
            
            if exclude.is_match(relative_path) {
                return ignore::WalkState::Continue;
            }
            
            if include.is_match(relative_path) {
                if let Some(s) = relative_path.to_str() {
                    let mut r = results.lock().unwrap();
                    r.push(s.to_string());
                }
            }
            ignore::WalkState::Continue
        })
    });

    Ok(results.into_inner().unwrap())
}

#[napi]
pub fn glob_sync(
    patterns: Either<String, Vec<String>>,
    options: Option<GlobOptions>,
) -> Result<Vec<String>> {
    let pattern_list = match patterns {
        Either::A(s) => vec![s],
        Either::B(v) => v,
    };

    let cwd = match options.as_ref().and_then(|o| o.cwd.as_ref()) {
        Some(cwd) => PathBuf::from(cwd),
        None => std::env::current_dir().map_err(|e| {
            Error::new(Status::GenericFailure, format!("Failed to get current directory: {}", e))
        })?,
    };

    let exclude_list = options.and_then(|o| o.exclude).unwrap_or_default();

    let include_globset = build_globset(&pattern_list)?;
    let exclude_globset = build_globset(&exclude_list)?;

    walk_and_filter(&cwd, &include_globset, &exclude_globset)
}

#[napi]
pub async fn glob(
    patterns: Either<String, Vec<String>>,
    options: Option<GlobOptions>,
) -> Result<Vec<String>> {
    let pattern_list = match patterns {
        Either::A(s) => vec![s],
        Either::B(v) => v,
    };

    let cwd = match options.as_ref().and_then(|o| o.cwd.as_ref()) {
        Some(cwd) => PathBuf::from(cwd),
        None => std::env::current_dir().map_err(|e| {
            Error::new(Status::GenericFailure, format!("Failed to get current directory: {}", e))
        })?,
    };

    let exclude_list = options.and_then(|o| o.exclude).unwrap_or_default();

    let include_globset = build_globset(&pattern_list)?;
    let exclude_globset = build_globset(&exclude_list)?;

    let result = tokio::task::spawn_blocking(move || {
        walk_and_filter(&cwd, &include_globset, &exclude_globset)
    })
    .await
    .map_err(|e| Error::new(Status::GenericFailure, format!("Join error: {}", e)))??;

    Ok(result)
}