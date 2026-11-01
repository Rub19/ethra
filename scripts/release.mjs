// Builds the app, packs it into the custom installer, signs it and writes latest.json.
// Usage: npm run release -- "Notes shown in the update card"
import { execSync } from "node:child_process";
import { copyFileSync, createReadStream, createWriteStream, existsSync, mkdirSync, readFileSync, statSync, writeFileSync } from "node:fs";
import { homedir } from "node:os";
import { join } from "node:path";
import { pipeline } from "node:stream/promises";
import { createGzip, constants } from "node:zlib";

const version = JSON.parse(readFileSync("src-tauri/tauri.conf.json", "utf8")).version;
const notes = process.argv[2] ?? "";
const key = join(homedir(), ".tauri", "kairo-updater.key");
if (!existsSync(key)) throw new Error(`Clé de signature introuvable : ${key}`);
const run = (cmd) => execSync(cmd, { stdio: "inherit" });

run("npm run tauri build -- --no-bundle");

mkdirSync("setup/payload", { recursive: true });
await pipeline(
  createReadStream("src-tauri/target/release/ethra.exe"),
  createGzip({ level: constants.Z_BEST_COMPRESSION }),
  createWriteStream("setup/payload/ethra.exe.gz"),
);

run("cargo build --release --manifest-path setup/Cargo.toml");

mkdirSync("release", { recursive: true });
const name = `Kairo_${version}_x64-setup.exe`;
const out = join("release", name);
copyFileSync("setup/target/release/kairo-setup.exe", out);

run(`npx tauri signer sign -f "${key}" -p "" "${out}"`);
writeFileSync(
  "release/latest.json",
  JSON.stringify(
    {
      version,
      notes,
      pub_date: new Date().toISOString(),
      platforms: {
        "windows-x86_64": {
          signature: readFileSync(`${out}.sig`, "utf8").trim(),
          url: `https://github.com/Rub19/ethra/releases/download/v${version}/${name}`,
        },
      },
    },
    null,
    2,
  ),
);
console.log(`${out} (${(statSync(out).size / 1048576).toFixed(1)} Mo) et release/latest.json prêts`);
