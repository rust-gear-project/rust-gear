# @rust-gear/glob

A fast glob alternative for Node.js powered by Rust.

## Performance

- **3–10x faster** than traditional JavaScript glob implementations.

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

| Option  | Type     | Default         | Description                             |
| :------ | :------- | :-------------- | :-------------------------------------- |
| cwd     | string   | `process.cwd()` | Current working directory for searching |
| exclude | string[] | `[]`            | Glob patterns to exclude                |
| dot     | boolean  | `false`         | Include dot files and directories       |
| sort    | boolean  | `false`         | Return sorted results                   |

## License

Apache-2.0
