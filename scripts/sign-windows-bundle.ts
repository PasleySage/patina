/**
 * Sign the Windows build output with a local code-signing certificate.
 *
 * This deliberately lives outside `tauri.conf.json`. A `bundle.windows.signCommand` needs an
 * absolute path to `signtool` plus a certificate thumbprint, and both belong to one machine;
 * committing them makes every other build try to launch a path that is not there, which
 * fails the bundle instead of skipping the step. Signing runs as its own command, and a
 * machine without a certificate reports that and exits clean instead of failing a build.
 *
 * Environment:
 *   PATINA_SIGN_SHA1        Certificate thumbprint. Required to sign; without it the script
 *                           reports that it skipped and exits 0.
 *   PATINA_SIGNTOOL         Explicit `signtool.exe` path. Optional; auto-detected from the
 *                           newest Windows Kits install otherwise.
 *   PATINA_SIGN_TIMESTAMP   RFC 3161 timestamp URL. Defaults to DigiCert.
 */

import { execFileSync } from "node:child_process";
import { existsSync } from "node:fs";
import { readdir, readFile } from "node:fs/promises";
import path from "node:path";

const ROOT = path.resolve(import.meta.dirname ?? process.cwd(), "..");
const RELEASE_DIR = path.join(ROOT, "src-tauri", "target", "release");
const BUNDLE_DIR = path.join(RELEASE_DIR, "bundle");
const DEFAULT_TIMESTAMP_URL = "http://timestamp.digicert.com";
const TARGET_EXTENSIONS = [".exe", ".msi"];

/** Newest `signtool.exe` under the Windows Kits install, if the SDK is present. */
async function findSigntool(): Promise<string | null> {
  const explicit = process.env.PATINA_SIGNTOOL?.trim();
  if (explicit) {
    return existsSync(explicit) ? explicit : null;
  }

  const kitsBin = path.join(
    process.env["ProgramFiles(x86)"] ?? "C:\\Program Files (x86)",
    "Windows Kits",
    "10",
    "bin",
  );
  if (!existsSync(kitsBin)) return null;

  const versions = (await readdir(kitsBin, { withFileTypes: true }))
    .filter((entry) => entry.isDirectory())
    // Version segments compare correctly as strings: each is a fixed-width number.
    .map((entry) => entry.name)
    .sort()
    .reverse();

  for (const version of versions) {
    const candidate = path.join(kitsBin, version, "x64", "signtool.exe");
    if (existsSync(candidate)) return candidate;
  }
  return null;
}

/** The signable artifacts for the current version, ignoring leftovers from earlier builds. */
async function collectTargets(version: string): Promise<string[]> {
  const targets: string[] = [];

  const releaseExe = path.join(RELEASE_DIR, "Patina.exe");
  if (existsSync(releaseExe)) targets.push(releaseExe);

  for (const sub of ["nsis", "msi"]) {
    const dir = path.join(BUNDLE_DIR, sub);
    if (!existsSync(dir)) continue;

    const entries = await readdir(dir, { withFileTypes: true });
    for (const entry of entries) {
      if (!entry.isFile()) continue;
      if (!TARGET_EXTENSIONS.includes(path.extname(entry.name).toLowerCase())) continue;
      // `.exe.sig` / `.msi.sig` are updater signature blobs, not artifacts to sign.
      // Bundle output from an earlier version stays in this directory after a bump, so
      // match on the version to avoid signing installers this build did not produce.
      if (!entry.name.includes(`_${version}_`)) continue;
      targets.push(path.join(dir, entry.name));
    }
  }

  return targets;
}

async function main(): Promise<void> {
  const sha1 = process.env.PATINA_SIGN_SHA1?.trim();
  if (!sha1) {
    console.log("PATINA_SIGN_SHA1 is not set; skipping code signing.");
    return;
  }

  const signtool = await findSigntool();
  if (!signtool) {
    throw new Error(
      "signtool.exe was not found. Install the Windows SDK or set PATINA_SIGNTOOL.",
    );
  }

  const manifest = JSON.parse(
    await readFile(path.join(ROOT, "package.json"), "utf8"),
  ) as { version?: string };
  const version = manifest.version;
  if (!version) {
    throw new Error("package.json has no version; cannot tell which artifacts to sign.");
  }

  const targets = await collectTargets(version);
  if (targets.length === 0) {
    throw new Error(
      `No signable artifacts for ${version} found under ${BUNDLE_DIR}. Run a build first.`,
    );
  }

  const timestampUrl = process.env.PATINA_SIGN_TIMESTAMP?.trim() || DEFAULT_TIMESTAMP_URL;
  for (const target of targets) {
    execFileSync(
      signtool,
      ["sign", "/sha1", sha1, "/fd", "sha256", "/tr", timestampUrl, "/td", "sha256", target],
      { stdio: "inherit" },
    );
    console.log(`Signed ${path.relative(ROOT, target)}`);
  }

  console.log(`Signed ${targets.length} artifact(s).`);
}

await main();
