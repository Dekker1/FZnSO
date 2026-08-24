// Doxygen XML → the shared API IR.
//
// Only `prot="public"` members are surfaced: the headers are header-only, so
// their private helpers are visible to Doxygen but are not interface.

import { readdir, readFile } from 'node:fs/promises';
import { join } from 'node:path';
import { XMLParser } from 'fast-xml-parser';

const parser = new XMLParser({
	ignoreAttributes: false,
	attributeNamePrefix: '@',
	// Order matters when flattening prose back to markdown: `a <code>b</code> c`
	// must not become "a c" + "b".
	preserveOrder: true,
	trimValues: false,
});

/** Sections whose members are part of the public surface. */
const PUBLIC_SECTIONS = new Set([
	'public-func',
	'public-static-func',
	'public-attrib',
	'public-static-attrib',
	'public-type',
	'user-defined',
	'enum',
	'typedef',
	'func',
	'var',
]);

// A `preserveOrder` node is `{ tagName: [...children], ':@': {attributes} }`,
// with no guaranteed key order — so the children must be looked up by the tag
// name rather than taken as the first value.
const tag = (node) => Object.keys(node).find((k) => k !== ':@');
const kids = (node) => {
	const children = node[tag(node)];
	return Array.isArray(children) ? children : [];
};

/** Flatten a Doxygen description tree into markdown. */
function describe(nodes, depth = 0) {
	let out = '';
	for (const node of nodes ?? []) {
		const name = tag(node);
		if (name === '#text') {
			out += node['#text'];
			continue;
		}
		const inner = kids(node);
		switch (name) {
			case 'para':
				out += `${describe(inner, depth).trim()}\n\n`;
				break;
			case 'computeroutput':
			case 'ref':
				out += `\`${describe(inner, depth).trim()}\``;
				break;
			case 'bold':
				out += `**${describe(inner, depth).trim()}**`;
				break;
			case 'emphasis':
				out += `*${describe(inner, depth).trim()}*`;
				break;
			case 'itemizedlist':
			case 'orderedlist': {
				const marker = name === 'itemizedlist' ? '-' : '1.';
				for (const li of inner) {
					const text = describe(kids(li), depth + 1).trim().replace(/\n+/g, ' ');
					out += `${'  '.repeat(depth)}${marker} ${text}\n`;
				}
				out += '\n';
				break;
			}
			case 'simplesect': {
				const kind = node[':@']?.['@kind'] ?? 'note';
				const label = kind.charAt(0).toUpperCase() + kind.slice(1);
				out += `\n**${label}:** ${describe(inner, depth).trim()}\n\n`;
				break;
			}
			case 'programlisting':
			case 'verbatim': {
				const code = describe(inner, depth).replace(/\n+$/, '');
				out += `\n\`\`\`cpp\n${code}\n\`\`\`\n\n`;
				break;
			}
			case 'codeline':
				out += `${describe(inner, depth)}\n`;
				break;
			case 'highlight':
				out += describe(inner, depth);
				break;
			case 'sp':
				out += ' ';
				break;
			case 'linebreak':
				out += '\n';
				break;
			case 'ulink':
				out += `[${describe(inner, depth).trim()}](${node[':@']?.['@url'] ?? ''})`;
				break;
			case 'parameterlist':
			case 'inbodydescription':
				break;
			default:
				out += describe(inner, depth);
		}
	}
	return out;
}

/** `briefdescription` then `detaileddescription`, as one markdown block. */
function docsOf(children) {
	const pick = (name) => children.find((c) => tag(c) === name);
	const brief = describe(kids(pick('briefdescription') ?? {})).trim();
	const detail = describe(kids(pick('detaileddescription') ?? {})).trim();
	return [brief, detail].filter(Boolean).join('\n\n');
}

const textOf = (children, name) => {
	const node = children.find((c) => tag(c) === name);
	return node ? describe(kids(node)).trim() : '';
};

function memberSignature(children, kind) {
	const name = textOf(children, 'name');
	const type = textOf(children, 'type');
	const args = textOf(children, 'argsstring');
	if (kind === 'function') return `${type ? `${type} ` : ''}${name}${args}`;
	if (kind === 'enum') return `enum ${name}`;
	if (kind === 'typedef') return textOf(children, 'definition') || `using ${name}`;
	return `${type ? `${type} ` : ''}${name}`;
}

function enumValues(children) {
	return children
		.filter((c) => tag(c) === 'enumvalue')
		.map((c) => {
			const inner = kids(c);
			return {
				kind: 'variant',
				name: textOf(inner, 'name'),
				signature: textOf(inner, 'name'),
				docs: docsOf(inner),
				links: {},
			};
		});
}

function membersOf(compoundChildren, repo) {
	const out = [];
	for (const section of compoundChildren.filter((c) => tag(c) === 'sectiondef')) {
		const kind = section[':@']?.['@kind'] ?? '';
		if (!PUBLIC_SECTIONS.has(kind)) continue;
		for (const member of kids(section).filter((c) => tag(c) === 'memberdef')) {
			const attrs = member[':@'] ?? {};
			if (attrs['@prot'] !== 'public') continue;
			const children = kids(member);
			const memberKind = attrs['@kind'];
			const name = textOf(children, 'name');
			if (!name || name.startsWith('@')) continue;
			out.push({
				kind: memberKind === 'function' ? 'method' : memberKind === 'variable' ? 'field' : memberKind,
				name,
				signature: memberSignature(children, memberKind),
				docs: docsOf(children),
				links: {},
				members: memberKind === 'enum' ? enumValues(children) : undefined,
				source: sourceLink(children, repo),
			});
		}
	}
	return out;
}

function sourceLink(children, repo) {
	const loc = children.find((c) => tag(c) === 'location');
	if (!loc || !repo) return null;
	const file = loc[':@']?.['@file'];
	const line = loc[':@']?.['@line'];
	return file ? `${repo}/blob/main/${file}#L${line}` : null;
}

/** Parse a Doxygen XML output directory into IR groups. */
export async function parseCpp(xmlDir, { repo, header }) {
	const files = (await readdir(xmlDir)).filter(
		(f) => f.endsWith('.xml') && (f.startsWith('class') || f.startsWith('struct')),
	);

	const classes = [];
	for (const file of files.sort()) {
		const doc = parser.parse(await readFile(join(xmlDir, file), 'utf8'));
		const root = doc.find((d) => tag(d) === 'doxygen');
		if (!root) continue;
		const compound = kids(root).find((c) => tag(c) === 'compounddef');
		if (!compound) continue;
		const attrs = compound[':@'] ?? {};
		if (attrs['@prot'] !== 'public') continue;

		const children = kids(compound);
		const name = textOf(children, 'compoundname');
		// One page per header, so a class only belongs to the header it lives in.
		const loc = children.find((c) => tag(c) === 'location');
		if (header && loc?.[':@']?.['@file'] !== header) continue;
		if (name.includes('detail::')) continue;

		classes.push({
			kind: attrs['@kind'] === 'struct' ? 'struct' : 'class',
			name,
			signature: `${attrs['@kind']} ${name}`,
			docs: docsOf(children),
			links: {},
			source: sourceLink(children, repo),
			members: membersOf(children, repo),
		});
	}

	return {
		names: new Map(),
		groups: [{ label: 'Classes', items: classes.sort((a, b) => a.name.localeCompare(b.name)) }],
	};
}
