# @rust-gear/glob

A high-performance globbing library for Node.js, powered by native Rust code.

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

// Synchronous
const files = globSync("src/**/*.rs");
console.log(files);

// Asynchronous
const filesAsync = await glob("src/**/*.rs");
console.log(filesAsync);
```

## Options

Both `globSync` and `glob` accept an optional options object as the second argument.

> **Return path type:**  
> Absolute patterns return absolute paths.  
> Relative patterns return paths relative to the specified `cwd`.

| Option  | Type     | Description                             |
| :------ | :------- | :-------------------------------------- |
| cwd     | string   | Current working directory for searching |
| exclude | string[] | Array of glob patterns to exclude       |

### Examples

```js
// Relative pattern - returns paths relative to cwd
const files = globSync("**/*.rs", {
  cwd: "src",
  exclude: ["**/test/**", "**/target/**"],
});

// Convert relative paths to absolute
const absFiles = globSync("**/*.rs", {
  cwd: "src",
  exclude: ["**/test/**", "**/target/**"],
}).map((f) => path.resolve(cwd(), f));

// Absolute pattern - returns absolute paths
const absoluteFiles = globSync("/Users/foo/project/src/**/*.rs");
```

## API

### `globSync(pattern, options?)`

Synchronously returns an array of file paths matching the pattern.

**Parameters:**

- `pattern` - A glob pattern string or array of patterns
- `options?` - Optional configuration object

**Returns:** `string[]`

### `glob(pattern, options?)`

Asynchronously returns an array of file paths matching the pattern.

**Parameters:**

- `pattern` - A glob pattern string or array of patterns
- `options?` - Optional configuration object

**Returns:** `Promise<string[]>`

## License

Apache-2.0
