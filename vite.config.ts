import { svelte } from '@sveltejs/vite-plugin-svelte'
import { Marked } from 'marked'
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs'
import { dirname, join, relative, resolve } from 'node:path'
import { defineConfig, type Plugin } from 'vite'

/** GitHub's anchor for a heading, so the README's own links work on GitHub and in the app. */
const slug = (t: string) => t.toLowerCase().replace(/<[^>]+>/g, '').replace(/[^\p{L}\p{N}\s-]/gu, '').trim().replace(/\s/g, '-')

/** `virtual:readme`: README.md made into HTML at build time (no network, one source), its chapters,
 * and its pictures as the page's own files. */
function readme(): Plugin {
  const file = resolve('README.md')
  return {
    name: 'fvh-readme',
    resolveId: id => (id === 'virtual:readme' ? '\0virtual:readme' : null),
    load(id) {
      if (id !== '\0virtual:readme') return null
      this.addWatchFile(file)
      const chapters: { id: string; text: string }[] = []
      const pictures: string[] = []
      const md = new Marked({
        renderer: {
          heading({ tokens, depth }) {
            const html = this.parser.parseInline(tokens), id = slug(html)
            if (depth === 2) chapters.push({ id, text: html.replace(/<[^>]+>/g, '') })
            return `<h${depth} id="${id}">${html}</h${depth}>\n`
          },
          image({ href, text }) {
            pictures.push('/' + relative(dirname(file), resolve(dirname(file), href)).replaceAll('\\', '/'))
            return `<img src="__PICTURE_${pictures.length - 1}__" alt="${text.replaceAll('"', '&quot;')}">`
          },
        },
      })
      const html = md.parse(readFileSync(file, 'utf8')) as string
      return pictures.map((p, i) => `import p${i} from ${JSON.stringify(p)}\n`).join('') +
        `export const chapters = ${JSON.stringify(chapters)}\n` +
        `export default ${JSON.stringify(html)}${pictures.map((_, i) => `.replace("__PICTURE_${i}__", p${i})`).join('')}\n`
    },
  }
}

/** Writes which npm packages end up in the built page, for scripts/notices.mjs. */
function bundled(): Plugin {
  return {
    name: 'fvh-bundled',
    apply: 'build',
    generateBundle(_, bundle) {
      const ids = [...this.getModuleIds()].map(String)
      for (const f of Object.values(bundle)) if (f.type === 'asset') ids.push(...(f.originalFileNames ?? []))
      const names = new Set<string>()
      for (const id of ids) {
        const m = id.replaceAll('\\', '/').match(/.*node_modules\/((?:@[^/]+\/)?[^/]+)/)
        if (m) names.add(m[1])
      }
      const out = join('node_modules', '.cache', 'fvh', 'bundled.json')
      mkdirSync(dirname(out), { recursive: true })
      writeFileSync(out, JSON.stringify([...names].sort(), null, 1) + '\n')
    },
  }
}

// Tauri loads the dev server at a fixed port and shows the Rust build's own errors.
export default defineConfig({
  plugins: [svelte(), readme(), bundled()],
  clearScreen: false,
  server: { port: 5173, strictPort: true },
})
