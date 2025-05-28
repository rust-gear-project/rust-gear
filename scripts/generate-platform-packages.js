const fs = require("fs");
const path = require("path");

const packageRoot = process.cwd();
const mainPackage = require(path.join(packageRoot, "package.json"));

const unscopedName = mainPackage.name.startsWith("@")
  ? mainPackage.name.split("/")[1]
  : mainPackage.name;

function getOS(platform) {
  return [platform.split("-")[0]];
}

function getCPU(platform) {
  return [platform.split("-")[1]];
}

const npmDir = path.join(packageRoot, "npm");
const platforms = fs.readdirSync(npmDir);

platforms.forEach((platform) => {
  const platformDir = path.join(npmDir, platform);
  if (!fs.statSync(platformDir).isDirectory()) return;
  const binaryName = `${unscopedName}.${platform}.node`;

  const parts = platform.split("-");
  let libc;
  if (parts.length === 3) {
    if (parts[2] === "musl") {
      libc = ["musl"];
    } else if (parts[2] === "gnu") {
      libc = ["glibc"];
    }
  }

  const platformPackage = {
    name: `${mainPackage.name}-${platform}`,
    version: mainPackage.version,
    description: mainPackage.description,
    keywords: [...mainPackage.keywords],
    repository: mainPackage.repository || {},
    license: "Apache-2.0 AND MIT",
    author: mainPackage.author,
    main: binaryName,
    files: [binaryName],
    engines: { node: ">= 10" },
    os: getOS(platform),
    cpu: getCPU(platform),
    ...(libc && { libc }),
    publishConfig: {
      registry: "https://registry.npmjs.org/",
      access: "public",
    },
  };

  fs.writeFileSync(
    path.join(platformDir, "package.json"),
    JSON.stringify(platformPackage, null, 2)
  );
});
