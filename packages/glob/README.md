# @rust-gear/glob

A fast glob alternative for Node.js powered by Rust.

## Performance

Measured against `fast-glob` and `glob` on a synthetic tree of ~50k files
(Apple Silicon, `node bench/bench.mjs`):

- **~2x faster** than the synchronous APIs of `fast-glob` and `glob`.
  The directory walk is parallelized on a Rust thread pool, so `globSync`
  runs at full speed without blocking on single-threaded I/O.
- **Matches or slightly beats** their async APIs on large trees
  (~36 ms vs ~39–40 ms for 10k matches), with ~2 ms calls on narrow
  patterns and ~0.04 ms fixed overhead per call.

## Installation

```sh
npm install --save-dev @rust-gear/glob
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

| Option    | Type     | Default         | Description                                            |
| :-------- | :------- | :-------------- | :----------------------------------------------------- |
| cwd       | string   | `process.cwd()` | Current working directory for searching                |
| exclude   | string[] | `[]`            | Glob patterns to exclude                               |
| dot       | boolean  | `false`         | Include dot files and directories                      |
| sort      | boolean  | `false`         | Return sorted results                                  |
| gitignore | boolean  | `true`          | Respect `.gitignore` files inside git (`.git`) repos   |

> **Note:** Set `gitignore: false` for results that depend only on the
> patterns and the filesystem, matching the behavior of `fast-glob`,
> which never reads `.gitignore`.

## License

Apache-2.0
