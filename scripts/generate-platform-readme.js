const fs = require("fs");
const path = require("path");

const packageRoot = process.cwd();
const mainPackage = require(path.join(packageRoot, "package.json"));

function getRustTarget(platform) {
  const [os, arch, libcOrMsvc] = platform.split("-");

  const rustOS = {
    darwin: "apple-darwin",
    linux: "unknown-linux",
    win32: "pc-windows-msvc",
  }[os];

  const rustArch = {
    x64: "x86_64",
    ia32: "i686",
    arm64: "aarch64",
  }[arch];

  if (os === "linux") {
    return `${rustArch}-${rustOS}-${libcOrMsvc}`;
  } else {
    return `${rustArch}-${rustOS}`;
  }
}

const npmDir = path.join(packageRoot, "npm");
const platforms = fs.readdirSync(npmDir);

platforms.forEach((platform) => {
  const platformDir = path.join(npmDir, platform);
  if (!fs.statSync(platformDir).isDirectory()) return;

  const rustTarget = getRustTarget(platform);
  const readmeContent = `# \`${mainPackage.name}-${platform}\`

This is the **${rustTarget}** binary for \`${mainPackage.name}\`
`;

  fs.writeFileSync(path.join(platformDir, "README.md"), readmeContent);
});
