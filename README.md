# rust-glob

A high-performance globbing library for Node.js, powered by native Rust code.

## installation

```sh
npm install --save-dev rust-glob
```

## Usage

```js
import { globSync, glob } from "rust-glob";

const files = globSync("src/**/*.rs");
console.log(files);

const filesAsync = await glob("src/**/*.rs");
console.log(filesAsync);
```

## Options

You can pass an options object as the second argument to `globSync`/`glob`.

| Option  | Type     | Description                             |
| :------ | :------- | :-------------------------------------- |
| cwd     | string   | Current working directory for searching |
| exclude | string[] | Array of glob patterns to exclude       |

### Example

```js
const files = globSync("**/*.rs", {
  cwd: "src",
  exclude: ["**/test/**", "**/target/**"],
});
```

## API

**globSync(pattern: string | string[], options?: GlobOptions): string[]**  
Synchronously returns an array of file paths.

**glob(pattern: string | string[], options?: GlobOptions): Promise<string[]>**  
Asynchronously returns an array of file paths.

## License

Apache-2.0
