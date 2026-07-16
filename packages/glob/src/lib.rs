use globset::{Candidate, Glob, GlobSet, GlobSetBuilder};
use ignore::{DirEntry, WalkBuilder, WalkState};
use napi::bindgen_prelude::*;
use napi_derive::napi;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

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
                if let Some(last_sep) = prefix.rfind(['/', '\\']) {
                    &prefix[..last_sep]
                } else {
                    ""
                }
            }
            None => {
                if let Some(last_sep) = pattern.rfind(['/', '\\']) {
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

struct WalkContext {
    cwd: PathBuf,
    include: GlobSet,
    exclude: GlobSet,
    has_absolute_pattern: bool,
    results: Mutex<Vec<String>>,
}

/// Per-thread visitor: collects matches locally and merges them into the
/// shared vec once on drop, so worker threads never contend mid-walk.
struct MatchCollector<'a> {
    ctx: &'a WalkContext,
    local: Vec<String>,
}

impl ignore::ParallelVisitor for MatchCollector<'_> {
    fn visit(&mut self, result: std::result::Result<DirEntry, ignore::Error>) -> WalkState {
        let entry = match result {
            Ok(e) => e,
            Err(_) => return WalkState::Continue,
        };
        let ctx = self.ctx;

        let path = entry.path();
        let relative_path = path.strip_prefix(&ctx.cwd).unwrap_or(path);
        let rel_candidate = Candidate::new(relative_path);

        if entry.file_type().is_none_or(|ft| ft.is_dir()) {
            if !ctx.exclude.is_empty()
                && (ctx.exclude.is_match_candidate(&rel_candidate)
                    || (ctx.has_absolute_pattern
                        && ctx.exclude.is_match_candidate(&Candidate::new(path))))
            {
                return WalkState::Skip;
            }
            return WalkState::Continue;
        }

        let is_match = if ctx.has_absolute_pattern {
            let abs_candidate = Candidate::new(path);
            (ctx.include.is_match_candidate(&rel_candidate)
                || ctx.include.is_match_candidate(&abs_candidate))
                && !(ctx.exclude.is_match_candidate(&rel_candidate)
                    || ctx.exclude.is_match_candidate(&abs_candidate))
        } else {
            ctx.include.is_match_candidate(&rel_candidate)
                && !ctx.exclude.is_match_candidate(&rel_candidate)
        };

        if !is_match {
            return WalkState::Continue;
        }

        let s = if ctx.has_absolute_pattern {
            path.to_string_lossy().into_owned()
        } else {
            relative_path.to_string_lossy().into_owned()
        };

        if !s.is_empty() {
            self.local.push(s);
        }

        WalkState::Continue
    }
}

impl Drop for MatchCollector<'_> {
    fn drop(&mut self) {
        let mut shared = self
            .ctx
            .results
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        shared.append(&mut self.local);
    }
}

struct MatchCollectorBuilder<'a> {
    ctx: &'a WalkContext,
}

impl<'s> ignore::ParallelVisitorBuilder<'s> for MatchCollectorBuilder<'s> {
    fn build(&mut self) -> Box<dyn ignore::ParallelVisitor + 's> {
        Box::new(MatchCollector {
            ctx: self.ctx,
            local: Vec::new(),
        })
    }
}

fn walk_and_filter(
    search_root: &Path,
    cwd: &Path,
    include: GlobSet,
    exclude: GlobSet,
    has_absolute_pattern: bool,
    hide_dot_file: bool,
    sort: bool,
) -> Result<Vec<String>> {
    let ctx = WalkContext {
        cwd: cwd.to_path_buf(),
        include,
        exclude,
        has_absolute_pattern,
        results: Mutex::new(Vec::new()),
    };

    WalkBuilder::new(search_root)
        .standard_filters(false)
        .hidden(hide_dot_file)
        .git_ignore(true)
        .build_parallel()
        .visit(&mut MatchCollectorBuilder { ctx: &ctx });

    let mut result = ctx
        .results
        .into_inner()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    if sort {
        result.sort_unstable();
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
            let normalized = if p.contains('\\') {
                p.replace('\\', "/")
            } else {
                p
            };
            match normalized.strip_prefix("./") {
                Some(stripped) => stripped.to_string(),
                None => normalized,
            }
        })
        .collect();

    let cwd = resolve_cwd(&options.cwd)?;
    let search_root = determine_base_path(&cwd, &pattern_list);
    let has_absolute = pattern_list.iter().any(|p| Path::new(p).is_absolute());

    let include = build_globset(&pattern_list)?;
    let exclude = build_globset(&options.exclude.unwrap_or_default())?;
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
