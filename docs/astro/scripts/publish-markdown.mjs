import { mkdir, readdir, readFile, writeFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const astro = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const serenade = path.resolve(astro, '../..');
const destination = path.join(astro, 'dist/markdown');
const published = [];
const publishedPaths = new Set();
const hashes = {};

async function publishMarkdown(relative, content) {
  if (publishedPaths.has(relative)) throw new Error(`Duplicate Markdown path: ${relative}`);
  const target = path.join(destination, relative);
  await mkdir(path.dirname(target), { recursive: true });
  await writeFile(target, content);
  published.push(relative);
  publishedPaths.add(relative);
  hashes[relative] = createHash('sha256').update(content).digest('hex');
}

async function copyMarkdown(source, relative) {
  await publishMarkdown(relative, await readFile(source));
}

async function publishGuides(directory, relative = '') {
  const entries = await readdir(directory, { withFileTypes: true });
  entries.sort((a, b) => a.name < b.name ? -1 : a.name > b.name ? 1 : 0);
  for (const entry of entries) {
    const source = path.join(directory, entry.name);
    const name = path.posix.join(relative, entry.name);
    if (entry.isDirectory()) {
      await publishGuides(source, name);
    } else if (entry.isFile() && name.endsWith('.md')) {
      await copyMarkdown(source, `serenade/guides/${name}`);
    } else if (entry.isFile() && name.endsWith('.mdx')) {
      if (name !== 'index.mdx') throw new Error(`Unsupported MDX page: ${name}`);
      const mdx = await readFile(source, 'utf8');
      const markdown = mdx
        .replace(/^import \{ Card, CardGrid \} from '@astrojs\/starlight\/components';\r?\n/m, '')
        .replace(/^<CardGrid>\r?\n|^<\/CardGrid>\r?\n/gm, '')
        .replace(
          /^  <Card title="([^"]+)" icon="[^"]+">\r?\n([\s\S]*?)^  <\/Card>\r?\n/gm,
          (_, title, body) => `### ${title}\n\n${body.replace(/^    /gm, '')}\n`,
        );
      if (/<\/?Card(?:Grid)?\b|^import\s/m.test(markdown)) {
        throw new Error(`Unsupported MDX syntax in ${name}`);
      }
      await publishMarkdown('serenade/guides/index.md', markdown);
    }
  }
}

for (const name of (await readdir(path.join(serenade, 'docs-dev'))).sort()) {
  if (name.endsWith('.md')) {
    await copyMarkdown(path.join(serenade, 'docs-dev', name), `serenade/docs-dev/${name}`);
  }
}

await publishGuides(path.join(astro, 'src/content/docs'));

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
