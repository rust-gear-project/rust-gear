# @rust-gear/glob

A high-performance napi-rs glob library powered by Rust's [globset](https://docs.rs/globset).

## ⚡ Performance

Significantly faster than Node.js built-in `fs.globSync`:

- **2-3x faster** for typical glob patterns
- **8x faster** for large file sets (5000+ files)

Based on benchmarks with 4,000-12,000 files across various patterns.

## Installation

```sh
npm install --save-dev @rust-gear/glob
```

## Usage

```js
import { globSync, glob } from "@rust-gear/glob";

const files = globSync("src/**/*.rs");

const filesAsync = await glob("src/**/*.rs");

const files = globSync("**/*.rs", {
  cwd: "src",
  exclude: ["**/test/**", "**/target/**"],
});
```

> **Return path type:**  
> Absolute patterns return absolute paths.  
> Relative patterns return paths relative to the specified `cwd`.

## Options

| Option  | Type     | Description                             |
| :------ | :------- | :-------------------------------------- |
| cwd     | string   | Current working directory for searching |
| exclude | string[] | Array of glob patterns to exclude       |

## License

Apache-2.0
