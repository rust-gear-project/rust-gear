# @rust-gear/glob

A fast glob alternative for Node.js powered by Rust.

## Performance

Measured against `fast-glob` on a synthetic tree of ~50k files
(Apple Silicon, `node bench/bench.mjs`, medians):

| pattern (~50k file tree) | `globSync` | `fast-glob` sync | `fast-glob` async |
| :----------------------- | ---------: | ---------------: | ----------------: |
| `**/*.js` (10k matches)  |      25 ms |            77 ms |             42 ms |
| `**/*.{js,ts}` (18k)     |      26 ms |            80 ms |             42 ms |
| `**/*` + exclude (37.8k) |      29 ms |            82 ms |             43 ms |
| `mod1/**/*.rs` (400)     |     1.3 ms |           3.7 ms |            1.9 ms |

- **~3x faster** than the synchronous APIs of `fast-glob` and `glob`.
  The directory walk is parallelized on a Rust thread pool, so `globSync`
  runs at full speed without blocking on single-threaded I/O.
- **~1.6x faster** than their async APIs, with ~0.04 ms fixed overhead
  per call.

### Thread pool size

How wide the walk runs is a property of the _filesystem_, not of the CPU, so
it is decided per platform.

On **Windows and Linux** the pool is every logical CPU the process may use.
NTFS takes a per-directory lock and Linux takes `inode->i_rwsem` per inode, so
enumerating different directories in parallel does not collide, and the walk
keeps scaling to the full width of the machine:

```
$ node bench/threads.mjs      # Windows 11, 16 logical CPUs, NTFS
threads   1    237.81 ms
threads   8     43.71 ms
threads  16     28.24 ms   <- fastest
threads  32     28.43 ms

$ node bench/threads.mjs      # Ubuntu, 8 logical CPUs, ext4
threads   1     37.08 ms
threads   4     11.57 ms
threads   8     10.75 ms
threads  16     10.79 ms
```

On **macOS** it is capped to the machine's fastest core class, read from
`hw.perflevel0`. APFS serializes enumeration on volume-wide state in the vnode
layer, so wall time bottoms out long before the CPU does — past the knee the
extra threads are queueing in the kernel, not working:

```
$ node bench/threads.mjs      # macOS, Apple Silicon 4P+4E, APFS
threads   1     64.13 ms
threads   4     24.16 ms   <- fastest
threads   8     35.16 ms
threads  16     34.76 ms
```

The knee tracks syscall _rate_ rather than core count, so on a Mac with many
more performance cores the optimum may sit below their count. On an Intel Mac,
where every core is one class, nothing is capped.

Set `RUST_GEAR_GLOB_THREADS` to override the choice on any platform; run
`node bench/threads.mjs` to see your own machine's curve. The count is always
clamped to the process's CPU allowance, so a cgroup quota or affinity mask
still wins.

## Installation

```sh
pnpm add -D @rust-gear/glob
```

## Usage

```js
import * as rs from "@rust-gear/glob";

const filesAsync = await rs.glob("src/**/*.rs");

const files = rs.globSync("**/*.rs", {
  cwd: "src",
  exclude: ["**/test/**", "**/target/**"],
});
```

> **Return paths:**  
> Absolute patterns → absolute paths  
> Relative patterns → paths relative to `cwd`

## Options

| Option    | Type     | Default         | Description                                          |
| :-------- | :------- | :-------------- | :--------------------------------------------------- |
| cwd       | string   | `process.cwd()` | Current working directory for searching              |
| exclude   | string[] | `[]`            | Glob patterns to exclude                             |
| dot       | boolean  | `false`         | Include dot files and directories                    |
| sort      | boolean  | `false`         | Return sorted results                                |
| gitignore | boolean  | `true`          | Respect `.gitignore` files inside git (`.git`) repos |

> **Note:** Set `gitignore: false` for results that depend only on the
> patterns and the filesystem, matching the behavior of `fast-glob`,
> which never reads `.gitignore`.

## License

Apache-2.0
