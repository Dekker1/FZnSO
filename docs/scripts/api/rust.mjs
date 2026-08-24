// rustdoc JSON → the shared API IR.
//
// `cargo rustdoc --output-format json` is a nightly-unstable format, so the
// version is asserted below: a bump fails the build loudly rather than quietly
// producing empty pages.

import { readFile } from 'node:fs/promises';

/** The `format_version` this parser was written against. */
const FORMAT_VERSION = 61;

/** Item kinds we surface as top-level entries, in the order they are listed. */
const TOP_LEVEL = [
	['macro', 'Macros'],
	['trait', 'Traits'],
	['struct', 'Structs'],
	['enum', 'Enums'],
	['function', 'Functions'],
	['type_alias', 'Type aliases'],
	['constant', 'Constants'],
];

/** Render a rustdoc `Type` as source-like text. */
function type(t) {
	if (t === null || t === undefined) return '_';
	if (typeof t === 'string') return t;
	const [kind, v] = Object.entries(t)[0];
	switch (kind) {
		case 'primitive':
		case 'generic':
			return v;
		case 'resolved_path': {
			const args = genericArgs(v.args);
			return v.path + args;
		}
		case 'borrowed_ref': {
			const lt = v.lifetime ? `${v.lifetime} ` : '';
			return `&${lt}${v.is_mutable ? 'mut ' : ''}${type(v.type)}`;
		}
		case 'raw_pointer':
			return `*${v.is_mutable ? 'mut' : 'const'} ${type(v.type)}`;
		case 'tuple':
			return `(${v.map(type).join(', ')})`;
		case 'slice':
			return `[${type(v)}]`;
		case 'array':
			return `[${type(v.type)}; ${v.len}]`;
		case 'function_pointer': {
			const unsafe_ = v.header?.is_unsafe ? 'unsafe ' : '';
			const abi = v.header?.abi && v.header.abi !== 'Rust' ? abiName(v.header.abi) : '';
			const inputs = v.sig.inputs.map(([n, t]) => (n && n !== '_' ? `${n}: ${type(t)}` : type(t)));
			const out = v.sig.output ? ` -> ${type(v.sig.output)}` : '';
			return `${unsafe_}${abi}fn(${inputs.join(', ')})${out}`;
		}
		case 'impl_trait':
			return `impl ${v.map(bound).filter(Boolean).join(' + ')}`;
		case 'dyn_trait':
			return `dyn ${v.traits.map((tr) => tr.trait.path).join(' + ')}`;
		case 'qualified_path':
			return `<${type(v.self_type)} as ${v.trait?.path ?? '_'}>::${v.name}`;
		case 'infer':
			return '_';
		default:
			return '_';
	}
}

function abiName(abi) {
	if (typeof abi === 'string') return `extern "${abi}" `;
	const name = Object.keys(abi)[0];
	return name === 'Rust' ? '' : `extern "${name.toUpperCase() === 'C' ? 'C' : name}" `;
}

function bound(b) {
	if (b.trait_bound) return b.trait_bound.trait.path;
	if (b.outlives) return b.outlives;
	return '';
}

function genericArgs(args) {
	if (!args?.angle_bracketed) return '';
	const parts = [];
	for (const a of args.angle_bracketed.args ?? []) {
		if (a.lifetime) parts.push(a.lifetime);
		else if (a.type) parts.push(type(a.type));
		else if (a.const) parts.push(a.const.expr);
	}
	return parts.length ? `<${parts.join(', ')}>` : '';
}

function generics(g) {
	const params = (g?.params ?? [])
		.filter((p) => !p.name.startsWith('impl ') && p.kind?.type?.synthetic !== true)
		.map((p) => p.name);
	return params.length ? `<${params.join(', ')}>` : '';
}

function fnSignature(name, inner) {
	const { sig, header } = inner;
	const parts = [];
	if (header?.is_const) parts.push('const');
	if (header?.is_unsafe) parts.push('unsafe');
	if (header?.abi && header.abi !== 'Rust') parts.push(abiName(header.abi).trim());
	const inputs = sig.inputs.map(([n, t]) => {
		if (n === 'self') {
			const s = type(t);
			return s === 'Self' ? 'self' : s.replace(/Self$/, 'self');
		}
		return `${n}: ${type(t)}`;
	});
	const out = sig.output ? ` -> ${type(sig.output)}` : '';
	return `${parts.join(' ')}${parts.length ? ' ' : ''}fn ${name}${generics(inner.generics)}(${inputs.join(', ')})${out}`;
}

/**
 * Methods reachable on a type, from its inherent impls.
 *
 * Trait impls are skipped: `Debug`, `Clone` and friends would bury the handful
 * of methods that actually say something, and the trait itself documents them.
 */
function methodsOf(item, index) {
	const inner = Object.values(item.inner)[0];
	const out = [];
	for (const implId of inner.impls ?? []) {
		const impl = index[implId];
		if (!impl || impl.inner.impl?.trait) continue;
		for (const id of impl.inner.impl.items ?? []) {
			const m = index[id];
			if (!m?.name || !m.inner.function) continue;
			out.push({
				kind: 'method',
				name: m.name,
				signature: fnSignature(m.name, m.inner.function),
				docs: m.docs ?? '',
				links: m.links ?? {},
			});
		}
	}
	return out.sort((a, b) => a.name.localeCompare(b.name));
}

