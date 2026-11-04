// Regenerates src-tauri/icons from the brand SVGs. Usage: node scripts/icons.mjs
// Run 1 builds the desktop set from the main icon; run 2 builds only icon.ico
// from the Windows variant. Mobile output (android/ios) is removed.
import { spawnSync } from "node:child_process";
import { copyFileSync, mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";

const root = resolve(import.meta.dirname, "..");
const outDir = join(root, "src-tauri", "icons");
const mainSvg = join(root, "assets", "brand", "tokenmonk-icon.svg");
const winSvg = join(root, "assets", "brand", "tokenmonk-icon-windows.svg");

function tauriIcon(input, output) {
  // shell: true is required because Node 22 will not spawn npx.cmd on Windows
  // without a shell; paths are quoted by hand for that reason.
  const r = spawnSync("npx", ["tauri", "icon", `"${input}"`, "-o", `"${output}"`], {
    cwd: root,
    stdio: "inherit",
    shell: true,
  });
  if (r.error) {
    throw Object.assign(new Error(`could not run npx: ${r.error.message}`), { status: 1 });
  }
  if (r.status !== 0) {
    throw Object.assign(new Error("tauri icon failed"), { status: r.status ?? 1 });
  }
}

const tmp = mkdtempSync(join(tmpdir(), "tokenmonk-icons-"));
try {
  tauriIcon(mainSvg, outDir);
  for (const d of ["android", "ios"]) rmSync(join(outDir, d), { recursive: true, force: true });
  tauriIcon(winSvg, tmp);
  copyFileSync(join(tmp, "icon.ico"), join(outDir, "icon.ico"));
  console.log("icons regenerated in src-tauri/icons");
} catch (err) {
  console.error(err.message);
  process.exitCode = err.status ?? 1;
} finally {
  rmSync(tmp, { recursive: true, force: true });
}
