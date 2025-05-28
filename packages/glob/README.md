# rust-glob

A high-performance globbing library for Node.js, powered by native Rust code.

## Installation

```sh
npm install --save-dev @rust-gear/glob
```

## Usage

```js
import { globSync, glob } from "@rust-gear/glob";

const files = globSync("src/**/*.rs", { cwd: process.cwd() });
console.log(files);

const filesAsync = await glob("src/**/*.rs", { cwd: process.cwd() });
console.log(filesAsync);
```

## Options

You must pass an options object as the second argument to `globSync`/`glob`.  
The `cwd` option is **required**.

| Option  | Type     | Description                                            |
| :------ | :------- | :----------------------------------------------------- |
| cwd     | string   | **(Required)** Current working directory for searching |
| exclude | string[] | Array of glob patterns to exclude                      |

### Example

```js
const files = globSync("**/*.rs", {
  cwd: "src",
  exclude: ["**/test/**", "**/target/**"],
});
```

## API

**globSync(pattern: string \| string[], options: GlobOptions): string[]**  
Synchronously returns an array of file paths.

**glob(pattern: string \| string[], options: GlobOptions): Promise<string[]>**  
Asynchronously returns an array of file paths.

## License

Apache-2.0
