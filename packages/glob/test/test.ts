const path = require("path");
const test = require("node:test");
const assert = require("node:assert");
const { globSync, glob } = require("../index");

test("async glob", async () => {
  const files = await glob(path.join(process.cwd()), "src/**/*.rs");
  console.log(files);
  assert(Array.isArray(files));
});

test("sync glob", () => {
  const files = globSync("src/**/*.rs");
  console.log(files);
  assert(Array.isArray(files));
});

test("glob with exclude", () => {
  const files = globSync("**/*.rs", {
    exclude: ["**/test/**", "**/target/**"],
  });
  console.log(files);
  assert(Array.isArray(files));
});

test("glob with cwd", () => {
  const files = globSync("**/*.rs", {
    cwd: "test/test_exclude",
  });
  console.log(files);
  assert(Array.isArray(files));
});
