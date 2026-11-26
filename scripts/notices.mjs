// Makes tools/notices.json and tools/THIRD-PARTY-NOTICES.txt: every part the package carries that
// is not the app's own code, with its version, licence, the notices its licence asks to ship and
// where its source is. Settings, Licences shows the first; the package ships both.
// Run from the repo root after `npm run fetch-tools`: `node scripts/notices.mjs [--appdir <AppDir> --runtime <commit>]`.
// `npm run package` runs it. It builds the page once, to learn which npm packages are in it;
// `--no-build` takes the list the last build left.
//
// The parts: the tools (tools.json; FFmpeg's libraries from scripts/ffmpeg-licences.json), the Rust
// crates in the app for this system (`cargo metadata`), Rust's standard library, the npm packages
// in the page, and in the Linux AppImage each system library linuxdeploy put in it (with the Debian
// copyright file it copied, the Ubuntu package and version from dpkg) and the AppImage's own runtime.
// scripts/licences/ holds the texts a part does not ship itself.
import { execFileSync, execSync } from 'node:child_process'
import { createHash } from 'node:crypto'
import { existsSync, readdirSync, readFileSync, statSync, writeFileSync } from 'node:fs'
import { basename, dirname, join, relative } from 'node:path'

const TOOLS = 'tools'
const LICENCES = join('scripts', 'licences')
const arg = (n) => { const i = process.argv.indexOf(n); return i > 0 ? process.argv[i + 1] : undefined }
const appdir = arg('--appdir')
const system = appdir || process.platform === 'linux' ? 'linux' : process.platform === 'darwin' ? 'macos' : 'windows'
const https = (u) => /^https:\/\//.test(u)
const LICENCE = /^(licen[cs]e|copying|copyright|notice|unlicense|patents)([._-].*)?$/i
const isLicence = (f) => LICENCE.test(f) && !/\.(cfg|py|sh|cmake|in|am|c|h|rs|json|ya?ml|toml)$/i.test(f)
const read = (f) => readFileSync(f, 'utf8').replace(/\r\n/g, '\n')

if (!existsSync(join(TOOLS, 'tools.json'))) { console.error('No tools: run `npm run fetch-tools` first.'); process.exit(1) }

// Each text is kept once and named by its hash; an item lists the files it ships.
const texts = {}
const keep = (t) => { const id = createHash('sha256').update(t).digest('hex').slice(0, 16); texts[id] = t; return id }
const files = (dir, names) => names.map(n => ({ file: n, text: keep(read(join(dir, n))) }))
const licenceFiles = (dir) => readdirSync(dir).filter(f => isLicence(f) && statSync(join(dir, f)).isFile()).sort()
// A short name for a licence text, for parts that do not name their licence; the text decides.
const KNOWN = [
  [/GNU LESSER GENERAL PUBLIC LICENSE\s+Version 2\.1/, 'LGPL 2.1'], [/GNU LESSER GENERAL PUBLIC LICENSE\s+Version 3/, 'LGPL 3'],
  [/GNU LIBRARY GENERAL PUBLIC LICENSE/, 'LGPL 2'], [/GNU GENERAL PUBLIC LICENSE\s+Version 2/, 'GPL 2'], [/GNU GENERAL PUBLIC LICENSE\s+Version 3/, 'GPL 3'],
  [/Apache License,?\s+Version 2\.0/, 'Apache 2.0'], [/Mozilla Public License,? [Vv]ersion 2\.0/, 'MPL 2.0'],
  [/Permission is hereby granted, free of charge/, 'MIT'], [/Redistribution and use in source and binary forms/, 'BSD'],
  [/Permission to use, copy, modify, and(\/or)? distribute this software for any purpose/, 'ISC'],
  [/provided 'as-is', without any express or implied/, 'zlib'], [/Boost Software License/, 'Boost 1.0'],
  [/FreeType Project LICENSE|The FreeType Project/, 'FreeType'], [/UNICODE LICENSE|Unicode, Inc\./, 'Unicode'],
  [/SIL OPEN FONT LICENSE/i, 'OFL 1.1'], [/libpng license|PNG Reference Library License/i, 'libpng'],
]
const guess = (ids) => [...new Set(ids.flatMap(id => KNOWN.filter(([re]) => re.test(texts[id])).map(([, n]) => n)))].join(', ')
const template = (name, who) => read(join(LICENCES, name)).replace('{who}', who)

