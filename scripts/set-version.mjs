import { readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const rootDir = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const versionArg = process.argv[2];

if (!versionArg) {
  fail('Usage: bun run set-version <version>\nExample: bun run set-version 1.0.2');
}

const version = versionArg.trim().replace(/^v/i, '');

if (!/^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$/.test(version)) {
  fail(`Invalid version "${versionArg}". Expected semver like 1.0.2 or v1.0.2.`);
}

const files = {
  packageJson: path.join(rootDir, 'package.json'),
  tauriConfig: path.join(rootDir, 'src-tauri', 'tauri.conf.json'),
  cargoToml: path.join(rootDir, 'src-tauri', 'Cargo.toml'),
  settings: path.join(rootDir, 'src', 'windows', 'Settings.tsx'),
};

await replaceInFile(
  files.packageJson,
  /(^\s*"version"\s*:)\s*"[^"]+"/m,
  `$1 "${version}"`,
  'package.json version',
);

await replaceInFile(
  files.tauriConfig,
  /(^\s*"version"\s*:)\s*"[^"]+"/m,
  `$1 "${version}"`,
  'Tauri config version',
);

await replaceInFile(
  files.cargoToml,
  /(^version\s*=\s*)"[^"]+"/m,
  `$1"${version}"`,
  'Cargo package version',
);

await replaceInFile(
  files.settings,
  /(>\s*)v\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?(\s*<\/span>)/,
  `$1v${version}$2`,
  'settings UI version',
);

console.log(`Updated Flamed version to ${version}`);

async function replaceInFile(filePath, pattern, replacement, label) {
  const original = await readFile(filePath, 'utf8');

  if (!pattern.test(original)) {
    fail(`Could not find ${label} in ${path.relative(rootDir, filePath)}.`);
  }

  const updated = original.replace(pattern, replacement);
  await writeFile(filePath, updated);
}

function fail(message) {
  console.error(message);
  process.exit(1);
}
