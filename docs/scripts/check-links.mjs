// Check every internal link in the built site, anchors included.
//
// Astro reports a link to a page that does not exist, but says nothing about a
// link to an anchor that does not exist — and the reference pages are generated,
// so their anchors are computed twice: once by the slugger that writes the
// heading id, once by whatever turns a doc-comment reference into a URL. When
// those two disagree the link still renders, it just goes nowhere.

import { readdir, readFile } from 'node:fs/promises';
import { join } from 'node:path';

const ROOT = 'dist';
const ASSET = /\.(css|js|svg|png|jpg|webp|xml|ico|json|txt|pagefind)$/;

// Served under a path rather than at a domain root, every href carries that
// prefix while the file it names does not. Strip it before matching, or every
// link on a project-page build reads as broken.
const BASE = (process.env.BASE || '/').replace(/\/$/, '');

const pages = [];
async function walk(dir = '') {
	for (const entry of await readdir(join(ROOT, dir), { withFileTypes: true })) {
		const path = dir ? `${dir}/${entry.name}` : entry.name;
		if (entry.isDirectory()) await walk(path);
		else if (entry.name === 'index.html') pages.push(path);
	}
}
await walk();

const ids = new Map();
const html = new Map();
for (const page of pages) {
	const source = await readFile(join(ROOT, page), 'utf8');
	const url = `/${page.replace(/index\.html$/, '')}`;
	html.set(url, source);
	ids.set(url, new Set([...source.matchAll(/\s(?:id|name)="([^"]+)"/g)].map((m) => m[1])));
}

const broken = [];
for (const [from, source] of html) {
	// The footer and nav repeat every page's links; checking the body is enough.
	const seen = new Set();
	for (const [, path, hash] of source.split('<footer')[0].matchAll(/href="(\/[^"#]*)(#[^"]*)?"/g)) {
		if (ASSET.test(path)) continue;
		const unprefixed = BASE && path.startsWith(`${BASE}/`) ? path.slice(BASE.length) : path;
		const to = unprefixed.endsWith('/') ? unprefixed : `${unprefixed}/`;
		if (seen.has(to + (hash ?? ''))) continue;
		seen.add(to + (hash ?? ''));
		if (!ids.has(to)) broken.push(`${from} -> ${path} (no such page)`);
		else if (hash && !ids.get(to).has(hash.slice(1)))
			broken.push(`${from} -> ${path}${hash} (no such anchor)`);
	}
}

if (broken.length) {
	console.error(`links: ${broken.length} broken\n  ${broken.join('\n  ')}`);
	process.exit(1);
}
console.log(`links: ${pages.length} pages, every internal link and anchor resolves`);
