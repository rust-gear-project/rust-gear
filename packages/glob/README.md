# rust-glob

A high-performance globbing library for Node.js, powered by native Rust code.

## Installation

```sh
npm install --save-dev @rust-gear/glob
```

## Usage

```js
import { globSync, glob } from "@rust-gear/glob";

const files = globSync("src/**/*.rs");
console.log(files);

const filesAsync = await glob("src/**/*.rs");
console.log(filesAsync);
```

## Options

You can pass an options object as the second argument to `globSync/glob`.

> **Return path type:**  
> If the pattern is absolute, absolute paths are returned.  
> If the pattern is relative, paths are returned relative to the specified `cwd`.

| Option  | Type     | Description                                            |
| :------ | :------- | :----------------------------------------------------- |
| cwd     | string   | **(Required)** Current working directory for searching |
| exclude | string[] | Array of glob patterns to exclude                      |

### Example

```js
// Relative pattern, returns paths relative to cwd
const files = globSync("**/*.rs", {
  cwd: "src",
  exclude: ["**/test/**", "**/target/**"],
});

// Convert all relative paths to absolute paths
const absFiles = globSync("**/*.rs", {
  cwd: "src",
  exclude: ["**/test/**", "**/target/**"],
}).map((f) => path.resolve(cwd(), f));

// Absolute pattern, returns absolute paths
const absoluteFiles = globSync("/Users/foo/project/src/**/*.rs");
```

## API

**globSync(pattern: string \| string[], options?: GlobOptions): string[]**  
Synchronously returns an array of file paths.

**glob(pattern: string \| string[], options?: GlobOptions): Promise<string[]>**  
Asynchronously returns an array of file paths.

## License

Apache-2.0
