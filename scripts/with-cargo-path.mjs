#!/usr/bin/env node
import { spawnSync } from "node:child_process";
import { existsSync } from "node:fs";
import { homedir } from "node:os";
import { delimiter, join } from "node:path";

const cargoBin = join(homedir(), ".cargo", "bin");
const cargoExe = join(cargoBin, process.platform === "win32" ? "cargo.exe" : "cargo");

if (existsSync(cargoExe)) {
  const pathKey = process.platform === "win32" ? "Path" : "PATH";
  const current = process.env[pathKey] ?? process.env.PATH ?? "";
  if (!current.split(delimiter).some((p) => p === cargoBin)) {
    process.env[pathKey] = `${cargoBin}${delimiter}${current}`;
    process.env.PATH = process.env[pathKey];
  }
}

const args = process.argv.slice(2);
const result = spawnSync(args[0], args.slice(1), {
  stdio: "inherit",
  shell: process.platform === "win32",
  env: process.env,
});
process.exit(result.status ?? 1);
