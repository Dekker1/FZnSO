// Renders `registry/*.toml` into Starlight pages.
//
// The TOML files are the single source of truth for the common names. Nothing
// here is hand-edited: `pnpm gen` runs before `astro dev` and `astro build`, and
// the output is gitignored.
//
// Each category differs only in how an entry's signature is spelled, so that is
// the only per-category code below.

import { mkdir, readdir, readFile, rm, writeFile } from 'node:fs/promises';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import katex from 'katex';
import { parse } from 'smol-toml';
import { attr, mdx } from './mdx.mjs';

const ROOT = dirname(dirname(fileURLToPath(import.meta.url)));
const SRC = join(ROOT, 'registry');
const OUT = join(ROOT, 'src', 'content', 'docs', 'registry');

/**
 * Split `"type: name"` into its two halves.
 *
 * Type first, as MiniZinc declares a predicate — the types are written in
 * MiniZinc's own notation, so writing them in MiniZinc's order costs nothing
 * and spares a reader the transposition.
 */
const parseArg = (arg) => {
	const at = arg.indexOf(': ');
	return at === -1 ? { name: arg, type: '' } : { type: arg.slice(0, at), name: arg.slice(at + 2) };
};

/**
 * How each category spells one entry's signature, and what it calls its items.
 *
 * Constraints are spread over several files so that no one page carries 160
 * entries; they all share this shape, and are picked up from disk rather than
 * listed here.
 */
const CONSTRAINTS = {
	noun: 'constraint',
	constraint: true,
	signature: (e) => `${e.ident}(${(e.args ?? []).join(', ')})`,
};

const CATEGORIES = {
	objectives: {
		noun: 'objective strategy',
		signature: (e) => `${e.ident}(${e.arg ?? ''})`,
	},
	options: {
		noun: 'option',
		signature: (e) => `${e.ident}: ${e.type} = ${e.default}`,
	},
	statistics: {
		noun: 'statistic',
		signature: (e) => `${e.ident}: ${e.type}`,
	},
	'message-scopes': {
		noun: 'message scope',
		signature: (e) => `${e.ident}: ${e.type}`,
	},
};

/**
 * The `_reif` and `_imp` names an entry also declares.
 *
 * Every constraint names both unless it is functional, so the names are derived
 * rather than listed: a list would be a second copy of the same fact, and the
 * ad-hoc one this replaced had drifted to a quarter of the entries.
 */
const variantNames = (e) =>
	e.functional || !e.args ? [] : [`${e.ident}_reif`, `${e.ident}_imp`];

/**
 * The tokens a filter query is matched against.
 *
 * `lin_le` also finds `lin le`, and a folded variant name such as
 * `int_le_reif` finds the `int_le` entry that documents it.
 */
const searchKey = (e) =>
	[
		e.ident,
		e.ident.replaceAll('_', ' '),
		...variantNames(e),
		...(e.args ?? []).map((a) => parseArg(a).name),
		e.summary,
		e.definition ?? '',
	]
		.join(' ')
		.toLowerCase();

/**
 * Wrap multi-letter argument names so KaTeX sets them as one identifier.
 *
 * Left alone, `coeffs_i` typesets as the product of six italic letters. Only
 * names the entry actually declares are touched, so a LaTeX command is never
 * caught by accident.
 */
function markUpNames(tex, args) {
	const names = (args ?? [])
		.map((a) => parseArg(a).name)
		.filter((n) => n.length > 1)
		// Longest first, so `lower_bound` wins over a hypothetical `lower`.
		.sort((a, b) => b.length - a.length)
		.map((n) => n.replace(/[.*+?^${}()|[\]\\]/g, '\\$&'));
	if (!names.length) return tex;
	// One pass over one alternation: substituting name by name would let a later
	// name match inside the `\mathit{…}` an earlier one just inserted.
	const pattern = new RegExp(`(?<![\\\\A-Za-z])(${names.join('|')})(?![A-Za-z])`, 'g');
	// `\text{…}` holds prose, and prose contains ordinary words like "from" and
	// "to" that happen to be argument names. Substituting there would both change
	// the sentence and be invalid: `\mathit` cannot appear in text mode.
	const protectedGroup = /\\(?:text|operatorname|mathrm)\*?\{[^}]*\}/g;
	// The underscore has to be escaped inside the wrapper: `\mathit{lower_bound}`
	// typesets as "lower" with a "bound" subscript, which is a different claim.
	const wrap = (s) => s.replace(pattern, (_, name) => `\\mathit{${name.replace(/_/g, '\\_')}}`);

	let out = '';
	let at = 0;
	for (const m of tex.matchAll(protectedGroup)) {
		out += wrap(tex.slice(at, m.index)) + m[0];
		at = m.index + m[0].length;
	}
	return out + wrap(tex.slice(at));
}

