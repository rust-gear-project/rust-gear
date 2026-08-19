use globset::{Candidate, Glob, GlobSet, GlobSetBuilder};
use ignore::Match;
use ignore::gitignore::{Gitignore, GitignoreBuilder};
use napi::bindgen_prelude::*;
use napi_derive::napi;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

#[allow(dead_code)]
enum WalkWidth {
    FastestCoreClass,
    Full,
}

#[cfg(target_os = "macos")]
const WALK_WIDTH: WalkWidth = WalkWidth::FastestCoreClass;

#[cfg(not(target_os = "macos"))]
const WALK_WIDTH: WalkWidth = WalkWidth::Full;

fn performance_cores() -> Option<usize> {
    platform::performance_cores()
}

#[cfg(target_os = "macos")]
mod platform {
    unsafe extern "C" {
        fn sysctlbyname(
            name: *const std::ffi::c_char,
            oldp: *mut std::ffi::c_void,
            oldlen: *mut usize,
            newp: *mut std::ffi::c_void,
            newlen: usize,
        ) -> i32;
    }

    fn sysctl_u32(name: &std::ffi::CStr) -> Option<u32> {
        let mut value: u32 = 0;
        let mut len = std::mem::size_of::<u32>();
        let ok = unsafe {
            sysctlbyname(
                name.as_ptr(),
                &mut value as *mut u32 as *mut std::ffi::c_void,
                &mut len,
                std::ptr::null_mut(),
                0,
            )
        };
        (ok == 0 && len == std::mem::size_of::<u32>()).then_some(value)
    }

    pub fn performance_cores() -> Option<usize> {
        let fastest = sysctl_u32(c"hw.perflevel0.logicalcpu").filter(|n| *n > 0)?;
        let total = sysctl_u32(c"hw.logicalcpu").unwrap_or(0);
        (total == 0 || fastest < total).then_some(fastest as usize)
    }
}

#[cfg(not(target_os = "macos"))]
mod platform {
    pub fn performance_cores() -> Option<usize> {
        None
    }
}

fn pool_threads() -> usize {
    let available = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);

    if let Some(requested) = std::env::var("RUST_GEAR_GLOB_THREADS")
        .ok()
        .and_then(|value| value.trim().parse::<usize>().ok())
        .filter(|n| *n > 0)
    {
        return requested;
    }

    match WALK_WIDTH {
        WalkWidth::Full => available,
        WalkWidth::FastestCoreClass => {
            performance_cores().map_or(available, |cores| cores.clamp(1, available))
        }
    }
}

fn pool() -> &'static rayon::ThreadPool {
    static POOL: OnceLock<rayon::ThreadPool> = OnceLock::new();
    POOL.get_or_init(|| {
        rayon::ThreadPoolBuilder::new()
            .num_threads(pool_threads())
            .thread_name(|i| format!("rust-gear-glob-{i}"))
            .build()
            .expect("failed to build glob thread pool")
    })
}

#[napi(object)]
#[derive(Clone)]
pub struct GlobOptions {
    pub exclude: Option<Vec<String>>,
    pub cwd: Option<String>,
    pub dot: Option<bool>,
    pub sort: Option<bool>,
    pub gitignore: Option<bool>,
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
    respect_gitignore: bool,
    results: Mutex<Vec<String>>,
}

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

