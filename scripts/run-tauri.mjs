import { execFileSync, spawnSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";

const repoRoot = path.resolve(path.dirname(new URL(import.meta.url).pathname), "..");
const args = process.argv.slice(2);

if (args[0] === "fix-dmg") {
  const dmgPaths = args.slice(1);
  if (dmgPaths.length === 0) {
    console.error("Usage: node scripts/run-tauri.mjs fix-dmg <path-to-dmg> [...]");
    process.exit(1);
  }

  for (const dmgPath of dmgPaths) {
    hideVolumeIconInDmg(path.resolve(repoRoot, dmgPath));
  }
  process.exit(0);
}

const tauriBin = path.join(
  repoRoot,
  "node_modules",
  ".bin",
  process.platform === "win32" ? "tauri.cmd" : "tauri"
);

const tauriRun = spawnSync(tauriBin, args, {
  cwd: repoRoot,
  stdio: "inherit",
});

if (tauriRun.status !== 0) {
  process.exit(tauriRun.status ?? 1);
}

if (process.platform === "darwin" && args.includes("build")) {
  postprocessDmgs();
}

function postprocessDmgs() {
  const dmgDir = path.join(repoRoot, "src-tauri", "target", "release", "bundle", "dmg");
  if (!fs.existsSync(dmgDir)) {
    return;
  }

  const dmgFiles = fs
    .readdirSync(dmgDir)
    .filter((name) => name.endsWith(".dmg"))
    .map((name) => path.join(dmgDir, name));

  for (const dmgFile of dmgFiles) {
    hideVolumeIconInDmg(dmgFile);
  }
}

function hideVolumeIconInDmg(dmgFile) {
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "kuse-dmg-"));
  const mountPoint = path.join(tempDir, "mount");
  const rwBase = path.join(tempDir, "rw");
  const rwDmg = `${rwBase}.dmg`;
  const fixedBase = path.join(tempDir, "fixed");
  const fixedDmg = `${fixedBase}.dmg`;

  fs.mkdirSync(mountPoint);

  try {
    execFileSync("hdiutil", ["convert", dmgFile, "-format", "UDRW", "-o", rwBase], {
      cwd: repoRoot,
      stdio: "inherit",
    });

    execFileSync(
      "hdiutil",
      ["attach", "-readwrite", "-noverify", "-nobrowse", "-mountpoint", mountPoint, rwDmg],
      {
        cwd: repoRoot,
        stdio: "inherit",
      }
    );

    const volumeIconPath = path.join(mountPoint, ".VolumeIcon.icns");
    if (fs.existsSync(volumeIconPath)) {
      fs.rmSync(volumeIconPath, { force: true });
    }

    configureDmgFinderLayout(mountPoint);

    execFileSync("hdiutil", ["detach", mountPoint], {
      cwd: repoRoot,
      stdio: "inherit",
    });

    execFileSync("hdiutil", ["convert", rwDmg, "-format", "UDZO", "-o", fixedBase], {
      cwd: repoRoot,
      stdio: "inherit",
    });

    fs.copyFileSync(fixedDmg, dmgFile);
  } finally {
    try {
      execFileSync("hdiutil", ["detach", mountPoint], {
        cwd: repoRoot,
        stdio: "ignore",
      });
    } catch {}

    fs.rmSync(tempDir, { recursive: true, force: true });
  }
}

function configureDmgFinderLayout(mountPoint) {
  const appName = findAppBundleName(mountPoint);
  if (!appName) {
    return;
  }

  const escapedMountPoint = escapeAppleScriptString(mountPoint);
  const escapedAppName = escapeAppleScriptString(appName);

  const script = `
    set dmgFolder to POSIX file "${escapedMountPoint}" as alias
    tell application "Finder"
      open dmgFolder
      delay 1
      set dmgWindow to front Finder window
      tell dmgWindow
        set current view to icon view
        set toolbar visible to false
        set statusbar visible to false
        set bounds to {120, 120, 780, 500}
      end tell

      tell icon view options of dmgWindow
        set arrangement to not arranged
        set icon size to 128
        set text size to 14
      end tell

      set position of item "${escapedAppName}" of dmgFolder to {170, 180}
      set position of item "Applications" of dmgFolder to {470, 180}
      close dmgWindow
      open dmgFolder
      delay 1
      close front Finder window
    end tell
  `;

  try {
    execFileSync("osascript", ["-e", script], {
      cwd: repoRoot,
      stdio: "inherit",
    });
  } catch (error) {
    console.warn(`Failed to configure Finder layout for ${mountPoint}: ${String(error)}`);
  }
}

function findAppBundleName(mountPoint) {
  return fs.readdirSync(mountPoint).find((name) => name.endsWith(".app"));
}

function escapeAppleScriptString(value) {
  return value.replaceAll("\\", "\\\\").replaceAll("\"", "\\\"");
}
