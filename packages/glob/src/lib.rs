use globset::{Candidate, Glob, GlobSet, GlobSetBuilder};
use ignore::Match;
use ignore::gitignore::{Gitignore, GitignoreBuilder};
use napi::bindgen_prelude::*;
use napi_derive::napi;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

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
    hide_dot_file: bool,
    results: Mutex<Vec<String>>,
}

/// One `.gitignore` matcher; nodes chain up to matchers from ancestor
/// directories so a deeper file's rules take precedence.
struct GitignoreNode {
    matcher: Gitignore,
    parent: Option<Arc<GitignoreNode>>,
}

fn is_gitignored(mut node: Option<&Arc<GitignoreNode>>, path: &Path, is_dir: bool) -> bool {
    while let Some(n) = node {
        match n.matcher.matched(path, is_dir) {
            Match::None => node = n.parent.as_ref(),
            Match::Ignore(_) => return true,
            Match::Whitelist(_) => return false,
        }
    }
    false
}

/// Recursively walk `dir`, spawning subdirectories onto rayon's global pool.
/// Gitignore rules only apply once a `.git` marker has been seen at or below
/// the walk root (`in_git_repo`), mirroring the ignore crate's require_git.
fn walk_dir<'a>(
    ctx: &'a WalkContext,
    dir: PathBuf,
    parent_gitignore: Option<Arc<GitignoreNode>>,
    mut in_git_repo: bool,
    scope: &rayon::Scope<'a>,
) {
    let read_dir = match std::fs::read_dir(&dir) {
        Ok(rd) => rd,
        Err(_) => return,
    };

    let mut entries: Vec<(std::fs::DirEntry, std::fs::FileType)> = Vec::new();
    let mut has_gitignore = false;

    for entry in read_dir.flatten() {
        let file_type = match entry.file_type() {
            Ok(ft) => ft,
            Err(_) => continue,
        };
        let name = entry.file_name();
        let bytes = name.as_encoded_bytes();
        if bytes.starts_with(b".") {
            if bytes == b".gitignore" {
                has_gitignore = true;
            } else if bytes == b".git" || bytes == b".jj" {
                in_git_repo = true;
            }
            if ctx.hide_dot_file {
                continue;
            }
        }
        entries.push((entry, file_type));
    }

    let gitignore = if has_gitignore {
        let mut builder = GitignoreBuilder::new(&dir);
        builder.add(dir.join(".gitignore"));
        match builder.build() {
            Ok(matcher) => Some(Arc::new(GitignoreNode {
                matcher,
                parent: parent_gitignore,
            })),
            Err(_) => parent_gitignore,
        }
    } else {
        parent_gitignore
    };

    let mut matched: Vec<String> = Vec::new();

    for (entry, file_type) in entries {
        let path = entry.path();
        let is_dir = file_type.is_dir();

        if in_git_repo && is_gitignored(gitignore.as_ref(), &path, is_dir) {
            continue;
        }

        let relative_path = path.strip_prefix(&ctx.cwd).unwrap_or(&path);
        let rel_candidate = Candidate::new(relative_path);

        if is_dir {
            if !ctx.exclude.is_empty()
                && (ctx.exclude.is_match_candidate(&rel_candidate)
                    || (ctx.has_absolute_pattern
                        && ctx.exclude.is_match_candidate(&Candidate::new(&path))))
            {
                continue;
            }
            let gitignore = gitignore.clone();
            scope.spawn(move |s| walk_dir(ctx, path, gitignore, in_git_repo, s));
            continue;
        }

        let is_match = if ctx.has_absolute_pattern {
            let abs_candidate = Candidate::new(&path);
            (ctx.include.is_match_candidate(&rel_candidate)
                || ctx.include.is_match_candidate(&abs_candidate))
                && !(ctx.exclude.is_match_candidate(&rel_candidate)
                    || ctx.exclude.is_match_candidate(&abs_candidate))
        } else {
            ctx.include.is_match_candidate(&rel_candidate)
                && !ctx.exclude.is_match_candidate(&rel_candidate)
        };

        if !is_match {
            continue;
        }

        let s = if ctx.has_absolute_pattern {
            path.to_string_lossy().into_owned()
        } else {
            relative_path.to_string_lossy().into_owned()
        };

        if !s.is_empty() {
            matched.push(s);
        }
    }

    if !matched.is_empty() {
        let mut shared = ctx
            .results
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        shared.append(&mut matched);
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
        hide_dot_file,
        results: Mutex::new(Vec::new()),
    };

    // The walk root itself can be pruned by an exclude pattern.
    if !ctx.exclude.is_empty() {
        let root_relative = search_root.strip_prefix(cwd).unwrap_or(search_root);
        if ctx.exclude.is_match_candidate(&Candidate::new(root_relative))
            || (has_absolute_pattern && ctx.exclude.is_match_candidate(&Candidate::new(search_root)))
        {
            return Ok(Vec::new());
        }
    }

    rayon::scope(|s| walk_dir(&ctx, search_root.to_path_buf(), None, false, s));

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

    if pattern_list.iter().all(|p| p.is_empty()) {
        return Ok(Vec::new());
    }

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
