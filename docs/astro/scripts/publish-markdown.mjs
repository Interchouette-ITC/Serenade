import { copyFile, mkdir, readdir, readFile, writeFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const astro = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const serenade = path.resolve(astro, '../..');
const destination = path.join(astro, 'dist/markdown');
const published = [];
const hashes = {};

async function copyMarkdown(source, relative) {
  const target = path.join(destination, relative);
  await mkdir(path.dirname(target), { recursive: true });
  await copyFile(source, target);
  published.push(relative);
  hashes[relative] = createHash('sha256').update(await readFile(target)).digest('hex');
}

for (const name of (await readdir(path.join(serenade, 'docs-dev'))).sort()) {
  if (name.endsWith('.md')) {
    await copyMarkdown(path.join(serenade, 'docs-dev', name), `serenade/docs-dev/${name}`);
  }
}

for (const group of ['first-steps', 'tutorials']) {
  const directory = path.join(astro, 'src/content/docs', group);
  for (const name of (await readdir(directory)).sort()) {
    if (name.endsWith('.md')) {
      await copyMarkdown(path.join(directory, name), `serenade/guides/${group}/${name}`);
    }
  }
}

for (const relative of ['concepts/architecture.md', 'concepts/bundles.md', 'reference/api-docs.md', 'how-to/agent-mcp.md']) {
  await copyMarkdown(
    path.join(astro, 'src/content/docs', relative),
    `serenade/guides/${relative}`,
  );
}

if (process.env.RANGULAR_DOCS_DIR) {
  const rangularDocs = path.resolve(process.env.RANGULAR_DOCS_DIR);
  for (const name of ['README.md', 'SPEC.md', 'OVERVIEW.md', 'CHANGELOG.md']) {
    await copyMarkdown(path.join(rangularDocs, name), `rangular/docs/${name}`);
  }
}

const workspace = await readFile(path.join(serenade, 'Cargo.toml'), 'utf8');
const version = workspace.match(/^\[workspace\.package\][\s\S]*?^version\s*=\s*"([^"]+)"/m)?.[1];
if (!version) throw new Error('Serenade workspace version not found');
await writeFile(
  path.join(destination, 'index.json'),
  `${JSON.stringify({
    serenadeWorkspaceVersion: version,
    declaredSourceRevisions: {
      serenade: process.env.SERENADE_DOCS_REV ?? null,
      rangular: process.env.RANGULAR_DOCS_REV ?? null,
    },
    files: published,
    sha256: hashes,
  }, null, 2)}\n`,
);
