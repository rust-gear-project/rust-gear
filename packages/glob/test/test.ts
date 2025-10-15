import test from "ava";
import path from "path";
import { globSync, glob } from "../index.mjs";

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