// The tools, as fetch-tools left them; FFmpeg's build links in other libraries (none on macOS).
const manifest = JSON.parse(read(join(TOOLS, 'tools.json')))
const bundled = manifest.tools.filter(t => !t.system)
const item = (t) => ({ name: t.name, version: t.version, licence: t.licence, sources: t.sources ?? [{ what: 'Source', url: t.source }],
  texts: (t.licences ?? []).map(f => ({ file: f, text: keep(read(join(TOOLS, f))) })) })
const tools = bundled.map(item)
// The MSYS2 packages whose DLLs ddjvu loads, each a part of its own.
const msys2 = bundled.filter(t => t.id !== 'ffmpeg').flatMap(t => t.packages ?? []).map(item)
for (const p of [...tools, ...msys2]) if (!p.texts.length) throw new Error(`${p.name} has no licence file in tools.json`)
// The app's own source, which the AppImage carries since a GPL 2 library (libjbig) is loaded into
// the app there; package.mjs puts it in tools/source/ before the first notices of a build.
const appSource = system === 'linux' && existsSync(join(TOOLS, 'source')) ? readdirSync(join(TOOLS, 'source')).filter(f => f.endsWith('.tar.gz')) : []
if (appSource.length > 1 || (appdir && !appSource.length)) throw new Error('The AppImage needs the app\'s source, one tar.gz, in tools/source/ (package.mjs makes it).')
// Every file in the tools folder belongs to a part: a program, a library, a licence or a source.
const listed = new Set(['tools.json', 'notices.json', 'THIRD-PARTY-NOTICES.txt', ...appSource.map(f => `source/${f}`),
  ...bundled.flatMap(t => [t, ...(t.packages ?? [])]).flatMap(t => [...(t.files ?? []), ...(t.ships ?? []), ...(t.licences ?? []),
    ...(t.sources ?? []).map(x => x.url).filter(u => !https(u))])])
const unlisted = readdirSync(TOOLS, { recursive: true, withFileTypes: true }).filter(f => f.isFile())
  .map(f => relative(TOOLS, join(f.parentPath, f.name)).replaceAll('\\', '/')).filter(f => !f.startsWith('.download/') && !listed.has(f))
