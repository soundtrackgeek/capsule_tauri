import { existsSync, readdirSync, readFileSync, statSync, writeFileSync } from "node:fs";
import { basename, join, resolve } from "node:path";
import process from "node:process";

const [artifactDirectory, tag, repository, outputPath] = process.argv.slice(2);

if (!artifactDirectory || !tag || !repository || !outputPath) {
  throw new Error(
    "Usage: node scripts/generate-release-manifest.mjs <artifact-directory> <tag> <repository> <output-path>",
  );
}

const artifacts = listFiles(resolve(artifactDirectory));
const windowsInstaller = findSingleArtifact(artifacts, (file) => file.endsWith("-setup.exe"));
const macosUpdater = findSingleArtifact(artifacts, (file) => file.endsWith(".app.tar.gz"));
const windowsSignature = readSignature(`${windowsInstaller}.sig`);
const macosSignature = readSignature(`${macosUpdater}.sig`);
const version = tag.replace(/^v/, "");
const releaseUrl = `https://github.com/${repository}/releases/download/${tag}`;
const updaterEntry = (file, signature) => ({
  signature,
  url: `${releaseUrl}/${encodeURIComponent(basename(file))}`,
});

const manifest = {
  version,
  notes: "See the GitHub release for details.",
  pub_date: new Date().toISOString().replace(/\.\d{3}Z$/, "Z"),
  platforms: {
    "windows-x86_64": updaterEntry(windowsInstaller, windowsSignature),
    "darwin-x86_64": updaterEntry(macosUpdater, macosSignature),
    "darwin-aarch64": updaterEntry(macosUpdater, macosSignature),
  },
};

writeFileSync(resolve(outputPath), `${JSON.stringify(manifest, null, 2)}\n`);

function listFiles(directory) {
  if (!existsSync(directory)) {
    throw new Error(`Artifact directory does not exist: ${directory}`);
  }

  return readdirSync(directory).flatMap((name) => {
    const path = join(directory, name);
    return statSync(path).isDirectory() ? listFiles(path) : [path];
  });
}

function findSingleArtifact(files, predicate) {
  const matches = files.filter(predicate);
  if (matches.length !== 1) {
    throw new Error(`Expected exactly one matching updater artifact, found ${matches.length}.`);
  }
  return matches[0];
}

function readSignature(path) {
  if (!existsSync(path)) {
    throw new Error(`Updater signature does not exist: ${path}`);
  }
  return readFileSync(path, "utf8").trim();
}
