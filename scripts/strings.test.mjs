// Every word the app shows lives in src/lib/strings.ts, so another language is one more file.
// This test reads each .svelte file's markup (its script and style left out, and everything inside
// { } too, where S.<key> is read) and fails on any text or visible attribute that still holds a
// letter. A word found here belongs in strings.ts, used as {S.<key>}.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync, readdirSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const src = join(dirname(fileURLToPath(import.meta.url)), '..', 'src');
const VISIBLE = ['title', 'placeholder', 'aria-label', 'aria-description', 'alt', 'label'];

// Blanks out { ... } expressions, skipping braces inside the quotes of their JavaScript.
function withoutExpressions(text) {
  let out = '', depth = 0, quote = null;
  for (let i = 0; i < text.length; i++) {
    const c = text[i];
    if (depth === 0) { if (c === '{') { depth = 1; out += ' ' } else out += c; continue }
    if (c === '\n') out += '\n';
    if (quote) { if (c === '\\') i++; else if (c === quote) quote = null; continue }
    if (c === '"' || c === "'" || c === '`') quote = c;
    else if (c === '{') depth++;
    else if (c === '}') depth--;
  }
  return out;
}

/** The words a component's markup shows that do not come from strings.ts. */
export function hardWords(svelte) {
  const blank = (m) => m.replace(/[^\n]/g, ' ');
  const markup = withoutExpressions(svelte
    .replace(/<script[\s\S]*?<\/script>/g, blank)
    .replace(/<style[\s\S]*?<\/style>/g, blank)
    .replace(/<!--[\s\S]*?-->/g, blank));
  const found = [];
  const attr = new RegExp(`\\s(${VISIBLE.join('|')})="([^"]*)"`, 'g');
  for (const m of markup.matchAll(attr)) if (/\p{L}/u.test(m[2])) found.push(`${m[1]}="${m[2]}"`);
  for (const line of markup.replace(/<[^>]*>/g, '\n').replace(/&\w+;/g, ' ').split('\n')) {
    if (/\p{L}/u.test(line)) found.push(line.trim());
  }
  return found;
}

test('the check finds a hard-coded word in text and in a visible attribute', () => {
  const sample = `<script>let a = 'Not shown'</script>
<button title="Close it" class="x" onclick={() => f('{')}>{S.ok} Done</button>
<style>.x { content: 'No' }</style><span>&nbsp;{n > 1 ? S.many : S.one}</span>`;
  assert.deepEqual(hardWords(sample), ['title="Close it"', 'Done']);
});

test('every .svelte file takes its words from strings.ts', () => {
  const files = [
    ...readdirSync(src).map((f) => f),
    ...readdirSync(join(src, 'lib')).map((f) => `lib/${f}`),
  ].filter((f) => f.endsWith('.svelte'));
  assert.ok(files.length > 10, `found only ${files.length} .svelte files under ${src}; did they move?`);
  const found = files.flatMap((f) => hardWords(readFileSync(join(src, f), 'utf8')).map((w) => `${f}: ${w}`));
  assert.deepEqual(found, [], 'move each of these words into src/lib/strings.ts and show it as {S.<key>}');
});
