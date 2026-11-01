import { mkdir, readdir, readFile, rm, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repositoryRoot = path.resolve(scriptDir, '../../..');
const sourceDirectory = path.join(repositoryRoot, 'docs-dev');
const docsDirectory = path.join(scriptDir, '../src/content/docs');
const generatedDirectory = path.join(docsDirectory, 'source-guides');

const guideGroups = {
	concepts: ['VISION', 'KERNEL', 'BUNDLES', 'PERSISTENCE', 'WASM', 'FRONT-MODES'],
	'how-to': [
		'RECIPES',
		'SECURITY',
		'FORMS',
		'HTTP_OPS',
		'I18N',
		'PERF',
		'VIEW',
		'CACHE',
		'SESSION',
		'MAILER',
		'NOTIFIER',
		'SEARCH',
		'WORKFLOW',
		'FILESYSTEM',
		'PROCESS',
		'FINDER',
		'HTTP_CLIENT',
		'MESSENGER',
		'LOCK',
		'RATE_LIMITER',
		'SCHEDULER',
		'OBSERVABILITY',
		'ADMIN',
		'PROFILER',
	],
	reference: ['CONSOLE', 'SERIALIZER', 'EXPRESSION', 'ERRORS', 'OPENAPI', 'STRING', 'SUPPLY-CHAIN'],
	examples: ['MYFEED', 'RUSTASHOP'],
};

const routes = new Map();
for (const [group, names] of Object.entries(guideGroups)) {
	for (const name of names) {
		const slug = name.toLowerCase().replaceAll('_', '-');
		const routeSlug = name === 'BUNDLES' ? 'bundle-extensions' : slug;
		routes.set(`${name}.md`, `${group}/${routeSlug}`);
	}
}

const sourceFiles = (await readdir(sourceDirectory))
	.filter((file) => file.endsWith('.md') && file !== 'README.md')
	.sort();
const expectedFiles = [...routes.keys()].sort();
if (sourceFiles.join('\n') !== expectedFiles.join('\n')) {
	const missing = sourceFiles.filter((file) => !routes.has(file));
	const obsolete = expectedFiles.filter((file) => !sourceFiles.includes(file));
	throw new Error(
		`Update guideGroups in scripts/import-guides.mjs. Unmapped: ${missing.join(', ') || 'none'}; missing: ${obsolete.join(', ') || 'none'}`,
	);
}

await rm(generatedDirectory, { recursive: true, force: true });

for (const [group, names] of Object.entries(guideGroups)) {
	const outputGroup = path.join(docsDirectory, group);
	await mkdir(outputGroup, { recursive: true });

	for (const name of names) {
		const filename = `${name}.md`;
		const source = await readFile(path.join(sourceDirectory, filename), 'utf8');
		const titleMatch = source.match(/^#\s+(.+)$/m);
		if (!titleMatch) {
			throw new Error(`${filename} is missing its top-level heading`);
		}

		const title = titleMatch[1].trim();
		const body = source.replace(/^#\s+.+\n*/, '').trimStart();
		const description = body
			.split(/\n\s*\n/)
			.find((paragraph) => paragraph.trim() && !paragraph.trimStart().startsWith('#'))
			?.replace(/\[([^\]]+)\]\([^)]+\)/g, '$1')
			?.replace(/[`*_>\[\]]/g, '')
			.replace(/\s+/g, ' ')
			.trim()
			.slice(0, 180) || `${title} for the Serenade Rust framework.`;
		const currentRoute = routes.get(filename);
		const markdown = body.replace(/\]\(([^)]+)\)/g, (match, destination) => {
			if (/^(?:https?:|mailto:|#|\/)/.test(destination)) {
				return match;
			}

			const [target, fragment] = destination.split('#', 2);
			const targetFilename = path.posix.basename(target);
			if (routes.has(targetFilename) && targetFilename.endsWith('.md')) {
				const relativeRoute = path.posix.relative(currentRoute, routes.get(targetFilename)) || '.';
				const href = `${relativeRoute.startsWith('.') ? relativeRoute : `./${relativeRoute}`}${fragment ? `#${fragment}` : ''}`;
				return `](${href})`;
			}

			const repositoryPath = target.replace(/^\.\.\//, '');
			if (repositoryPath.startsWith('examples/') || repositoryPath.startsWith('crates/')) {
				return `](https://github.com/Interchouette-ITC/Serenade/tree/dev/${repositoryPath}${fragment ? `#${fragment}` : ''})`;
			}

			return match;
		});

		const output = [
			'---',
			`title: ${JSON.stringify(title)}`,
			`description: ${JSON.stringify(description)}`,
			'---',
			'',
			markdown,
			'',
		].join('\n');
		const slug = routes.get(filename).split('/').at(-1);
		await writeFile(path.join(outputGroup, `${slug}.md`), output);
	}
}

