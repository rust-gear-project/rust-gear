const fs = require("fs");
const path = require("path");

const packageRoot = process.cwd();
const packageJsonPath = path.join(packageRoot, "package.json");
const mainPackage = require(packageJsonPath);

const archMap = { aarch64: "arm64", x86_64: "x64", i686: "ia32", armv7: "arm" };

function platformSuffix(triple) {
  const parts = triple.split("-");
  const arch = archMap[parts[0]] || parts[0];
  if (triple.includes("apple-darwin")) return `darwin-${arch}`;
  if (triple.includes("windows")) return `win32-${arch}-msvc`;
  const abi = parts[parts.length - 1];
  return `linux-${arch}-${abi}`;
}

const targets = (mainPackage.napi && mainPackage.napi.targets) || [];
if (targets.length === 0) {
  console.error("No napi.targets found in package.json");
  process.exit(1);
}

const optionalDependencies = {};
for (const triple of targets) {
  const name = `${mainPackage.name}-${platformSuffix(triple)}`;
  optionalDependencies[name] = mainPackage.version;
}

mainPackage.optionalDependencies = optionalDependencies;

fs.writeFileSync(packageJsonPath, JSON.stringify(mainPackage, null, 2) + "\n");
console.log(
  `Injected ${targets.length} optionalDependencies at version ${mainPackage.version}`,
);