fn walk_dir<'a>(
    ctx: &'a WalkContext,
    dir: PathBuf,
    mut rel: PathBuf,
    parent_gitignore: Option<Arc<GitignoreNode>>,
    mut in_git_repo: bool,
    scope: &rayon::Scope<'a>,
) {
    let read_dir = match std::fs::read_dir(&dir) {
        Ok(rd) => rd,
        Err(_) => return,
    };

    let mut entries: Vec<(std::ffi::OsString, std::fs::FileType)> = Vec::new();
    let mut has_gitignore = false;

    for entry in read_dir.flatten() {
        let file_type = match entry.file_type() {
            Ok(ft) => ft,
            Err(_) => continue,
        };
        let name = entry.file_name();
        let bytes = name.as_encoded_bytes();
        if bytes.starts_with(b".") {
            if ctx.respect_gitignore {
                if bytes == b".gitignore" {
                    has_gitignore = true;
                } else if bytes == b".git" || bytes == b".jj" {
                    in_git_repo = true;
                }
            }
            if ctx.hide_dot_file {
                continue;
            }
        }
        entries.push((name, file_type));
    }

    let gitignore = if has_gitignore {
        let mut builder = GitignoreBuilder::new(&rel);
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

    for (name, file_type) in entries {
        let is_dir = file_type.is_dir();
        rel.push(&name);

        if in_git_repo && is_gitignored(gitignore.as_ref(), &rel, is_dir) {
            rel.pop();
            continue;
        }

        let abs = ctx.has_absolute_pattern.then(|| dir.join(&name));

        if is_dir {
            if !ctx.exclude.is_empty()
                && (ctx.exclude.is_match_candidate(&Candidate::new(&rel))
                    || abs
                        .as_ref()
                        .is_some_and(|p| ctx.exclude.is_match_candidate(&Candidate::new(p))))
            {
                rel.pop();
                continue;
            }
            let child_dir = abs.unwrap_or_else(|| dir.join(&name));
            let child_rel = rel.clone();
            rel.pop();
            let gitignore = gitignore.clone();
            scope.spawn(move |s| walk_dir(ctx, child_dir, child_rel, gitignore, in_git_repo, s));
            continue;
        }

        let rel_candidate = Candidate::new(&rel);
        let is_match = if let Some(abs) = &abs {
            let abs_candidate = Candidate::new(abs);
            (ctx.include.is_match_candidate(&rel_candidate)
                || ctx.include.is_match_candidate(&abs_candidate))
                && !(ctx.exclude.is_match_candidate(&rel_candidate)
                    || ctx.exclude.is_match_candidate(&abs_candidate))
        } else {
            ctx.include.is_match_candidate(&rel_candidate)
                && !ctx.exclude.is_match_candidate(&rel_candidate)
        };

        if !is_match {
            rel.pop();
            continue;
        }

        let s = match &abs {
            Some(abs) => abs.to_string_lossy().into_owned(),
            None => rel.to_string_lossy().into_owned(),
        };
        rel.pop();

        // Always return forward slashes, even on Windows
        #[cfg(windows)]
        let s = s.replace('\\', "/");

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

fn walk_and_filter(search_root: &Path, ctx: WalkContext, sort: bool) -> Result<Vec<String>> {
    // The walk root itself can be pruned by an exclude pattern.
    if !ctx.exclude.is_empty() {
        let root_relative = search_root.strip_prefix(&ctx.cwd).unwrap_or(search_root);
        if ctx
            .exclude
            .is_match_candidate(&Candidate::new(root_relative))
            || (ctx.has_absolute_pattern
                && ctx.exclude.is_match_candidate(&Candidate::new(search_root)))
        {
            return Ok(Vec::new());
        }
    }

    let root_rel = search_root
        .strip_prefix(&ctx.cwd)
        .unwrap_or(Path::new(""))
        .to_path_buf();

    pool().install(|| {
        rayon::scope(|s| walk_dir(&ctx, search_root.to_path_buf(), root_rel, None, false, s))
    });

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
        gitignore: None,
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
    let sort = options.sort.unwrap_or(false);

    let ctx = WalkContext {
        cwd,
        include,
        exclude,
        has_absolute_pattern: has_absolute,
        hide_dot_file: !options.dot.unwrap_or(false),
        respect_gitignore: options.gitignore.unwrap_or(true),
        results: Mutex::new(Vec::new()),
    };

    walk_and_filter(&search_root, ctx, sort)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_detected_topology() {
        eprintln!(
            "target_os={} available_parallelism={:?} performance_cores={:?} pool_threads={}",
            std::env::consts::OS,
            std::thread::available_parallelism().map(|n| n.get()),
            performance_cores(),
            pool_threads(),
        );
    }

    #[test]
    fn pool_threads_stays_within_available_parallelism() {
        unsafe { std::env::remove_var("RUST_GEAR_GLOB_THREADS") };

        let available = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1);
        let threads = pool_threads();

        assert!(threads >= 1, "pool must have at least one thread");
        assert!(
            threads <= available,
            "pool_threads() = {threads} exceeds available_parallelism() = {available}"
        );
    }
}
