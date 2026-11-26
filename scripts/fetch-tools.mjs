// Fetches the tools the app bundles for thumbnails (`decisions/2026/10/05/filesviewhub-is-a-tauri-app-shipped-portable.md`) into app/tools/, with a tools.json
// the app reads for its "What this app does" screen. Every download is pinned by SHA-256.
// Run from the repo root: `npm run fetch-tools`. Windows and Linux (x86_64), and macOS (arm64 or x64, the
// Mac's own). On Linux ffmpeg and PDFium are bundled and ddjvu is the system's (Arch: djvulibre); it needs
// patchelf. On macOS FFmpeg and ddjvu are built here from their pinned source (Xcode's command line tools;
// on an Intel Mac also nasm), FFmpeg once per pin and Mac type, kept in tools/.download.
//
// A pinned URL can disappear (BtbN keeps dated ffmpeg builds for a few weeks, MSYS2 prunes old
// packages). The script then stops; pick the newer file, put its URL and hash here, and run again.
import { createHash } from 'node:crypto'
import { execFileSync } from 'node:child_process'
import { chmodSync, copyFileSync, cpSync, existsSync, mkdirSync, readdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { join, relative } from 'node:path'

if (!['win32', 'linux', 'darwin'].includes(process.platform)) { console.error('The tools are fetched on Windows, Linux and macOS.'); process.exit(1) }
if (process.platform === 'darwin' && !['arm64', 'x64'].includes(process.arch)) { console.error(`No tools for a ${process.arch} Mac.`); process.exit(1) }

const FFMPEG = 'ffmpeg-n9.0.2-39-g7adfbe40dc-win64-lgpl-shared-9.0'
// The FFmpeg commit that build is made from, and the FFmpeg-Builds commit whose scripts made it; the
// libraries those scripts link in have their licences in scripts/ffmpeg-licences.json, collected at
// that commit by scripts/ffmpeg-licences.mjs.
const FFMPEG_COMMIT = '7adfbe40dcde598f93a6963d2f7b22cd75202ccc'
const BUILDS_COMMIT = '5031c6f1082414e53688d39e521849905071070d'
// BtbN deletes its daily builds after some weeks but keeps the last one of each month: pin only those.
const BUILDS_TAG = 'autobuild-2026-10-31-12-59'
const monthEnd = (tag) => { const [, y, m, d] = tag.match(/^autobuild-(\d{4})-(\d{2})-(\d{2})-/).map(Number); return new Date(Date.UTC(y, m - 1, d + 1)).getUTCDate() === 1 }
if (!monthEnd(BUILDS_TAG)) throw new Error(`${BUILDS_TAG} is not the last build of a month, which BtbN deletes after some weeks; pin a month-end one`)
const BUILDS_RELEASE = `https://github.com/BtbN/FFmpeg-Builds/releases/tag/${BUILDS_TAG}`
const PDFIUM_RELEASE = 'https://github.com/bblanchon/pdfium-binaries/releases/tag/chromium/8165'
const MSYS = 'https://repo.msys2.org/mingw/ucrt64/'
const msys = (name, sha256) => ({ url: MSYS + encodeURIComponent(name), file: name, sha256 })

const DOWNLOADS = {
  ffmpeg: {
    url: `https://github.com/BtbN/FFmpeg-Builds/releases/download/${BUILDS_TAG}/${FFMPEG}.zip`,
    file: `${FFMPEG}.zip`, sha256: 'e907b01fa4783913151237e37df2a10fa97d99a1598a021bf59d4ae62ffed4c0',
  },
  pdfium: {
    url: 'https://github.com/bblanchon/pdfium-binaries/releases/download/chromium/8165/pdfium-win-x64.tgz',
    file: 'pdfium-win-x64.tgz', sha256: 'ca7338ae7779b8b1b82f4580305c4d762d214f8df1d54b47647fe3cc75989ded',
  },
}
// ddjvu.exe and every DLL it loads, found by walking their import tables.
const DJVU = [
  msys('mingw-w64-ucrt-x86_64-djvulibre-3.5.30-1-any.pkg.tar.zst', '12cf80ecbc195620348276b4ecc5267ec4ba85c92e5703cb5f34c7324fcc2257'),
  msys('mingw-w64-ucrt-x86_64-libtiff-4.7.2-2-any.pkg.tar.zst', '258e08a91e6b1703d41730ffe6080c2a40968fe90896b45716c1b41042775076'),
  msys('mingw-w64-ucrt-x86_64-libjpeg-turbo-3.2.1-1-any.pkg.tar.zst', '8eed77c1e0d6de7f8822687f7b1dfa32cda09394806ef67b23399e63de1a92a6'),
  msys('mingw-w64-ucrt-x86_64-libgcc-16.2.0-7-any.pkg.tar.zst', 'bc0ae31c582e0847964b4c8c4b55bb2404a9b7ddce7ebf0b176b6693b76e0c62'),
  msys('mingw-w64-ucrt-x86_64-libstdc++-16.2.0-7-any.pkg.tar.zst', '4dd4b216b0e92ffa160127dd50620372609e98cb219ff1c5dcb9b3fbfe153253'),
  msys('mingw-w64-ucrt-x86_64-libwinpthread-14.0.0.r598.ge3d18511f-1-any.pkg.tar.zst', 'c35d6be83d141c8cfbb61f1637dd4b53aac8f5562d766aa26d0741c40bad4e4d'),
  msys('mingw-w64-ucrt-x86_64-zlib-1.4-2-any.pkg.tar.zst', '172541cf5e2391a402f092104988699b0163129d22be1d6b4b201bfb84af6fcf'),
  msys('mingw-w64-ucrt-x86_64-xz-5.8.4-1-any.pkg.tar.zst', 'a9d771f816d8c0c5bde152ffb27a687ede91079b8e77e5b8297efa3b5a835934'),
  msys('mingw-w64-ucrt-x86_64-zstd-1.5.7-2-any.pkg.tar.zst', 'dbdb8427280046a2b41697780aa4c52983b708082b0da4755951dc3bea96ca89'),
  msys('mingw-w64-ucrt-x86_64-libdeflate-1.26-1-any.pkg.tar.zst', '17720c77bd01641a3554801cafd71d96fb7fff7427f4cc20ac9f36efb8cb3da0'),
  msys('mingw-w64-ucrt-x86_64-lerc-4.1.1-1-any.pkg.tar.zst', 'c0afbf4832488d67030c6f313349cd814946c9a7bc3ae9fd54524825c9ca055d'),
  msys('mingw-w64-ucrt-x86_64-jbigkit-2.1-6-any.pkg.tar.zst', '9e237d1041350e7436294cfa13d4bd3d1606d1f2a32ade2eb2958b4d4e79b7ad'),
  msys('mingw-w64-ucrt-x86_64-libwebp-1.6.0-1-any.pkg.tar.zst', '34ff8396c403d484c8777ad49ad0fc06dfb94fadf6f14bcf84d50c45bc6fd17e'),
]
// ddjvu is GPL, so its exact source ships next to it.
const DJVU_SOURCE = { url: MSYS.replace('/ucrt64/', '/sources/') + 'mingw-w64-djvulibre-3.5.30-1.src.tar.zst',
  file: 'mingw-w64-djvulibre-3.5.30-1.src.tar.zst', sha256: 'f72480c4f49a5083632ea4a1ef29bee0f9e99b5258aac144fa7c6a043ce894ce' }
// jbigkit is GPL too, and ddjvu loads it (through libtiff), so its source ships as well.
const JBIG_SOURCE = { url: MSYS.replace('/ucrt64/', '/sources/') + 'mingw-w64-jbigkit-2.1-6.src.tar.zst',
  file: 'mingw-w64-jbigkit-2.1-6.src.tar.zst', sha256: '64757867df26308c14d22b184aaecb43742c25ee8d24f18da51e3d1e09be8ac2' }
const DJVU_FILES = ['ddjvu.exe', 'libdjvulibre-21.dll', 'libtiff-6.dll', 'libjpeg-8.dll', 'libgcc_s_seh-1.dll', 'libstdc++-6.dll',
  'libwinpthread-1.dll', 'zlib1.dll', 'liblzma-5.dll', 'libzstd.dll', 'libdeflate.dll', 'libLerc.dll', 'libjbig-0.dll',
  'libwebp-7.dll', 'libsharpyuv-0.dll']
const FFMPEG_FILES = ['ffmpeg.exe', 'ffprobe.exe', 'avcodec-63.dll', 'avdevice-63.dll', 'avfilter-12.dll', 'avformat-63.dll',
  'avutil-61.dll', 'swresample-7.dll', 'swscale-10.dll']

const TOOLS = 'tools'
const CACHE = join(TOOLS, '.download')
const WORK = join(CACHE, 'unpacked')

async function get({ url, file, sha256 }) {
  const f = join(CACHE, file)
  // A file kept from an older pin of the same name (PDFium's) is fetched again.
  if (existsSync(f) && createHash('sha256').update(readFileSync(f)).digest('hex') !== sha256) rmSync(f)
  if (!existsSync(f)) {
    console.log(`fetching ${url}`)
    const r = await fetch(url)
    if (!r.ok) throw new Error(`${url}: HTTP ${r.status}. The pinned file is gone; pin a newer one in scripts/fetch-tools.mjs.`)
    writeFileSync(f, Buffer.from(await r.arrayBuffer()))
  }
  const got = createHash('sha256').update(readFileSync(f)).digest('hex')
  if (got !== sha256) { rmSync(f); throw new Error(`${file}: SHA-256 is ${got}, expected ${sha256}. Deleted it.`) }
  return f
}

const unpack = (archive, into) => { mkdirSync(into, { recursive: true }); execFileSync('tar', ['-xf', archive, '-C', into]) }
function copy(from, files, to) {
  mkdirSync(to, { recursive: true })
  for (const n of files) {
    if (!existsSync(join(from, n))) throw new Error(`${n} is not in ${from}`)
    copyFileSync(join(from, n), join(to, n))
  }
}

// ddjvu reads its arguments in the ANSI code page, so a path outside it (Cyrillic on an English
// Windows) reaches it as "?". Its embedded manifest is rewritten in place, at the same length, to
// ask Windows 10 1903 and later for UTF-8 instead.
function utf8(exe) {
  const d = readFileSync(exe)
  const start = d.indexOf('<assembly'), end = d.indexOf('</assembly>', start) + '</assembly>'.length
  if (start < 0 || end < start) throw new Error(`${exe} has no manifest`)
  const old = d.subarray(start, end).toString('latin1')
  const trust = old.match(/<trustInfo[\s\S]*?<\/trustInfo>/)[0].replace(/\s+/g, ' ')
  const compat = old.match(/<compatibility[\s\S]*?<\/compatibility>/)[0].replace(/<!--[\s\S]*?-->/g, '').replace(/\s+/g, ' ')
  const now = '<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">' + trust + compat +
    '<application xmlns="urn:schemas-microsoft-com:asm.v3"><windowsSettings>' +
    '<activeCodePage xmlns="http://schemas.microsoft.com/SMI/2019/WindowsSettings">UTF-8</activeCodePage>' +
    '</windowsSettings></application></assembly>'
  if (now.length > old.length) throw new Error(`the new manifest of ${exe} does not fit`)
  d.write(now.padEnd(old.length, ' '), start, 'latin1')
  writeFileSync(exe, d)
}

const LINUX = {
  ffmpeg: {
    url: `https://github.com/BtbN/FFmpeg-Builds/releases/download/${BUILDS_TAG}/${FFMPEG.replace('win64', 'linux64')}.tar.xz`,
    file: `${FFMPEG.replace('win64', 'linux64')}.tar.xz`, sha256: '5a58e2dffad4835ee3418939f2676d6898489f8e047299f5f62ee0da3f8dd5dc',
  },
  pdfium: {
    url: 'https://github.com/bblanchon/pdfium-binaries/releases/download/chromium/8165/pdfium-linux-x64.tgz',
    file: 'pdfium-linux-x64.tgz', sha256: 'ba7865e16c0271daab75256790be06c26272190afe6e2a70eae0bb39bb99089c',
  },
}
// ffmpeg and ffprobe find these by their RPATH, `$ORIGIN/../lib`; each is copied under its soname.
const FFMPEG_LIBS = ['libavcodec.so.63', 'libavdevice.so.63', 'libavfilter.so.12', 'libavformat.so.63', 'libavutil.so.61',
  'libswresample.so.7', 'libswscale.so.10']

// macOS: no FFmpeg build to fetch matches the others, so FFmpeg is built from the pinned commit with
// only its own decoders and the system's zlib and bzip2: LGPL 2.1 or later, nothing else
// inside. DjVuLibre's ddjvu is built from its release, without JPEG and TIFF, which writing PNM needs
// neither: a DjVu page whose pictures are JPEG shows without them.
const MAC = {
  ffmpeg: { url: `https://github.com/FFmpeg/FFmpeg/archive/${FFMPEG_COMMIT}.tar.gz`, file: `ffmpeg-${FFMPEG_COMMIT}.tar.gz`,
    sha256: '4babda050dcfccb1194dd2a9e75b5e5ec1aa33b3e99d4f6a7106b6337b6b4a0c' },
  // The same file as the one inside MSYS2's source package the Windows ddjvu is built from.
  djvulibre: { url: 'https://downloads.sourceforge.net/project/djvu/DjVuLibre/3.5.30/djvulibre-3.5.30.tar.gz', file: 'djvulibre-3.5.30.tar.gz',
    sha256: 'ee5e457d4cfebe566f94b99e5e3d3cc7f5c79ddb741c2ac2ba2e456f00329644' },
  pdfium: {
    arm64: { url: 'https://github.com/bblanchon/pdfium-binaries/releases/download/chromium/8165/pdfium-mac-arm64.tgz', file: 'pdfium-mac-arm64.tgz',
      sha256: '112e74d717f3e0310273b70f3c8780e8a61ff11b34834124465c8305c841d350' },
    x64: { url: 'https://github.com/bblanchon/pdfium-binaries/releases/download/chromium/8165/pdfium-mac-x64.tgz', file: 'pdfium-mac-x64.tgz',
      sha256: '07285915b564e46a432a9909dbc1f15892885ca6489e3faeefc70dee77ed371a' },
  },
}
// The oldest macOS the app runs on (src-tauri/tauri.macos.conf.json); the tools are built for it too.
const MIN_MACOS = '13.0'
const macFlags = `-O2 -mmacosx-version-min=${MIN_MACOS}`
const FFMPEG_CONFIGURE = ['--disable-autodetect', '--enable-pthreads', '--enable-zlib', '--enable-bzlib', '--disable-network', '--disable-doc',
  '--disable-ffplay', '--disable-debug', '--enable-static', '--disable-shared', `--extra-cflags=-mmacosx-version-min=${MIN_MACOS}`,
  `--extra-ldflags=-mmacosx-version-min=${MIN_MACOS}`]
const DJVU_CONFIGURE = ['--disable-shared', '--enable-static', '--without-jpeg', '--without-tiff', '--disable-xmltools', '--disable-desktopfiles']
const jobs = String(Math.max(2, (await import('node:os')).availableParallelism()))
const run = (cmd, args, cwd, env = {}) => execFileSync(cmd, args, { cwd, stdio: 'inherit', env: { ...process.env, ...env } })

const parts = JSON.parse(readFileSync(join('scripts', 'ffmpeg-licences.json'), 'utf8'))
if (parts.builds !== BUILDS_COMMIT) throw new Error(`scripts/ffmpeg-licences.json is for FFmpeg-Builds ${parts.builds}; run scripts/ffmpeg-licences.mjs ${BUILDS_COMMIT}`)
// Where each tool's exact source is. FFmpeg's build links in other libraries; their licences and
// sources are in THIRD-PARTY-NOTICES.txt, made by scripts/notices.mjs from ffmpeg-licences.json.
const FFMPEG_SOURCES = [
  { what: 'FFmpeg source at the commit this build is made from', url: `https://github.com/FFmpeg/FFmpeg/tree/${FFMPEG_COMMIT}` },
  { what: 'The build scripts, which pin each library it links in', url: `https://github.com/BtbN/FFmpeg-Builds/tree/${BUILDS_COMMIT}` },
  { what: 'The build itself', url: BUILDS_RELEASE },
]
// BtbN's LGPL build also links in FFTW (GPL 2.0 or later, through Chromaprint), so as a whole it is GPL.
const FFMPEG_LICENCE = 'LGPL 3.0 or later; with FFTW (GPL 2.0 or later) inside, GPL 3.0 as a whole'
const PDFIUM_SOURCES = [{ what: 'The build and the PDFium branch it is made from', url: PDFIUM_RELEASE }]
// PDFium's own licence and those of the libraries built into it.
function pdfiumLicences(from, to) {
  cpSync(join(from, 'LICENSE'), join(to, 'LICENSE.txt'))
  cpSync(join(from, 'licenses'), join(to, 'licenses'), { recursive: true })
  return ['pdfium/LICENSE.txt', ...readdirSync(join(from, 'licenses')).sort().map(f => `pdfium/licenses/${f}`)]
}

mkdirSync(CACHE, { recursive: true })
rmSync(WORK, { recursive: true, force: true })
for (const d of ['ffmpeg', 'pdfium', 'djvulibre', 'tools.json']) rmSync(join(TOOLS, d), { recursive: true, force: true })

if (process.platform === 'linux') {
  const ff = FFMPEG.replace('win64', 'linux64')
  unpack(await get(LINUX.ffmpeg), join(WORK, 'ffmpeg'))
  copy(join(WORK, 'ffmpeg', ff, 'bin'), ['ffmpeg', 'ffprobe'], join(TOOLS, 'ffmpeg', 'bin'))
  copy(join(WORK, 'ffmpeg', ff, 'lib'), FFMPEG_LIBS, join(TOOLS, 'ffmpeg', 'lib'))
  // The libraries load each other from their own folder, which the AppImage bundler checks.
  for (const l of FFMPEG_LIBS) execFileSync('patchelf', ['--set-rpath', '$ORIGIN', join(TOOLS, 'ffmpeg', 'lib', l)])
  cpSync(join(WORK, 'ffmpeg', ff, 'LICENSE.txt'), join(TOOLS, 'ffmpeg', 'LICENSE.txt'))
  unpack(await get(LINUX.pdfium), join(WORK, 'pdfium'))
  copy(join(WORK, 'pdfium', 'lib'), ['libpdfium.so'], join(TOOLS, 'pdfium'))
  const pdfiumFiles = pdfiumLicences(join(WORK, 'pdfium'), join(TOOLS, 'pdfium'))
  const build = readFileSync(join(WORK, 'pdfium', 'VERSION'), 'utf8').match(/BUILD=(\d+)/)[1]
  const manifest = {
    v: 1,
    tools: [
      { id: 'ffmpeg', name: 'FFmpeg', version: '9.0.2', licence: FFMPEG_LICENCE, files: ['ffmpeg/bin/ffmpeg', 'ffmpeg/bin/ffprobe'],
        ships: ['ffmpeg/bin/ffmpeg', 'ffmpeg/bin/ffprobe', ...FFMPEG_LIBS.map(l => `ffmpeg/lib/${l}`)], source: BUILDS_RELEASE, sources: FFMPEG_SOURCES, licences: ['ffmpeg/LICENSE.txt'], parts: 'ffmpeg' },
      { id: 'pdfium', name: 'PDFium', version: `chromium/${build}`, licence: 'BSD 3-Clause and Apache 2.0', files: ['pdfium/libpdfium.so'],
        ships: ['pdfium/libpdfium.so'], source: PDFIUM_RELEASE, sources: PDFIUM_SOURCES, licences: pdfiumFiles },
      // Not bundled: the system's, from its djvulibre package. Missing, DjVu files get no picture.
      { id: 'ddjvu', name: 'DjVuLibre ddjvu', version: 'from your system', licence: 'GPL 2.0 or later', files: ['/usr/bin/ddjvu'],
        source: 'the djvulibre package of your Linux', system: true },
    ],
  }
  writeFileSync(join(TOOLS, 'tools.json'), JSON.stringify(manifest, null, 2) + '\n')
  rmSync(WORK, { recursive: true, force: true })
  // tauri.linux.conf.json bundles the notices and the app's source, so a build or a test fails while
  // they are missing: the notices are made now (each package makes them again, with the AppImage's
  // libraries), and tools/source/ is left for package.mjs to fill.
  execFileSync(process.execPath, [join('scripts', 'notices.mjs')], { stdio: 'inherit' })
  mkdirSync(join(TOOLS, 'source'), { recursive: true })
  console.log(`\ntools ready in ${TOOLS}/`)
  process.exit(0)
}

if (process.platform === 'darwin') {
  const arch = process.arch
  if (arch === 'x64') try { execFileSync('nasm', ['-v']) } catch { console.error('FFmpeg needs nasm on an Intel Mac: brew install nasm'); process.exit(1) }
  // FFmpeg, built once per pin, configure options and Mac type; a build takes about 10 minutes.
  const recipe = createHash('sha256').update(JSON.stringify([FFMPEG_COMMIT, FFMPEG_CONFIGURE])).digest('hex').slice(0, 12)
  const built = join(CACHE, `ffmpeg-built-${recipe}-${arch}`)
  if (!existsSync(join(built, 'ffprobe'))) {
    rmSync(built, { recursive: true, force: true })
    unpack(await get(MAC.ffmpeg), join(WORK, 'ffmpeg'))
    const src = join(WORK, 'ffmpeg', `FFmpeg-${FFMPEG_COMMIT}`)
    run('./configure', FFMPEG_CONFIGURE, src)
    run('make', [`-j${jobs}`, 'ffmpeg', 'ffprobe'], src)
    mkdirSync(built, { recursive: true })
    for (const f of ['LICENSE.md', 'COPYING.LGPLv2.1']) copyFileSync(join(src, f), join(built, f))
    for (const f of ['ffmpeg', 'ffprobe']) copyFileSync(join(src, f), join(built, f))
  }
  copy(built, ['ffmpeg', 'ffprobe', 'LICENSE.md', 'COPYING.LGPLv2.1'], join(TOOLS, 'ffmpeg'))
  for (const f of ['ffmpeg', 'ffprobe']) chmodSync(join(TOOLS, 'ffmpeg', f), 0o755)
  writeFileSync(join(TOOLS, 'ffmpeg', 'BUILD.txt'), `ffmpeg and ffprobe are built by FilesViewHub's tools script (scripts/fetch-tools.mjs) from
FFmpeg's source at commit ${FFMPEG_COMMIT}
(${MAC.ffmpeg.url}), unchanged, for a ${arch} Mac, with:

./configure ${FFMPEG_CONFIGURE.join(' ')}
make ffmpeg ffprobe

Nothing else is built into them; they use macOS's own zlib and bzip2.
`)

  unpack(await get(MAC.djvulibre), join(WORK, 'djvu'))
  const dj = join(WORK, 'djvu', 'djvulibre-3.5.30')
  run('./configure', DJVU_CONFIGURE, dj, { CFLAGS: macFlags, CXXFLAGS: macFlags, LDFLAGS: `-mmacosx-version-min=${MIN_MACOS}` })
  run('make', [`-j${jobs}`, '-C', 'libdjvu'], dj)
  run('make', [`-j${jobs}`, '-C', 'tools', 'ddjvu'], dj)
  copy(join(dj, 'tools'), ['ddjvu'], join(TOOLS, 'djvulibre'))
  for (const f of ['COPYING', 'COPYRIGHT']) cpSync(join(dj, f), join(TOOLS, 'djvulibre', `${f}.txt`))
  // GPL 2: the exact source ships next to it, with how it is built.
  mkdirSync(join(TOOLS, 'djvulibre', 'source'), { recursive: true })
  copyFileSync(join(CACHE, MAC.djvulibre.file), join(TOOLS, 'djvulibre', 'source', MAC.djvulibre.file))
  writeFileSync(join(TOOLS, 'djvulibre', 'BUILD.txt'), `ddjvu is built by FilesViewHub's tools script (scripts/fetch-tools.mjs) from DjVuLibre's release
source/${MAC.djvulibre.file}, unchanged, for a ${arch} Mac, with:

CFLAGS="${macFlags}" CXXFLAGS="${macFlags}" LDFLAGS="-mmacosx-version-min=${MIN_MACOS}" ./configure ${DJVU_CONFIGURE.join(' ')}
make -C libdjvu
make -C tools ddjvu
`)

  unpack(await get(MAC.pdfium[arch]), join(WORK, 'pdfium'))
  copy(join(WORK, 'pdfium', 'lib'), ['libpdfium.dylib'], join(TOOLS, 'pdfium'))
  const pdfiumFiles = pdfiumLicences(join(WORK, 'pdfium'), join(TOOLS, 'pdfium'))
  // A Mac runs only signed code; until the app is signed (issue #3) each tool gets an ad-hoc
  // signature, which strip and copying can drop.
  for (const f of ['ffmpeg/ffmpeg', 'ffmpeg/ffprobe', 'djvulibre/ddjvu', 'pdfium/libpdfium.dylib']) execFileSync('codesign', ['--force', '-s', '-', join(TOOLS, f)])
  const build = readFileSync(join(WORK, 'pdfium', 'VERSION'), 'utf8').match(/BUILD=(\d+)/)[1]
  const manifest = {
    v: 1,
    tools: [
      { id: 'ffmpeg', name: 'FFmpeg', version: '9.0.2', licence: 'LGPL 2.1 or later', files: ['ffmpeg/ffmpeg', 'ffmpeg/ffprobe'],
        ships: ['ffmpeg/ffmpeg', 'ffmpeg/ffprobe'], source: `https://github.com/FFmpeg/FFmpeg/tree/${FFMPEG_COMMIT}`, licences: ['ffmpeg/LICENSE.md', 'ffmpeg/COPYING.LGPLv2.1'],
        sources: [{ what: 'FFmpeg source at the commit this build is made from', url: `https://github.com/FFmpeg/FFmpeg/tree/${FFMPEG_COMMIT}` },
          { what: 'How it is built', url: 'ffmpeg/BUILD.txt' }] },
      { id: 'pdfium', name: 'PDFium', version: `chromium/${build}`, licence: 'BSD 3-Clause and Apache 2.0', files: ['pdfium/libpdfium.dylib'],
        ships: ['pdfium/libpdfium.dylib'], source: PDFIUM_RELEASE, sources: PDFIUM_SOURCES, licences: pdfiumFiles },
      { id: 'ddjvu', name: 'DjVuLibre ddjvu', version: '3.5.30', licence: 'GPL 2.0 or later', files: ['djvulibre/ddjvu'], ships: ['djvulibre/ddjvu'],
        source: `tools/djvulibre/source/${MAC.djvulibre.file}`, licences: ['djvulibre/COPYING.txt', 'djvulibre/COPYRIGHT.txt'],
        sources: [{ what: 'The exact source, in this package', url: `djvulibre/source/${MAC.djvulibre.file}` },
          { what: 'How it is built', url: 'djvulibre/BUILD.txt' }, { what: 'The same file at DjVuLibre', url: MAC.djvulibre.url }] },
    ],
  }
  writeFileSync(join(TOOLS, 'tools.json'), JSON.stringify(manifest, null, 2) + '\n')
  rmSync(WORK, { recursive: true, force: true })
  console.log(`\ntools ready in ${TOOLS}/`)
  process.exit(0)
}

unpack(await get(DOWNLOADS.ffmpeg), join(WORK, 'ffmpeg'))
const ffBin = join(WORK, 'ffmpeg', FFMPEG, 'bin')
copy(ffBin, FFMPEG_FILES, join(TOOLS, 'ffmpeg'))
cpSync(join(WORK, 'ffmpeg', FFMPEG, 'LICENSE.txt'), join(TOOLS, 'ffmpeg', 'LICENSE.txt'))

unpack(await get(DOWNLOADS.pdfium), join(WORK, 'pdfium'))
copy(join(WORK, 'pdfium', 'bin'), ['pdfium.dll'], join(TOOLS, 'pdfium'))
const pdfiumFiles = pdfiumLicences(join(WORK, 'pdfium'), join(TOOLS, 'pdfium'))
const pdfiumBuild = readFileSync(join(WORK, 'pdfium', 'VERSION'), 'utf8').match(/BUILD=(\d+)/)[1]

for (const p of DJVU) unpack(await get(p), join(WORK, 'djvu'))
copy(join(WORK, 'djvu', 'ucrt64', 'bin'), DJVU_FILES, join(TOOLS, 'djvulibre'))
utf8(join(TOOLS, 'djvulibre', 'ddjvu.exe'))
cpSync(join(WORK, 'djvu', 'ucrt64', 'share', 'licenses'), join(TOOLS, 'djvulibre', 'licenses'), { recursive: true })
// The source package (MSYS2's build recipe and DjVuLibre's own tarball) ships as it is; its
// licence and copyright notice come from that tarball.
const src = join(TOOLS, 'djvulibre', 'source')
mkdirSync(src, { recursive: true })
copyFileSync(await get(DJVU_SOURCE), join(src, DJVU_SOURCE.file))
unpack(join(src, DJVU_SOURCE.file), join(WORK, 'djvu-src'))
execFileSync('tar', ['-xzf', join(WORK, 'djvu-src', 'mingw-w64-djvulibre', 'djvulibre-3.5.30.tar.gz'), '-C', join(WORK, 'djvu-src'),
  'djvulibre-3.5.30/COPYING', 'djvulibre-3.5.30/COPYRIGHT'])
cpSync(join(WORK, 'djvu-src', 'djvulibre-3.5.30', 'COPYING'), join(TOOLS, 'djvulibre', 'COPYING.txt'))
cpSync(join(WORK, 'djvu-src', 'djvulibre-3.5.30', 'COPYRIGHT'), join(TOOLS, 'djvulibre', 'COPYRIGHT.txt'))
// GPL 2 asks a changed program to say so: ddjvu.exe's manifest is not MSYS2's.
writeFileSync(join(TOOLS, 'djvulibre', 'CHANGES.txt'), `ddjvu.exe is MSYS2's build (mingw-w64-ucrt-x86_64-djvulibre-3.5.30-1), changed by
FilesViewHub's tools script (scripts/fetch-tools.mjs) as it is fetched: its embedded application
manifest is rewritten, at the same length, so Windows 10 1903 and later pass it paths in UTF-8.
Nothing else in the file is changed. The manifest it now carries:

${readFileSync(join(TOOLS, 'djvulibre', 'ddjvu.exe')).toString('latin1').match(/<assembly[\s\S]*?<\/assembly>/)[0]}
`)
copyFileSync(await get(JBIG_SOURCE), join(src, JBIG_SOURCE.file))
const djvuFiles = ['djvulibre/COPYING.txt', 'djvulibre/COPYRIGHT.txt', 'djvulibre/CHANGES.txt']
// Each MSYS2 package ddjvu's DLLs come from is a part of its own: its name, version and licence from
// its .PKGINFO, the DLLs it gives, its licence files and its MSYS2 source package.
const packages = DJVU.filter(p => !p.file.includes('-djvulibre-')).map(p => {
  const pkg = join(CACHE, p.file)
  const info = Object.fromEntries(execFileSync('tar', ['-xOf', pkg, '.PKGINFO'], { encoding: 'utf8' }).split(/\r?\n/)
    .filter(l => / = /.test(l)).map(l => l.split(' = ')).reverse())
  const name = info.pkgname.replace('mingw-w64-ucrt-x86_64-', '')
  const dlls = execFileSync('tar', ['-tf', pkg], { encoding: 'utf8' }).split(/\r?\n/).map(f => f.replace(/^ucrt64\/bin\//, ''))
    .filter(f => DJVU_FILES.includes(f)).map(f => `djvulibre/${f}`)
  const lic = join(TOOLS, 'djvulibre', 'licenses', name)
  const licences = existsSync(lic) ? readdirSync(lic, { recursive: true, withFileTypes: true }).filter(f => f.isFile())
    .map(f => relative(TOOLS, join(f.parentPath, f.name)).replaceAll('\\', '/')).sort() : []
  if (!dlls.length || !licences.length) throw new Error(`${p.file}: ${dlls.length ? 'no licence files' : 'no DLL ddjvu uses'}`)
  const source = `${info.pkgbase}-${info.pkgver}.src.tar.zst`
  return { name, version: info.pkgver, licence: info.license.replace(/^spdx:/, ''), files: dlls, licences,
    sources: name === 'jbigkit'
      ? [{ what: 'The exact source, in this package', url: `djvulibre/source/${JBIG_SOURCE.file}` }, { what: 'The same file at MSYS2', url: JBIG_SOURCE.url }]
      : [{ what: 'MSYS2 source package', url: MSYS.replace('/ucrt64/', '/sources/') + encodeURIComponent(source) }] }
})

// What the app shows for each tool. `files` are relative to tools/; the first one is what it runs.
const manifest = {
  v: 1,
  tools: [
    { id: 'ffmpeg', name: 'FFmpeg', version: '9.0.2', licence: FFMPEG_LICENCE, files: ['ffmpeg/ffmpeg.exe', 'ffmpeg/ffprobe.exe'],
      ships: FFMPEG_FILES.map(f => `ffmpeg/${f}`), source: BUILDS_RELEASE, sources: FFMPEG_SOURCES, licences: ['ffmpeg/LICENSE.txt'], parts: 'ffmpeg' },
    { id: 'pdfium', name: 'PDFium', version: `chromium/${pdfiumBuild}`, licence: 'BSD 3-Clause and Apache 2.0', files: ['pdfium/pdfium.dll'],
      ships: ['pdfium/pdfium.dll'], source: PDFIUM_RELEASE, sources: PDFIUM_SOURCES, licences: pdfiumFiles },
    { id: 'ddjvu', name: 'DjVuLibre ddjvu', version: '3.5.30', licence: 'GPL 2.0 or later', files: ['djvulibre/ddjvu.exe'],
      source: `tools/djvulibre/source/${DJVU_SOURCE.file}`, licences: djvuFiles, ships: ['djvulibre/ddjvu.exe', 'djvulibre/libdjvulibre-21.dll'], packages,
      sources: [{ what: 'The exact source, in this package', url: `djvulibre/source/${DJVU_SOURCE.file}` }, { what: 'The same file at MSYS2', url: DJVU_SOURCE.url }] },
  ],
}
writeFileSync(join(TOOLS, 'tools.json'), JSON.stringify(manifest, null, 2) + '\n')
rmSync(WORK, { recursive: true, force: true })
console.log(`\ntools ready in ${TOOLS}${'\\'}`)
