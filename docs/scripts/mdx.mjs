// Escaping helpers shared by the page generators.
//
// Both the registry and the API reference build MDX from text that was written
// for another audience — TOML prose, Rust doc comments, Doxygen descriptions —
// so neither can assume the input is MDX-safe.

/** Escape MDX syntax in a run of prose. */
const escapeProse = (text) =>
	text
		.replace(/<((?:https?|mailto):[^>\s]+)>/g, '[$1]($1)')
		.replace(/\{/g, '\\{')
		.replace(/</g, '&lt;');

/**
 * Make text safe to place in an MDX document body.
 *
 * `{` opens an expression and `<` opens an element, so both have to go from
 * prose — but *not* from code, where markdown does not decode entities and does
 * not honour backslash escapes. Escaping a doc comment wholesale renders its
 * example as `impl Trait for T \{` and its `fznso_<name>_run` as
 * `fznso_&lt;name>_run`, so fenced blocks and code spans are passed through
 * untouched.
 *
 * `<https://…>` autolinks, which the Rust doc comments use heavily, survive as
 * links rather than becoming escaped noise.
 */
export function mdx(text) {
	// Odd indices are the captured code runs; even indices are prose.
	return String(text)
		.split(/(```[\s\S]*?```|``[^`]*``|`[^`\n]*`)/g)
		.map((part, i) => (i % 2 === 1 ? part : escapeProse(part)))
		.join('');
}

/**
 * Make text safe as a JSX attribute value.
 *
 * Signatures carry `extern "C"` and doc comments carry newlines; either would
 * terminate the attribute and leave the rest of the document as broken JSX.
 */
export const attr = (text) =>
	String(text)
		.replace(/\s+/g, ' ')
		.replace(/["\\]/g, '')
		.replace(/\{/g, '(')
		.replace(/</g, '&lt;')
		.trim();

/** Inline code. Backticks inside would end the span. */
export const code = (text) => `\`${String(text).replace(/`/g, '')}\``;
