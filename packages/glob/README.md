# @rust-gear/glob

A high-performance [napi-rs](https://napi.rs/) glob library powered by Rust's [globset](https://docs.rs/globset).

## ⚡ Performance

Faster than Node.js built-in fs.globSync:

- **3-9x faster** for typical glob patterns

## Installation

```sh
npm install --save-dev @rust-gear/glob
```

## Usage

```js
import * as rs from "@rust-gear/glob";

const files = rs.globSync("src/**/*.rs");

const filesAsync = await rs.glob("src/**/*.rs");

const files = rs.globSync("**/*.rs", {
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
