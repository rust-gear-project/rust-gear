use std::path::{Path, PathBuf};
use napi::bindgen_prelude::*;
use napi_derive::napi;
use globset::{Glob, GlobSet, GlobSetBuilder};
use ignore::WalkBuilder;

#[napi(object)]
#[derive(Clone)]
pub struct GlobOptions {
    pub exclude: Option<Vec<String>>,
    pub cwd: Option<String>,
}

fn resolve_cwd(cwd: &Option<String>) -> Result<PathBuf> {
    if let Some(cwd_str) = cwd {
        let path = PathBuf::from(cwd_str);
        if path.is_absolute() {
            Ok(path)
        } else {
            let current_dir = std::env::current_dir()
                .map_err(|e| Error::new(Status::GenericFailure, format!("Failed to get current directory: {}", e)))?;
            Ok(current_dir.join(path))
        }
    } else {
        std::env::current_dir()
            .map_err(|e| Error::new(Status::GenericFailure, format!("Failed to get current directory: {}", e)))
    }
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

fn is_absolute_pattern(pattern: &str) -> bool {
    Path::new(pattern).is_absolute()
}

fn walk_and_filter(
    cwd: &Path,
    include: &GlobSet,
    exclude: &GlobSet,
    patterns: &[String],
) -> Result<Vec<String>> {
    let has_absolute_pattern = patterns.iter().any(|pat| is_absolute_pattern(pat));
    let walker = WalkBuilder::new(cwd)
        .standard_filters(true)
        .build_parallel();

    let results = std::sync::Mutex::new(Vec::new());

    walker.run(|| {
        let results = &results;
        let cwd = cwd.to_path_buf();
        Box::new(move |result| {
            let entry = match result {
                Ok(e) => e,
                Err(_) => return ignore::WalkState::Continue,
            };

            let path = entry.path();
            let relative_path = path.strip_prefix(&cwd).unwrap_or(path);

            let is_included = include.is_match(relative_path) || include.is_match(path);
            let is_excluded = exclude.is_match(relative_path) || exclude.is_match(path);

            if !is_included || is_excluded {
                return ignore::WalkState::Continue;
            }

            // If the pattern is an absolute path, return the absolute path
            // If the pattern is a relative path, return the path relative to the cwd
            let s = if has_absolute_pattern {
                path.to_string_lossy().to_string()
            } else {
                relative_path.to_string_lossy().to_string()
            };

            let mut r = results.lock().unwrap();
            r.push(s);

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
    let options = options.unwrap_or_else(|| GlobOptions { exclude: None, cwd: None });
    let pattern_list = match patterns {
        Either::A(s) => vec![s],
        Either::B(v) => v,
    };

    let cwd = resolve_cwd(&options.cwd)?;
    let exclude_list = options.exclude.unwrap_or_default();

    let include_globset = build_globset(&pattern_list)?;
    let exclude_globset = build_globset(&exclude_list)?;

    walk_and_filter(&cwd, &include_globset, &exclude_globset, &pattern_list)
}

#[napi]
pub async fn glob(
    patterns: Either<String, Vec<String>>,
    options: Option<GlobOptions>,
) -> Result<Vec<String>> {
    let options = options.unwrap_or_else(|| GlobOptions { exclude: None, cwd: None });
    let pattern_list = match patterns {
        Either::A(s) => vec![s],
        Either::B(v) => v,
    };

    let cwd = resolve_cwd(&options.cwd)?;
    let exclude_list = options.exclude.unwrap_or_default();

    let include_globset = build_globset(&pattern_list)?;
    let exclude_globset = build_globset(&exclude_list)?;

    let result = tokio::task::spawn_blocking(move || {
        walk_and_filter(&cwd, &include_globset, &exclude_globset, &pattern_list)
    })
    .await
    .map_err(|e| Error::new(Status::GenericFailure, format!("Join error: {}", e)))??;

    Ok(result)
}