/**
 * Render a page's `[[notation]]` block as a table.
 *
 * The symbols the definitions lean on are typeset by the same KaTeX that
 * typesets the definitions, so the two cannot drift apart in look or in
 * spelling. Written as prose with code spans instead, they read as a different
 * language from the formulas that use them.
 *
 * Argument names are marked up against every name declared anywhere on the
 * page: a notation entry belongs to no single constraint, but the names it
 * mentions are the ones its constraints declare.
 */
function renderNotation(notation, entries, file) {
	if (!notation?.length) return '';
	const args = [...new Set(entries.flatMap((e) => e.args ?? []))];
	const cell = (tex) => {
		if (!tex) return '<td />';
		const marked = markUpNames(tex.trim(), args);
		try {
			katex.renderToString(marked, { displayMode: false, throwOnError: true, strict: 'warn' });
		} catch (error) {
			throw new Error(
				`registry/${file}.toml: notation \`${tex}\` does not typeset: ${error.message}`
			);
		}
		return `<td><Formula tex={${JSON.stringify(marked)}} inline /></td>`;
	};
	const rows = notation.map(
		(n) => `<tr>${cell(n.tex)}${cell(n.is)}<td>${mdx(n.means ?? '')}</td></tr>`
	);
	return [
		'\n## Notation',
		'',
		'The definitions below are formulas rather than sentences, and use these throughout.',
		'',
		`<table class="notation"><tbody>\n${rows.join('\n')}\n</tbody></table>\n`,
	].join('\n');
}

/**
 * Check that a definition only refers to names the signature declares.
 *
 * This is what stops a formula drifting from its arguments when one is renamed:
 * the two are written in different places and nothing else relates them.
 */
function checkDefinition(entry, file) {
	const declared = new Set((entry.args ?? []).map((a) => parseArg(a).name));

	/** `xs_i` counts as `xs`; a bare single letter is a free index. */
	const known = (name) => {
		let n = name;
		while (n.length > 1) {
			if (declared.has(n)) return true;
			const cut = n.lastIndexOf('_');
			if (cut <= 0) return false;
			n = n.slice(0, cut);
		}
		return true;
	};

	const used =
		entry.definition
			// Operator, text and environment names name themselves, not arguments.
			.replace(/\\(?:operatorname|text|mathrm|mathit|mathbb|mathcal)\*?\{[^}]*\}/g, ' ')
			.replace(/\\(?:begin|end)\{[^}]*\}/g, ' ')
			.replace(/\\[A-Za-z]+/g, ' ')
			.match(/[A-Za-z][A-Za-z0-9]*(?:_[A-Za-z0-9]+)*/g) ?? [];

	for (const [, name] of entry.definition.matchAll(
		/\\(?:operatorname|text|mathrm)\*?\{([^}]*)\}/g,
	)) {
		if (/(?<!\\\\)_/.test(name)) {
			throw new Error(
				`registry/${file}.toml: \`${entry.ident}\` definition has an unescaped underscore in ` +
					`\`${name}\` — inside an operator name it typesets as a subscript. Escape it, or ` +
					`state the condition directly instead of naming another constraint.`,
			);
		}
	}

	// Render it here rather than leaving it to the page build, which reports a
	// KaTeX stack trace with no clue which of 200 entries produced it.
	try {
		katex.renderToString(markUpNames(entry.definition, entry.args), {
			displayMode: true,
			throwOnError: true,
		});
	} catch (err) {
		throw new Error(
			`registry/${file}.toml: \`${entry.ident}\` definition does not typeset: ${err.message}`,
		);
	}

	for (const name of used) {
		if (!known(name)) {
			throw new Error(
				`registry/${file}.toml: \`${entry.ident}\` definition refers to \`${name}\`, ` +
					`which is not one of its arguments (${[...declared].join(', ')})`,
			);
		}
	}
}

function renderEntry(entry, category) {
	const notes = entry.notes ? `\n${mdx(entry.notes.trim())}\n` : '';
	const formula = entry.definition
		? `\n<Formula tex={${JSON.stringify(markUpNames(entry.definition.trim(), entry.args))}} />\n`
		: '';
	// The reified and implied forms take the same arguments plus `r`, so they are
	// derived rather than given entries of their own.
	const named = variantNames(entry);
	const variants = named.length
		? `\n<p class="entry-variants">also declarable as ${named
				.map((v) => `<code>${v}</code>`)
				.join(' and ')}</p>\n`
		: '';
	return `
<div class="filter-entry" data-search="${attr(searchKey(entry))}">

### ${mdx(entry.ident)}

<Signature text={${JSON.stringify(category.signature(entry))}}${category.constraint ? ' typeFirst' : ''} />
${variants}
${mdx(entry.summary.trim())}
${formula}${notes}
</div>
`;
}

