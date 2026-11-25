import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readdir, readFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import test from 'node:test';

const astro = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const source = path.join(astro, 'src/content/docs');
const destination = path.join(astro, 'dist/markdown');

async function pages(directory, prefix = '') {
  const result = [];
  for (const entry of await readdir(directory, { withFileTypes: true })) {
    const name = path.posix.join(prefix, entry.name);
    if (entry.isDirectory()) {
      result.push(...await pages(path.join(directory, entry.name), name));
    } else if (entry.isFile() && /\.mdx?$/.test(name)) {
      result.push(`serenade/guides/${name.replace(/\.mdx$/, '.md')}`);
    }
  }
  return result;
}

test('Markdown index covers every website page and hashes the published files', async () => {
  const index = JSON.parse(await readFile(path.join(destination, 'index.json'), 'utf8'));
  const indexed = new Set(index.files);
  assert.equal(indexed.size, index.files.length, 'duplicate index entries');
  for (const page of await pages(source)) {
    assert.ok(indexed.has(page), `missing website page ${page}`);
  }
  for (const file of index.files) {
    const data = await readFile(path.join(destination, file));
    assert.equal(createHash('sha256').update(data).digest('hex'), index.sha256[file], file);
  }
  const home = await readFile(path.join(destination, 'serenade/guides/index.md'), 'utf8');
  assert.match(home, /^### Start here$/m);
  assert.doesNotMatch(home, /<\/?Card(?:Grid)?\b|^import\s/m);
});
