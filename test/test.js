const test = require("node:test");
const assert = require("node:assert");
const { globSync, glob } = require("../index.js");

test("async glob", async (t) => {
  const files = await glob("src/**/*.rs");
  console.log(files);
  assert(Array.isArray(files));
});

test("sync glob", (t) => {
  const files = globSync("src/**/*.rs");
  console.log(files);
  assert(Array.isArray(files));
});

test("glob with exclude", (t) => {
  const files = globSync("**/*.rs", {
    exclude: ["**/test/**", "**/target/**"],
  });
  console.log(files);
  assert(Array.isArray(files));
});

test("glob with cwd", (t) => {
  const files = globSync("**/*.rs", {
    cwd: "test/test_exclude",
  });
  console.log(files);
  assert(Array.isArray(files));
});
