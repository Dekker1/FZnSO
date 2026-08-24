// Builds the reference section from the four API surfaces.
//
// Each toolchain emits a machine-readable dump — rustdoc JSON, Doxygen XML,
// griffe JSON — which is parsed into one shared shape and rendered as ordinary
// Starlight pages. The reference is therefore part of the site rather than a
// second site linked from it: same theme, same search, same table of contents,
// and anchors the specification can link straight into.
//
// Inputs are produced by `just docs-api`. A missing one is skipped with a
// warning rather than failing, so a contributor without Doxygen or a Python
// toolchain can still work on the prose.

import { access } from 'node:fs/promises';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { parseCpp } from './api/cpp.mjs';
import { parsePython } from './api/python.mjs';
import { indexSymbols, renderPage } from './api/render.mjs';
import { parseRust } from './api/rust.mjs';

const ROOT = dirname(dirname(fileURLToPath(import.meta.url)));
const REPO_ROOT = dirname(ROOT);
const OUT = join(ROOT, 'src', 'content', 'docs', 'reference');
const REPO = 'https://github.com/Dekker1/FZnSO';

const PAGES = [
	{
		slug: 'abi-types',
		title: 'ABI types',
		description: 'The types that cross the FZnSO interface, generated from the fznso-types crate.',
		noun: 'type',
		order: 1,
		source: join(REPO_ROOT, 'target/doc/fznso_types.json'),
		parse: (path) => parseRust(path, { repo: REPO }),
		intro: `
These are the types that cross the interface, and they are the **same declarations a C program
sees**: \`c/fznso_types.h\` is generated from this crate by cbindgen, so \`FznsoModelMethods\` here and
\`FznsoModelMethods\` in the header are one thing described once.

Read [Types & values](../../spec/types-and-values/) for what they mean, and
[Lifetimes & ownership](../../spec/lifetimes-and-ownership/) for the rules that govern all of them.
`,
	},
	{
		slug: 'abi-entry-points',
		title: 'ABI entry points',
		description: 'The thirteen symbols a solver library exports, generated from the solver template.',
		noun: 'entry point',
		order: 2,
		source: join(REPO_ROOT, 'target/doc/fznso_solver_template.json'),
		parse: (path) => parseRust(path, { repo: REPO }),
		intro: `
The thirteen entry points every solver exports, with the contract on each.

\`NAME\` is a placeholder: a real solver substitutes its own
[name](../../spec/naming-and-versioning/), so these are \`fznso_gecode_solver_run\` and friends in a
library named \`gecode\`. See [Entry points](../../spec/entry-points/) for what each is for, and
\`c/fznso_solver_template.c\` for the C template to copy.
`,
	},
	{
		slug: 'rust',
		title: 'Rust bindings',
		description: 'The fznso crate: both sides of the interface, from Rust.',
		noun: 'item',
		order: 3,
		source: join(REPO_ROOT, 'target/doc/fznso.json'),
		parse: (path) => parseRust(path, { repo: REPO }),
		intro: `
The \`fznso\` crate covers both sides: [\`Library\`](#library) loads a solver and drives it, and
[\`fznso_export\`](#fznso_export) exposes one you have written.

See [Using a solver](../../guides/using-a-solver/) and
[Implementing a solver](../../guides/implementing-a-solver/) for the walkthroughs.
`,
	},
	{
		slug: 'cpp',
		title: 'C++ bindings',
		description: 'fznso.hpp — building models and loading solvers from C++.',
		noun: 'class',
		order: 4,
		source: join(REPO_ROOT, 'target/doxygen/xml'),
		parse: (path) => parseCpp(path, { repo: REPO, header: 'cpp/fznso.hpp' }),
		intro: `
Header-only C++17 bindings for the consumer side: \`#include "fznso.hpp"\`, add \`c/\` and \`cpp/\` to
your include path, and link nothing.

Implementing a solver in C++ is [\`fznso_export.hpp\`](../cpp-export/) instead.
`,
	},
	{
		slug: 'cpp-export',
		title: 'C++ solver export',
		description: 'fznso_export.hpp — implementing a solver in C++.',
		noun: 'class',
		order: 5,
		source: join(REPO_ROOT, 'target/doxygen/xml'),
		parse: (path) => parseCpp(path, { repo: REPO, header: 'cpp/fznso_export.hpp' }),
		intro: `
Subclass [\`fznso::Solver\`](#fznsosolver) and expand \`FZNSO_EXPORT_SOLVER(YourSolver, yoursolver)\`
once; the macro writes all thirteen entry points.

The consumer side is [\`fznso.hpp\`](../cpp/).
`,
	},
	{
		slug: 'python',
		title: 'Python bindings',
		description: 'The fznso Python module.',
		noun: 'item',
		order: 6,
		source: '/tmp/fznso-griffe.json',
		parse: (path) =>
			parsePython(path, { repo: REPO, sourceFile: 'python/src/lib.rs' }),
		intro: `
The \`fznso\` module, built with maturin from \`python/src/lib.rs\`.

Values are ordinary Python objects: a \`range\` or \`int_set\` for a domain, an \`int\`, a
[\`Decision\`](#decision), a list. See [Using a solver](../../guides/using-a-solver/).
`,
	},
];

const exists = async (path) => {
	try {
		await access(path);
		return true;
	} catch {
		return false;
	}
};

// Two passes: every page is parsed before any is rendered, so that a doc comment
// in one crate can resolve a cross-reference into another.
const parsed = [];
for (const page of PAGES) {
	if (!(await exists(page.source))) {
		console.warn(`api: ${page.slug} — skipped, ${page.source} not built`);
		continue;
	}
	const { groups } = await page.parse(page.source);
	parsed.push({ ...page, groups });
}

const symbols = new Map();
for (const page of parsed) indexSymbols(page, symbols);

for (const page of parsed) {
	const n = await renderPage(page, symbols, OUT);
	console.log(`api: ${page.slug} — ${n} items`);
}
console.log(`api: ${symbols.size} symbols indexed for cross-references`);
