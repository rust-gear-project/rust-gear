use flume::unbounded;
use globset::{Glob, GlobSet, GlobSetBuilder};
use ignore::WalkBuilder;
use napi::bindgen_prelude::*;
use napi_derive::napi;
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[napi(object)]
#[derive(Clone)]
pub struct GlobOptions {
    pub exclude: Option<Vec<String>>,
    pub cwd: Option<String>,
    pub dot: Option<bool>,
    pub sort: Option<bool>,
}

fn resolve_cwd(cwd: &Option<String>) -> Result<PathBuf> {
    if let Some(cwd_str) = cwd {
        let path = PathBuf::from(cwd_str);
        if path.is_absolute() {
            Ok(path)
        } else {
            let current_dir = std::env::current_dir()
                .map_err(|e| Error::new(Status::GenericFailure, e.to_string()))?;
            Ok(current_dir.join(path))
        }
    } else {
        std::env::current_dir().map_err(|e| Error::new(Status::GenericFailure, e.to_string()))
    }
}

fn build_globset(patterns: &[String]) -> Result<GlobSet> {
    let mut builder = GlobSetBuilder::new();
    for pat in patterns {
        builder.add(Glob::new(pat).map_err(|e| {
            Error::new(
                Status::InvalidArg,
                format!("Invalid glob pattern '{}': {}", pat, e),
            )
        })?);
    }
    builder
        .build()
        .map_err(|e| Error::new(Status::InvalidArg, e.to_string()))
}

/// Extract the common static base directory from a set of patterns
fn determine_base_path(cwd: &Path, patterns: &[String]) -> PathBuf {
    if patterns.is_empty() {
        return cwd.to_path_buf();
    }

    let mut common_base: Option<PathBuf> = None;
    let glob_chars = ['*', '?', '[', '{'];

    for pattern in patterns {
        // If an absolute path is included, scan from cwd (safety measure)
        if Path::new(pattern).is_absolute() {
            return cwd.to_path_buf();
        }

        let static_part = match pattern.find(|c| glob_chars.contains(&c)) {
            Some(idx) => {
                let prefix = &pattern[..idx];
                if let Some(last_sep) = prefix.rfind(|c| c == '/' || c == '\\') {
                    &prefix[..last_sep]
                } else {
                    ""
                }
            }
            None => {
                if let Some(last_sep) = pattern.rfind(|c| c == '/' || c == '\\') {
                    &pattern[..last_sep]
                } else {
                    ""
                }
            }
        };

        if static_part.is_empty() {
            return cwd.to_path_buf();
        }

        let path = PathBuf::from(static_part);
        match common_base {
            None => common_base = Some(path),
            Some(ref mut base) => {
                let mut new_base = PathBuf::new();
                for (c1, c2) in base.components().zip(path.components()) {
                    if c1 == c2 {
                        new_base.push(c1);
                    } else {
                        break;
                    }
                }
                *base = new_base;
            }
        }
    }

    match common_base {
        Some(base) if !base.as_os_str().is_empty() => {
            let full_path = cwd.join(base);
            if full_path.is_dir() {
                full_path
            } else {
                cwd.to_path_buf()
            }
        }
        _ => cwd.to_path_buf(),
    }
}

fn walk_and_filter(
    search_root: &Path,
    cwd: &Path,
    include: Arc<GlobSet>,
    exclude: Arc<GlobSet>,
    has_absolute_pattern: bool,
    hide_dot_file: bool,
    sort: bool,
) -> Result<Vec<String>> {
    let (tx, rx) = unbounded::<String>();

    let cwd_owned = cwd.to_path_buf();
    let has_relative_only = !has_absolute_pattern;

    let handle = std::thread::spawn(move || rx.into_iter().collect::<Vec<_>>());

    WalkBuilder::new(search_root)
        .standard_filters(false)
        .hidden(hide_dot_file)
        .git_ignore(true)
        .build_parallel()
        .run(|| {
            let tx = tx.clone();
            let include = Arc::clone(&include);
            let exclude = Arc::clone(&exclude);
            let cwd = cwd_owned.clone();

            Box::new(move |result| {
                let entry = match result {
                    Ok(e) => e,
                    Err(_) => return ignore::WalkState::Continue,
                };

                let path = entry.path();
                let relative_path = path.strip_prefix(&cwd).unwrap_or(path);

                if entry.file_type().map_or(true, |ft| ft.is_dir()) {
                    if !exclude.is_empty()
                        && (exclude.is_match(relative_path)
                            || (!has_relative_only && exclude.is_match(path)))
                    {
                        return ignore::WalkState::Skip;
                    }
                    return ignore::WalkState::Continue;
                }

                let is_included = if has_relative_only {
                    include.is_match(relative_path)
                } else {
                    include.is_match(relative_path) || include.is_match(path)
                };

                if !is_included {
                    return ignore::WalkState::Continue;
                }

                let is_excluded = if has_relative_only {
                    exclude.is_match(relative_path)
                } else {
                    exclude.is_match(relative_path) || exclude.is_match(path)
                };

                if is_excluded {
                    return ignore::WalkState::Continue;
                }

                let s = if has_absolute_pattern {
                    path.to_string_lossy().into_owned()
                } else {
                    relative_path.to_string_lossy().into_owned()
                };

                if !s.is_empty() {
                    let _ = tx.send(s);
                }

                ignore::WalkState::Continue
            })
        });

    drop(tx);

    let mut result = handle.join().map_err(|_| {
        Error::new(
            Status::GenericFailure,
            "failed to join result collector thread",
        )
    })?;

    if sort {
        result.sort();
    }

    Ok(result)
}

fn core(
    patterns: Either<String, Vec<String>>,
    options: Option<GlobOptions>,
) -> Result<Vec<String>> {
    let options = options.unwrap_or(GlobOptions {
        exclude: None,
        cwd: None,
        dot: None,
        sort: None,
    });
    let pattern_list = match patterns {
        Either::A(s) => vec![s],
        Either::B(v) => v,
    };

    let pattern_list: Vec<String> = pattern_list
        .into_iter()
        .map(|p| {
            let normalized = p.replace("\\", "/");
            normalized
                .strip_prefix("./")
                .unwrap_or(&normalized)
                .to_string()
        })
        .collect();

    let cwd = resolve_cwd(&options.cwd)?;
    let search_root = determine_base_path(&cwd, &pattern_list);
    let has_absolute = pattern_list.iter().any(|p| Path::new(p).is_absolute());

    let include = Arc::new(build_globset(&pattern_list)?);
    let exclude = Arc::new(build_globset(&options.exclude.unwrap_or_default())?);
    let hide_dot_file = !options.dot.unwrap_or(false);
    let sort = options.sort.unwrap_or(false);

    walk_and_filter(
        &search_root,
        &cwd,
        include,
        exclude,
        has_absolute,
        hide_dot_file,
        sort,
    )
}

#[napi]
pub fn glob_sync(
    patterns: Either<String, Vec<String>>,
    options: Option<GlobOptions>,
) -> Result<Vec<String>> {
    core(patterns, options)
}

#[napi]
pub async fn glob(
    patterns: Either<String, Vec<String>>,
    options: Option<GlobOptions>,
) -> Result<Vec<String>> {
    tokio::task::spawn_blocking(move || core(patterns, options))
        .await
        .map_err(|e| Error::new(Status::GenericFailure, e.to_string()))?
}
