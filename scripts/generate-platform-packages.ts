import fs from "node:fs";
import path from "node:path";
import {
  npmDir,
  platformDirectories,
  readPackageJson,
  type PackageJson,
} from "./package-json.ts";

const mainPackage = readPackageJson();

const unscopedName = mainPackage.name.startsWith("@")
  ? mainPackage.name.split("/")[1]
  : mainPackage.name;

function getOS(platform: string): string[] {
  return [platform.split("-")[0]];
}

function getCPU(platform: string): string[] {
  return [platform.split("-")[1]];
}

// Only the three-part linux platforms carry a libc, e.g. `linux-x64-musl`.
function getLibc(platform: string): string[] | undefined {
  const parts = platform.split("-");
  if (parts.length !== 3) return undefined;
  if (parts[2] === "musl") return ["musl"];
  if (parts[2] === "gnu") return ["glibc"];
  return undefined;
}

type PlatformPackage = PackageJson & {
  main: string;
  files: string[];
  engines: { node: string };
  os: string[];
  cpu: string[];
  libc?: string[];
  license: string;
  publishConfig: { registry: string; access: string };
};

for (const platform of platformDirectories()) {
  const binaryName = `${unscopedName}.${platform}.node`;
  const libc = getLibc(platform);

  const platformPackage: PlatformPackage = {
    name: `${mainPackage.name}-${platform}`,
    version: mainPackage.version,
    description: mainPackage.description,
    keywords: [...(mainPackage.keywords ?? [])],
    repository: mainPackage.repository ?? {},
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
    path.join(npmDir, platform, "package.json"),
    JSON.stringify(platformPackage, null, 2),
  );
}