function membersOf(item, index) {
	const [kind, inner] = Object.entries(item.inner)[0];

	if (kind === 'struct') {
		const ids = inner.kind?.plain?.fields ?? [];
		const fields = ids
			.map((id) => index[id])
			.filter((f) => f?.name)
			.map((f) => ({
				kind: 'field',
				name: f.name,
				signature: `${f.name}: ${type(Object.values(f.inner)[0])}`,
				docs: f.docs ?? '',
				links: f.links ?? {},
			}));
		return [...fields, ...methodsOf(item, index)];
	}

	if (kind === 'enum') {
		const variants = (inner.variants ?? [])
			.map((id) => index[id])
			.filter((v) => v?.name)
			.map((v) => ({
				kind: 'variant',
				name: v.name,
				signature: v.name,
				docs: v.docs ?? '',
				links: v.links ?? {},
			}));
		return [...variants, ...methodsOf(item, index)];
	}

	if (kind === 'trait') {
		return (inner.items ?? [])
			.map((id) => index[id])
			.filter((m) => m?.name)
			.map((m) => ({
				kind: m.inner.function ? 'method' : 'assoc',
				name: m.name,
				signature: m.inner.function
					? fnSignature(m.name, m.inner.function)
					: `type ${m.name}`,
				docs: m.docs ?? '',
				links: m.links ?? {},
			}));
	}

	return [];
}

function signatureOf(item) {
	const [kind, inner] = Object.entries(item.inner)[0];
	switch (kind) {
		case 'struct':
			return `struct ${item.name}${generics(inner.generics)}`;
		case 'enum':
			return `enum ${item.name}${generics(inner.generics)}`;
		case 'trait':
			return `trait ${item.name}${generics(inner.generics)}`;
		case 'function':
			return fnSignature(item.name, inner);
		case 'type_alias':
			return `type ${item.name}${generics(inner.generics)} = ${type(inner.type)}`;
		case 'constant':
			return `const ${item.name}: ${type(inner.type)}`;
		case 'macro':
			return `macro_rules! ${item.name}`;
		default:
			return item.name;
	}
}

/** Parse one crate's rustdoc JSON into IR groups. */
export async function parseRust(jsonPath, { repo, sourceRoot }) {
	const doc = JSON.parse(await readFile(jsonPath, 'utf8'));
	if (doc.format_version !== FORMAT_VERSION) {
		throw new Error(
			`${jsonPath}: rustdoc format_version ${doc.format_version}, expected ` +
				`${FORMAT_VERSION}. The nightly format changed — update ` +
				`docs/scripts/api/rust.mjs before trusting its output.`,
		);
	}
	const index = doc.index;

	// `id -> name` so intra-doc links in doc comments can become real anchors.
	const names = new Map();
	for (const [id, item] of Object.entries(index)) {
		if (item.name) names.set(Number(id), item.name);
	}

	// Walk down from the root module. Scanning the whole index instead would
	// pull in every impl method as a top-level function, since a method is a
	// `function` item too — it is module membership that makes something
	// top-level, not its kind.
	const topLevelIds = [];
	const seen = new Set();
	const queue = [doc.root];
	while (queue.length) {
		const id = queue.pop();
		if (seen.has(id)) continue;
		seen.add(id);
		const item = index[id];
		if (!item) continue;
		if (item.inner.module) {
			queue.push(...(item.inner.module.items ?? []));
			continue;
		}
		// `fznso` presents a flat API built from `pub use` of its private
		// modules, so a re-export has to be followed to reach the real item.
		if (item.inner.use) {
			if (item.inner.use.id !== null && item.inner.use.id !== undefined) {
				queue.push(item.inner.use.id);
			}
			continue;
		}
		topLevelIds.push(id);
	}

	const byKind = new Map();
	for (const id of topLevelIds) {
		const item = index[id];
		if (item.crate_id !== 0 || !item.name || item.visibility !== 'public') continue;
		const kind = Object.keys(item.inner)[0];
		if (!TOP_LEVEL.some(([k]) => k === kind)) continue;
		if (!byKind.has(kind)) byKind.set(kind, []);
		byKind.get(kind).push({
			kind,
			name: item.name,
			signature: signatureOf(item),
			docs: item.docs ?? '',
			links: item.links ?? {},
			source:
				item.span && repo
					? `${repo}/blob/main/${item.span.filename.replace(sourceRoot ?? '', '')}#L${item.span.begin[0]}`
					: null,
			members: membersOf(item, index),
		});
	}

	return {
		names,
		groups: TOP_LEVEL.filter(([kind]) => byKind.has(kind)).map(([kind, label]) => ({
			label,
			items: byKind.get(kind).sort((a, b) => a.name.localeCompare(b.name)),
		})),
	};
}
