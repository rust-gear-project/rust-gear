import test from "ava";
import path from "path";
import fs from "fs";
import os from "os";
import { globSync, glob } from "../index.js";

function makeTree(files: Record<string, string>): string {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "glob-test-"));
  for (const [rel, content] of Object.entries(files)) {
    const abs = path.join(root, rel);
    fs.mkdirSync(path.dirname(abs), { recursive: true });
    fs.writeFileSync(abs, content);
  }
  return root;
}

test("async glob", async (t) => {
  const files = await glob(path.join(process.cwd(), "src/**/*.rs"));
  t.true(Array.isArray(files));
});

test("sync glob", (t) => {
  const files = globSync("src/**/*.rs");
  t.true(Array.isArray(files));
});

test("glob with exclude", (t) => {
  const files = globSync("**/*.rs", {
    exclude: ["**/test/**", "**/target/**"],
  });
  t.true(Array.isArray(files));
});

test("glob with cwd", (t) => {
  const files = globSync("**/*.rs", {
    cwd: "test/test_exclude",
  });
  t.true(Array.isArray(files));
});

// オプション: 空のパターンを渡した場合のテスト
test("glob with empty pattern returns empty array", async (t) => {
  const files = await glob("");
  t.deepEqual(
    files,
    [],
    "Glob with empty pattern should return an empty array"
  );
});

test("globSync with empty pattern returns empty array", (t) => {
  const files = globSync("");
  t.deepEqual(
    files,
    [],
    "GlobSync with empty pattern should return an empty array"
  );
});

test("gitignore is respected by default inside a git repo", (t) => {
  const root = makeTree({
    "keep.js": "",
    "ignored.js": "",
    "sub/also-ignored.js": "",
    "sub/kept.js": "",
    ".gitignore": "ignored.js\nalso-ignored.js\n",
  });
  fs.mkdirSync(path.join(root, ".git"));
  const files = globSync("**/*.js", { cwd: root, sort: true });
  t.deepEqual(files, ["keep.js", "sub/kept.js"]);
});

test("gitignore has no effect outside a git repo", (t) => {
  const root = makeTree({
    "keep.js": "",
    "ignored.js": "",
    ".gitignore": "ignored.js\n",
  });
  const files = globSync("**/*.js", { cwd: root, sort: true });
  t.deepEqual(files, ["ignored.js", "keep.js"]);
});

test("nested gitignore overrides parent with re-include", (t) => {
  const root = makeTree({
    "top.log": "",
    "sub/nested.log": "",
    "sub/other.js": "",
    ".gitignore": "*.log\n",
    "sub/.gitignore": "!nested.log\n",
  });
  fs.mkdirSync(path.join(root, ".git"));
  const files = globSync("**/*", { cwd: root, sort: true });
  t.deepEqual(files, ["sub/nested.log", "sub/other.js"]);
});

test("gitignore: false includes ignored files even in a git repo", (t) => {
  const root = makeTree({
    "keep.js": "",
    "ignored.js": "",
    ".gitignore": "ignored.js\n",
  });
  fs.mkdirSync(path.join(root, ".git"));
  const files = globSync("**/*.js", { cwd: root, sort: true, gitignore: false });
  t.deepEqual(files, ["ignored.js", "keep.js"]);
});

test("gitignored directory is not descended", (t) => {
  const root = makeTree({
    "src/a.js": "",
    "node_modules/pkg/b.js": "",
    ".gitignore": "node_modules/\n",
  });
  fs.mkdirSync(path.join(root, ".git"));
  const files = globSync("**/*.js", { cwd: root, sort: true });
  t.deepEqual(files, ["src/a.js"]);
});