if (unlisted.length) throw new Error(`These files in tools/ belong to no part in tools.json:\n${unlisted.join('\n')}`)
const parts = JSON.parse(read(join('scripts', 'ffmpeg-licences.json')))
// A link to the source at that commit, where the host has one.
const at = (repo, rev) => {
  const r = repo.replace(/\.git$/, '')
  return /github\.com/.test(r) ? `${r}/tree/${rev}` : /gitlab|code\.videolan\.org/.test(r) ? `${r}/-/tree/${rev}`
    : /googlesource\.com/.test(r) ? `${r}/+/${rev}` : /svn\./.test(r) ? `${r}?p=${rev}`
    : /git\.savannah\.gnu\.org\/git\//.test(r) ? `${r.replace('/git/', '/cgit/')}.git/tree/?id=${rev}`
    : /git\.code\.sf\.net\/p\//.test(r) ? `${r.replace('git.code.sf.net/p/', 'sourceforge.net/p/')}/ci/${rev}/tree/` : r
}
// On macOS FFmpeg's libraries are the packages its tools.json entry lists (dav1d), built by fetch-tools.
const inFfmpeg = system === 'macos' ? (bundled.find(t => t.id === 'ffmpeg')?.packages ?? []).map(item) : parts.parts.filter(p => p.systems.includes(system)).map(p => {
  for (const [f, id] of Object.entries(p.files)) if (keep(parts.texts[id]) !== id) throw new Error(`${p.name}: ${f} in ffmpeg-licences.json is not the text it names`)
  if (!Object.keys(p.files).length) throw new Error(`${p.name} has no licence file in ffmpeg-licences.json`)
  return { name: p.name, version: p.rev, licence: guess(Object.values(p.files)),
    sources: [{ what: 'Source', url: at(p.repo, p.rev) }, ...(at(p.repo, p.rev) === p.repo.replace(/\.git$/, '') ? [] : [{ what: 'Repository', url: p.repo }])],
    texts: Object.entries(p.files).map(([file, text]) => ({ file, text })) }
})
// Each library the shipped FFmpeg says it was built with must be one of those parts.
const ffmpegTool = bundled.find(t => t.id === 'ffmpeg')
const ffprobe = join(TOOLS, ffmpegTool.files.find(f => /ffprobe/.test(f)))
const conf = execFileSync(ffprobe, ['-hide_banner', '-buildconf'], { encoding: 'utf8' })
const ALIAS = { lzma: 'xz', libvpl: 'onevpl', libopencoreamrnb: 'opencoreamr', libopencoreamrwb: 'opencoreamr', lv2: 'lilv', vaapi: 'libva',
  libaom: 'aom', libdav1d: 'dav1d', libjxl: 'libjxl', libsvtav1: 'svtav1', librav1e: 'rav1e', libzimg: 'zimg', libsoxr: 'soxr', libvvenc: 'vvenc',
  libuavs3d: 'uavs3d', libkvazaar: 'kvazaar', libtwolame: 'twolame', libsnappy: 'snappy', libsrt: 'srt', libgme: 'gme', libzvbi: 'zvbi',
  libopenmpt: 'openmpt', libopenjpeg: 'openjpeg', libopenh264: 'openh264', liboapv: 'openapv', liblcevcdec: 'lcevcdec', libvmaf: 'vmaf',
  libfreetype: 'freetype', libfribidi: 'fribidi', libharfbuzz: 'harfbuzz', libxml2: 'libxml2', libzmq: 'libzmq', sdl2: 'sdl', ffnvcodec: 'ffnvcodec',
  cudallvm: 'ffnvcodec', libmp3lame: 'libmp3lame', libvorbis: 'libvorbis', libtheora: 'libtheora', libopus: 'libopus', libwebp: 'libwebp',
  libvpx: 'libvpx', librsvg: 'librsvg', libass: 'libass', libbluray: 'libbluray', libplacebo: 'libplacebo', librist: 'librist', libssh: 'libssh',
  libaribb24: 'libaribb24', libaribcaption: 'libaribcaption', libxcb: 'libxcb', libpulse: 'pulseaudio', libdrm: 'libdrm', iconv: 'libiconv' }
const SYSTEM = new Set(['version3', 'shared', 'pthreads', 'schannel', 'xlib', 'vulkan', 'opencl', 'amf', 'gmp', 'zlib', 'fontconfig', 'lcms2', 'chromaprint',
  'openal', 'vapoursynth', 'openssl'])
const norm = (n) => n.toLowerCase().replace(/[^a-z0-9]/g, '')
const names = new Set(inFfmpeg.map(p => norm(p.name)))
const builtWith = [...conf.matchAll(/--enable-([\w-]+)/g)].map(m => norm(m[1]))
const unknown = builtWith.filter(f => !names.has(ALIAS[f] ?? f) && !names.has(f.replace(/^lib/, '')) && !(SYSTEM.has(f) && [...names].some(n => n.includes(f))))
  .filter(f => !['version3', 'shared', 'pthreads', 'schannel', 'xlib', 'cudallvm'].includes(f))
  // The macOS FFmpeg is built by fetch-tools with only these, the system's own (`FFMPEG_CONFIGURE`).
  .filter(f => !(system === 'macos' && ['static', 'zlib', 'bzlib'].includes(f)))
if (unknown.length) throw new Error(`FFmpeg was built with ${unknown.join(', ')}, which no part in ffmpeg-licences.json names; add it to ALIAS in notices.mjs`)

// Rust: every crate the app is built from for this system, but not those used only by tests.
const host = execFileSync('rustc', ['-vV'], { encoding: 'utf8' }).match(/^host: (.*)$/m)[1]
const meta = JSON.parse(execFileSync('cargo', ['metadata', '--format-version', '1', '--locked', '--filter-platform', host],
  { cwd: 'src-tauri', encoding: 'utf8', maxBuffer: 256 << 20 }))
