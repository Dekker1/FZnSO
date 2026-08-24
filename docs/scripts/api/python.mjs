// griffe JSON → the shared API IR.
//
// The bindings are a compiled pyo3 extension, so griffe is run with
// `--force-inspection`: there is no Python source to read statically.

import { readFile } from 'node:fs/promises';

/** Dunders worth showing, mapped to how they are labelled. */
const KEEP_DUNDERS = new Map([
	['__new__', 'constructor'],
	['__init__', 'constructor'],
]);

function signature(obj, name) {
	const params = (obj.parameters ?? [])
		.filter((p) => p.name !== 'self' && p.name !== 'cls')
		.map((p) => {
			const annotated = p.annotation ? `${p.name}: ${p.annotation}` : p.name;
			return p.default ? `${annotated} = ${p.default}` : annotated;
		});
	const returns = obj.returns ? ` -> ${obj.returns}` : '';
	return `${name}(${params.join(', ')})${returns}`;
}

const docsOf = (obj) => obj.docstring?.value ?? '';

/** Resolve a dotted `target_path` from the dump root, following aliases. */
function resolve(dump, path) {
	const [pkg, ...rest] = path.split('.');
	let node = dump[pkg];
	for (const part of rest) {
		node = node?.members?.[part];
		if (!node) return null;
	}
	return node?.kind === 'alias' ? resolve(dump, node.target_path) : node;
}

/**
 * A pyo3 constructor that inspection could not recover a signature for.
 *
 * `#[pyo3(signature = ...)]` does not reach `__new__` through dynamic
 * inspection, so these arrive as a bare `(*args, **kwargs)` carrying CPython's
 * stock docstring. Showing that is worse than showing nothing — the class's own
 * documentation says how to construct it.
 */
const isOpaqueConstructor = (obj) =>
	(obj.parameters ?? []).length > 0 &&
	(obj.parameters ?? []).every((p) => p.kind?.startsWith('variadic'));

function membersOf(cls) {
	const out = [];
	for (const [name, member] of Object.entries(cls.members ?? {})) {
		const special = KEEP_DUNDERS.get(name);
		if (name.startsWith('__') && !special) continue;
		const labels = member.labels ?? [];
		if (member.kind === 'attribute') {
			out.push({
				kind: labels.includes('property') ? 'property' : 'attribute',
				name,
				signature: name,
				docs: docsOf(member),
				links: {},
			});
		} else if (member.kind === 'function') {
			if (special && isOpaqueConstructor(member)) continue;
			out.push({
				kind: special ?? (labels.includes('staticmethod') ? 'static method' : 'method'),
				name: special ? cls.name : name,
				signature: signature(member, special ? cls.name : name),
				docs: docsOf(member),
				links: {},
			});
		}
	}
	// Constructor first, then properties, then methods — the order someone reads
	// a class in.
	const rank = (m) =>
		m.kind === 'constructor' ? 0 : m.kind === 'property' || m.kind === 'attribute' ? 1 : 2;
	return out.sort((a, b) => rank(a) - rank(b) || a.name.localeCompare(b.name));
}

/** Parse a `griffe dump` file into IR groups. */
export async function parsePython(jsonPath, { packageName = 'fznso', repo, sourceFile } = {}) {
	const dump = JSON.parse(await readFile(jsonPath, 'utf8'));
	const root = dump[packageName];
	if (!root) throw new Error(`${jsonPath}: no package \`${packageName}\` in the griffe dump`);

	const source = repo && sourceFile ? `${repo}/blob/main/${sourceFile}` : null;
	const classes = [];
	const functions = [];

	for (const [name, raw] of Object.entries(root.members ?? {})) {
		if (name.startsWith('__')) continue;
		const member = raw.kind === 'alias' ? resolve(dump, raw.target_path) : raw;
		if (!member) continue;
		if (member.kind === 'class') {
			classes.push({
				kind: 'class',
				name,
				signature: `class ${name}`,
				docs: docsOf(member),
				links: {},
				source,
				members: membersOf({ ...member, name }),
			});
		} else if (member.kind === 'function') {
			functions.push({
				kind: 'function',
				name,
				signature: signature(member, name),
				docs: docsOf(member),
				links: {},
				source,
				members: [],
			});
		}
	}

	const byName = (a, b) => a.name.localeCompare(b.name);
	return {
		names: new Map(),
		groups: [
			{ label: 'Classes', items: classes.sort(byName) },
			{ label: 'Functions', items: functions.sort(byName) },
		].filter((g) => g.items.length),
	};
}
