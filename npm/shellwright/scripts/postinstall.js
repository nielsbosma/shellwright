#!/usr/bin/env node

// Post-install script: resolves the platform-specific binary package
// and creates symlinks/shims in the bin directory.

const fs = require("fs");
const path = require("path");

const PLATFORMS = {
  "win32-x64": "@shellwright/cli-win32-x64",
  "linux-x64": "@shellwright/cli-linux-x64",
  "darwin-x64": "@shellwright/cli-darwin-x64",
  "darwin-arm64": "@shellwright/cli-darwin-arm64",
};

const EXE_SUFFIX = process.platform === "win32" ? ".exe" : "";
const PLATFORM_KEY = `${process.platform}-${process.arch}`;

function main() {
  const pkgName = PLATFORMS[PLATFORM_KEY];
  if (!pkgName) {
    console.error(
      `shellwright: unsupported platform ${PLATFORM_KEY}. ` +
        `Supported: ${Object.keys(PLATFORMS).join(", ")}`
    );
    console.error("Install from source instead: cargo install shellwright");
    process.exit(0); // Don't fail install — just warn
  }

  let pkgDir;
  try {
    pkgDir = path.dirname(require.resolve(`${pkgName}/package.json`));
  } catch {
    console.error(
      `shellwright: platform package ${pkgName} not installed. ` +
        "This usually means your package manager skipped optional dependencies. " +
        "Try: npm install --include=optional"
    );
    process.exit(0);
  }

  const binDir = path.join(__dirname, "..", "bin");
  fs.mkdirSync(binDir, { recursive: true });

  for (const binary of ["shellwright", "shellwrightd"]) {
    const src = path.join(pkgDir, `${binary}${EXE_SUFFIX}`);
    const dest = path.join(binDir, `${binary}${EXE_SUFFIX}`);

    if (!fs.existsSync(src)) {
      console.error(`shellwright: binary not found: ${src}`);
      continue;
    }

    // Copy binary (symlinks can be unreliable across platforms/node_modules layouts)
    fs.copyFileSync(src, dest);
    fs.chmodSync(dest, 0o755);
  }

  // Create shell shims for Unix (npm needs these to find the binaries)
  if (process.platform !== "win32") {
    for (const binary of ["shellwright", "shellwrightd"]) {
      const shimPath = path.join(binDir, binary);
      // The binary IS the shim on Unix — already copied above
      // Just ensure it's executable
      try {
        fs.chmodSync(shimPath, 0o755);
      } catch {}
    }
  }

  console.log(`shellwright: installed binaries for ${PLATFORM_KEY}`);
}

main();