const byId = new Map(meta.packages.map(p => [p.id, p]))
const nodes = new Map(meta.resolve.nodes.map(n => [n.id, n]))
const used = new Set(), todo = [meta.resolve.root]
while (todo.length) {
  const id = todo.pop()
  if (used.has(id)) continue
  used.add(id)
  for (const d of nodes.get(id).deps) if (d.dep_kinds.some(k => k.kind !== 'dev')) todo.push(d.pkg)
}
used.delete(meta.resolve.root)
const crates = [...used].map(id => byId.get(id)).sort((a, b) => a.name.localeCompare(b.name) || a.version.localeCompare(b.version))
const own = (p) => licenceFiles(dirname(p.manifest_path))
// Crates that ship no licence file, whose repository has one.
const REPO_LICENCES = { 'https://github.com/wravery/webview2-rs': 'webview2-rs.txt' }
const crateTexts = (p) => {
  const dir = dirname(p.manifest_path), mine = own(p)
  if (mine.length) return files(dir, mine)
  // No licence file in the crate: one from the same repository, else the licence's own text.
  const sib = crates.find(q => q !== p && q.repository && q.repository === p.repository && q.license === p.license && own(q).length)
  if (sib) return files(dirname(sib.manifest_path), own(sib)).map(f => ({ ...f, file: `${f.file} (from ${sib.name})` }))
  if (REPO_LICENCES[p.repository]) return [{ file: `LICENSE (of ${p.repository})`, text: keep(read(join(LICENCES, REPO_LICENCES[p.repository]))) }]
  const id = (p.license ?? '').split(/\s+OR\s+|\//)[0].trim()
  const who = p.authors?.length ? p.authors.join(', ') : `the ${p.name} authors`
  if (existsSync(join(LICENCES, `${id}.txt`))) return [{ file: `${id} licence`, text: keep(template(`${id}.txt`, who)) }]
  const same = crates.find(q => q.license === id && own(q).length)
  if (same) return files(dirname(same.manifest_path), own(same)).map(f => ({ ...f, file: `${id} licence` }))
  throw new Error(`${p.name} ${p.version} has no licence text; add scripts/licences/${id}.txt`)
}
const rust = crates.map(p => ({
  name: p.name, version: p.version, licence: p.license ?? p.license_file ?? '',
  sources: [{ what: 'Source', url: `https://crates.io/crates/${p.name}/${p.version}` }], texts: crateTexts(p),
}))
const loader = crates.find(p => p.name === 'webview2-com-sys')
if (loader) rust.push({ name: 'Microsoft WebView2 loader', version: `in webview2-com-sys ${loader.version}`, licence: 'BSD 3-Clause',
  sources: [{ what: 'The WebView2 SDK', url: 'https://www.nuget.org/packages/Microsoft.Web.WebView2' }],
  texts: [{ file: 'LICENSE.txt', text: keep(read(join(LICENCES, 'webview2-loader.txt'))) }] })
// The standard library is built into the app too.
const sysroot = execFileSync('rustc', ['--print', 'sysroot'], { encoding: 'utf8' }).trim()
const rustDoc = join(sysroot, 'share', 'doc', 'rust')
const rustc = execFileSync('rustc', ['-V'], { encoding: 'utf8' }).trim().split(' ')[1]
const plain = (html) => html.replace(/<(script|style)[\s\S]*?<\/\1>/g, '').replace(/<br\s*\/?>|<\/(p|li|h\d|tr|div)>/g, '\n')
  .replace(/<[^>]+>/g, '').replace(/&lt;/g, '<').replace(/&gt;/g, '>').replace(/&quot;/g, '"').replace(/&#39;/g, "'").replace(/&amp;/g, '&')
  .replace(/\n\s*\n\s*\n+/g, '\n\n').trim() + '\n'
rust.unshift({ name: 'Rust standard library', version: rustc, licence: 'MIT OR Apache-2.0',
  sources: [{ what: 'Source', url: `https://github.com/rust-lang/rust/tree/${rustc}/library` }],
  texts: [
    ...(existsSync(join(rustDoc, 'COPYRIGHT-library.html')) ? [{ file: 'COPYRIGHT-library', text: keep(plain(read(join(rustDoc, 'COPYRIGHT-library.html')))) }] : []),
    ...['MIT.txt', 'Apache-2.0.txt'].filter(f => existsSync(join(rustDoc, 'licenses', f))).map(f => ({ file: f, text: keep(read(join(rustDoc, 'licenses', f))) })),
  ] })
if (rust[0].texts.length < 3) throw new Error(`Rust's COPYRIGHT-library.html and licence files are not all in ${rustDoc}`)

// npm: the packages Vite put in the built page (vite.config.ts writes the list).
if (!process.argv.includes('--no-build')) execSync('npx vite build', { stdio: 'inherit' })
const web = JSON.parse(read(join('node_modules', '.cache', 'fvh', 'bundled.json'))).map(n => {
  const dir = join('node_modules', n), p = JSON.parse(read(join(dir, 'package.json')))
  const names = licenceFiles(dir)
  if (!names.length) throw new Error(`${n} has no licence file`)
  // A repository as "owner/name" or "github:owner/name" is on GitHub; only an https link is kept.
  let repo = (typeof p.repository === 'string' ? p.repository : p.repository?.url ?? '').replace(/^git\+/, '').replace(/\.git$/, '').replace(/^git:\/\//, 'https://')
  if (/^(github:)?[\w.-]+\/[\w.-]+$/.test(repo)) repo = `https://github.com/${repo.replace(/^github:/, '')}`
  return { name: n, version: p.version, licence: p.license ?? '',
    sources: [{ what: 'Source', url: `https://www.npmjs.com/package/${n}/v/${p.version}` }, ...(https(repo) ? [{ what: 'Repository', url: repo }] : [])],
    texts: files(dir, names) }
})

// Linux: every file linuxdeploy copied into the AppImage from an Ubuntu package (libraries, their
// plugins, typelibs, schemas), by its package, and the AppImage's runtime. linuxdeploy copies the
// copyright file of some packages only, so each one is read from the system that made the AppImage.
let linux = []
if (appdir) {
  const common = '/usr/share/common-licenses'
  const ours = new Set(['AppRun', 'AppRun.wrapped', '.DirIcon', 'FilesViewHub.png', 'filesviewhub.png', 'FilesViewHub.desktop', 'filesviewhub', 'linuxdeploy-plugin-gtk.sh',
    // Indexes linuxdeploy writes for the copied plugins and schemas.
    'loaders.cache', 'giomodule.cache', 'immodules.cache', 'gschemas.compiled'])
  const ffmpegLibs = new Set(readdirSync(join(TOOLS, 'ffmpeg', 'lib')))
  const copied = [], walk = d => {
    for (const n of readdirSync(d)) {
      const f = join(d, n), r = relative(appdir, f)
      if (r.startsWith(join('usr', 'lib', 'FilesViewHub')) || r.startsWith(join('usr', 'share', 'doc'))) continue
      if (statSync(f).isDirectory()) walk(f)
      else if (!ours.has(n) && !ffmpegLibs.has(n)) copied.push(f)
    }
  }
  walk(appdir)
  // The deepest copies first, so a copy linuxdeploy also put flat in usr/lib follows its package.
  copied.sort((a, b) => b.split(/[\\/]/).length - a.split(/[\\/]/).length || a.localeCompare(b))
  // dpkg -S prints "pkg[, pkg]:arch: /path" for each file a package holds whose name fits.
  const owners = (pattern) => {
    let out = ''
    try { out = execFileSync('dpkg', ['-S', pattern], { encoding: 'utf8', stdio: ['ignore', 'pipe', 'ignore'], maxBuffer: 64 << 20 }) } catch { return [] }
    return out.split('\n').filter(l => l.includes(': /')).flatMap(l => l.slice(0, l.indexOf(': /')).split(', ').map(p => ({ p: p.replace(/:[a-z0-9]+$/, ''), path: l.slice(l.indexOf(': /') + 2) })))
  }
  const byPackage = new Map(), loose = []
  for (const f of copied) {
    const n = basename(f)
    let found = owners(`*/${n}`).filter(o => basename(o.path) === n)
    // An unversioned library is the -dev package's link to the versioned one linuxdeploy copied.
    if (!found.some(o => !o.p.endsWith('-dev')) && /\.so$/.test(n)) found = owners(`*/${n}.*`)
    // Two packages may hold a file of that name (GTK 3's and GTK 4's plugins): the one whose path
    // ends most like the copy's wins, then one already found, a -dev package last.
    const tail = (path) => { const a = path.split('/').reverse(), b = relative(appdir, f).split(/[\\/]/).reverse(); let i = 0; while (i < a.length && a[i] === b[i]) i++; return i }
    const pkg = found.sort((x, y) => (+x.p.endsWith('-dev') - +y.p.endsWith('-dev')) || tail(y.path) - tail(x.path)
      || +byPackage.has(y.p) - +byPackage.has(x.p))[0]?.p
    if (pkg) byPackage.set(pkg, [...(byPackage.get(pkg) ?? []), relative(appdir, f)])
    else loose.push(relative(appdir, f))
  }
  if (loose.length) throw new Error(`No Ubuntu package holds these files of the AppImage:\n${loose.join('\n')}`)
  linux = [...byPackage.keys()].sort().map(p => {
    const [version, src, srcVersion] = execFileSync('dpkg-query', ['-W', '-f', '${Version}\t${source:Package}\t${source:Version}', p], { encoding: 'utf8' }).split('\t')
    const file = join('/usr/share/doc', p, 'copyright')
    if (!existsSync(file)) throw new Error(`${p} has no ${file}`)
    const copyright = read(file)
    // A Debian copyright file names the common licence texts instead of holding them.
    const refs = [...new Set([...copyright.matchAll(/\/usr\/share\/common-licenses\/([\w.+-]+[\w+])/g)].map(m => m[1]))]
      .filter(r => existsSync(join(common, r)))
    // A machine-readable copyright file names each licence on a License: line; another names the
    // common licence files it uses.
    const named = /^Format: /m.test(copyright) ? [...new Set([...copyright.matchAll(/^License: *(\S[^\n]*)$/gm)].map(m => m[1].trim()))] : refs
    return { name: p, version, licence: named.length ? named.join(', ') : guess([keep(copyright)]),
      sources: [{ what: `Ubuntu source package ${src} ${srcVersion}`, url: `https://launchpad.net/ubuntu/+source/${src}/${srcVersion}` }],
      texts: [{ file, text: keep(copyright) }, ...refs.map(r => ({ file: `${common}/${r}`, text: keep(read(join(common, r))) }))] }
  })
  const runtime = arg('--runtime')
  if (!/^[0-9a-f]{7,40}$/.test(runtime ?? '')) throw new Error('Give the AppImage runtime commit with --runtime (package.mjs reads it from the AppImage).')
  linux.unshift({ name: 'AppImage runtime', version: runtime,
    licence: 'MIT; built into it: musl libc (MIT), libfuse (LGPL 2.1), squashfuse (BSD 2-Clause), zstd (BSD 3-Clause), zlib (zlib)',
    sources: [{ what: 'Source, with the build that pins each library in it', url: `https://github.com/AppImage/type2-runtime/tree/${runtime}` }],
    texts: [['LICENSE', 'appimage-runtime.txt'], ['musl COPYRIGHT', 'musl.txt'], ['squashfuse LICENSE', 'squashfuse.txt'], ['zstd LICENSE', 'zstd.txt'], ['zlib LICENSE', 'zlib.txt']]
      .map(([file, f]) => ({ file, text: keep(read(join(LICENCES, f))) })).concat({ file: 'libfuse: LGPL 2.1', text: keep(read(join(common, 'LGPL-2.1'))) }) },
  { name: 'AppRun (the script linuxdeploy writes)', version: '', licence: 'MIT', sources: [{ what: 'Source', url: 'https://github.com/linuxdeploy/linuxdeploy' }],
    texts: [{ file: 'LICENSE.txt', text: keep(read(join(LICENCES, 'linuxdeploy.txt'))) }] },
  { name: 'AppRun.wrapped (AppImageKit\'s AppRun)', version: '', licence: 'MIT', sources: [{ what: 'Source', url: 'https://github.com/AppImage/AppImageKit' }],
    texts: [{ file: 'LICENSE', text: keep(read(join(LICENCES, 'appimage-apprun.txt'))) }] },
  { name: 'linuxdeploy-plugin-gtk', version: '', licence: 'MIT', sources: [{ what: 'Source', url: 'https://github.com/linuxdeploy/linuxdeploy-plugin-gtk' }],
    texts: [{ file: 'LICENSE.txt', text: keep(read(join(LICENCES, 'linuxdeploy-plugin-gtk.txt'))) }] })
}

const version = JSON.parse(read(join('src-tauri', 'tauri.conf.json'))).version
const groups = [
  ...(appSource.length ? [{ id: 'app', items: [{ name: 'FilesViewHub', version: JSON.parse(read(join('src-tauri', 'tauri.conf.json'))).version, licence: 'GPL-3.0-or-later',
    sources: [{ what: 'The exact source, in this package', url: `source/${appSource[0]}` }], texts: [{ file: 'LICENSE', text: keep(read('LICENSE')) }] }] }] : []),
  { id: 'tools', items: tools },
  // On macOS ddjvu's libraries are built into it, not MSYS2 DLLs.
  ...(msys2.length ? [{ id: system === 'macos' ? 'ddjvu' : 'msys2', items: msys2 }] : []),
  ...(inFfmpeg.length ? [{ id: 'ffmpeg', items: inFfmpeg, source: parts.builds }] : []),
  { id: 'rust', items: rust },
  { id: 'web', items: web },
  ...(linux.length ? [{ id: 'linux', items: linux }] : []),
]
// Only the texts some item ships.
const usedTexts = new Set(groups.flatMap(g => g.items.flatMap(i => i.texts.map(t => t.text))))
for (const id of Object.keys(texts)) if (!usedTexts.has(id)) delete texts[id]
writeFileSync(join(TOOLS, 'notices.json'), JSON.stringify({ v: 1, version, system, groups, texts }) + '\n')

const TITLES = {
  app: 'FilesViewHub itself',
  tools: 'Programs in the tools folder',
  msys2: 'Libraries ddjvu uses (MSYS2 packages)',
  ddjvu: 'Libraries built into ddjvu',
  ffmpeg: `Libraries built into FFmpeg (its build scripts: https://github.com/BtbN/FFmpeg-Builds/tree/${parts.builds})`,
  rust: 'Rust crates built into the app',
  web: 'Parts of the app\'s window (npm packages)',
  linux: 'Libraries and programs inside the AppImage',
}
const numbers = new Map(Object.keys(texts).map((id, i) => [id, `T${i + 1}`]))
let out = `FilesViewHub ${version}: third-party notices\n\nFilesViewHub's own code is free software under the GNU General Public License, version 3 or\nlater (LICENSE.txt; its text is also inside the app). The package also carries the parts below,\neach under its own licence. Each part names the licence files it ships; their texts follow at\nthe end, by number.\n`
for (const g of groups) {
  out += `\n\n== ${TITLES[g.id]} ==\n`
  for (const i of g.items) {
    out += `\n${i.name}${i.version ? ` ${i.version}` : ''}${i.licence ? ` - ${i.licence}` : ''}\n`
    for (const s of i.sources) out += `  ${s.what}: ${s.url}\n`
    if (i.texts.length) out += `  Licence: ${i.texts.map(t => `${t.file} [${numbers.get(t.text)}]`).join(', ')}\n`
  }
}
out += '\n\n== Licence texts ==\n'
for (const [id, t] of Object.entries(texts)) out += `\n----- [${numbers.get(id)}] -----\n\n${t.trimEnd()}\n`
writeFileSync(join(TOOLS, 'THIRD-PARTY-NOTICES.txt'), out)
const count = groups.reduce((n, g) => n + g.items.length, 0)
console.log(`\n${count} parts, ${Object.keys(texts).length} licence texts: ${join(TOOLS, 'notices.json')}, ${join(TOOLS, 'THIRD-PARTY-NOTICES.txt')}`)