async function renderCategory(name) {
	const category = CATEGORIES[name] ?? CONSTRAINTS;
	const doc = parse(await readFile(join(SRC, `${name}.toml`), 'utf8'));
	const entries = doc.entry ?? [];

	// A duplicate identifier would silently shadow an entry on the page and, worse,
	// mean two different meanings for one name — the thing the registry exists to
	// prevent. Fail the build instead.
	const seen = new Set();
	for (const e of entries) {
		if (seen.has(e.ident)) {
			throw new Error(`registry/${name}.toml: duplicate identifier \`${e.ident}\``);
		}
		seen.add(e.ident);

		if (!category.constraint) continue;
		// A constraint without these is not documented, it is merely listed.
		for (const field of ['args', 'summary', 'definition']) {
			if (!e[field]) {
				throw new Error(`registry/${name}.toml: \`${e.ident}\` has no \`${field}\``);
			}
		}
		for (const arg of e.args) {
			if (!arg.includes(': ')) {
				throw new Error(
					`registry/${name}.toml: \`${e.ident}\` argument \`${arg}\` is unnamed ` +
						`(expected "name: type")`,
				);
			}
		}
		checkDefinition(e, name);
	}

	entries.sort((a, b) => a.ident.localeCompare(b.ident));

	const body = entries.map((e) => renderEntry(e, category)).join('\n');
	const notation = renderNotation(doc.notation, entries, name);
	// A nested page sits one directory deeper, so its imports climb one more level.
	const up = '../'.repeat(name.split('/').length + 2);
	// `label` shortens the sidebar entry where the full title would be redundant
	// under its group — "Arithmetic" rather than "Arithmetic constraints".
	const sidebarFields = [
		doc.meta.order ? `  order: ${doc.meta.order}` : '',
		doc.meta.label ? `  label: ${JSON.stringify(doc.meta.label)}` : '',
	].filter(Boolean);
	const sidebar = sidebarFields.length ? `sidebar:\n${sidebarFields.join('\n')}\n` : '';

	const page = `---
# Generated from registry/${name}.toml by scripts/gen-registry.mjs — do not edit.
title: ${JSON.stringify(doc.meta.title)}
description: ${JSON.stringify(doc.meta.description)}
${sidebar}---

import FilterBox from '${up}components/FilterBox.astro';
import Signature from '${up}components/Signature.astro';
import Formula from '${up}components/Formula.astro';

${doc.meta.intro.trim()}
${notation}
<FilterBox count={${entries.length}} noun="${category.noun}" />

<div class="filter-list">
${body}
</div>
`;

	await mkdir(dirname(join(OUT, `${name}.mdx`)), { recursive: true });
	await writeFile(join(OUT, `${name}.mdx`), page);
	return entries.length;
}

await mkdir(OUT, { recursive: true });

/** Every `.toml` under `registry/`, as a path relative to it and without the extension. */
async function findPages(dir = '') {
	const found = [];
	for (const entry of await readdir(join(SRC, dir), { withFileTypes: true })) {
		const path = dir ? `${dir}/${entry.name}` : entry.name;
		if (entry.isDirectory()) found.push(...(await findPages(path)));
		else if (entry.name.endsWith('.toml')) found.push(path.slice(0, -'.toml'.length));
	}
	return found.sort();
}

const pages = await findPages();

// Deleting a TOML would otherwise leave its page behind, still in the sidebar
// and still indexed, with nothing left to regenerate it from.
const wanted = new Set(pages.map((name) => `${name}.mdx`));
async function pruneStale(dir = '') {
	for (const entry of await readdir(join(OUT, dir), { withFileTypes: true }).catch(() => [])) {
		const path = dir ? `${dir}/${entry.name}` : entry.name;
		if (entry.isDirectory()) await pruneStale(path);
		else if (entry.name.endsWith('.mdx') && !wanted.has(path)) {
			await rm(join(OUT, path));
			console.log(`registry: removed stale ${path}`);
		}
	}
}
await pruneStale();

let total = 0;
for (const name of pages) {
	if (!CATEGORIES[name] && !name.startsWith('constraints/')) {
		throw new Error(`registry/${name}.toml: no category shape for this file`);
	}
	const n = await renderCategory(name);
	total += n;
	console.log(`registry: ${name} — ${n} entries`);
}
console.log(`registry: ${total} entries over ${pages.length} pages`);
