const fs = require("fs");
const path = require("path");

const packageRoot = process.cwd();
const packageJsonPath = path.join(packageRoot, "package.json");
const mainPackage = require(packageJsonPath);

const updatedOptionalDependencies = {};

for (const [packageName, _] of Object.entries(
  mainPackage.optionalDependencies
)) {
  updatedOptionalDependencies[packageName] = mainPackage.version;
}

mainPackage.optionalDependencies = updatedOptionalDependencies;

fs.writeFileSync(packageJsonPath, JSON.stringify(mainPackage, null, 2) + "\n");