const workspaceManifest = await readFile(path.join(repositoryRoot, 'Cargo.toml'), 'utf8');
const members = workspaceManifest.match(/members\s*=\s*\[([\s\S]*?)\]/)?.[1];
if (!members) {
	throw new Error('Cargo.toml is missing workspace members');
}
const workspaceVersion = workspaceManifest.match(/^\[workspace\.package\][\s\S]*?^version\s*=\s*"([^"]+)"/m)?.[1];
if (!workspaceVersion) {
	throw new Error('Cargo.toml is missing workspace.package.version');
}

const cratePaths = [...members.matchAll(/"((?:crates)\/[^"]+)"/g)].map(([, member]) => member);
const catalogRows = [];
for (const cratePath of cratePaths) {
	const manifestPath = path.join(repositoryRoot, cratePath, 'Cargo.toml');
	const manifest = await readFile(manifestPath, 'utf8');
	const packageSection = manifest.match(/^\[package\]\s*([\s\S]*?)(?=^\[|\s*$)/m)?.[1] ?? manifest;
	const name = packageSection.match(/^name\s*=\s*"([^"]+)"/m)?.[1];
	const description = packageSection.match(/^description\s*=\s*"([^"]+)"/m)?.[1];
	const version =
		packageSection.match(/^version\s*=\s*"([^"]+)"/m)?.[1] ?? workspaceVersion;
	if (!name) {
		throw new Error(`${manifestPath} is missing package.name`);
	}

	const directory = path.posix.basename(cratePath);
	const readme = await readFile(path.join(repositoryRoot, cratePath, 'README.md'), 'utf8').then(
		() => true,
		(error) => {
			if (error.code === 'ENOENT') return false;
			throw error;
		},
	);
	const links = [
		`[${name}](https://github.com/Interchouette-ITC/Serenade/tree/dev/${cratePath})`,
	];
	if (readme) {
		links.push(
			`[README](https://github.com/Interchouette-ITC/Serenade/blob/dev/${cratePath}/README.md)`,
		);
	}
	catalogRows.push(
		`| ${links.join(' · ')} | ${(description ?? 'No crate description yet.').replaceAll('|', '\\|')} | ${version.startsWith('0.') ? 'Pre-1.0' : `v${version}`} · readiness not individually assessed |`,
	);
}

const catalog = [
	'---',
	'title: Workspace crate catalog',
	'description: Workspace crate descriptions and links generated from Cargo manifests.',
	'---',
	'',
	`This list is generated from the current workspace membership in [Cargo.toml](https://github.com/Interchouette-ITC/Serenade/blob/dev/Cargo.toml). A pre-1.0 label describes the version only. The project does not maintain per-crate readiness labels; inspect each linked README, tests, and examples before adoption.`,
	'',
	'| Crate and docs | Description | Maturity |',
	'| --- | --- | --- |',
	...catalogRows,
	'',
].join('\n');
const componentsDirectory = path.join(docsDirectory, 'components');
await mkdir(componentsDirectory, { recursive: true });
await writeFile(path.join(componentsDirectory, 'catalog.md'), catalog);
