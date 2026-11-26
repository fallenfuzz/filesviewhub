// Builds the portable zip (`decisions/2026/10/05/filesviewhub-is-a-tauri-app-shipped-portable.md`): the .exe, its tools folder, the README for people, the app's
// licence and the third-party notices, named with the version and build number. Run from
// the repo root: `npm run package`. On Linux it makes one AppImage instead, its tools and notices inside
// (`src-tauri/tauri.linux.conf.json`). On macOS it makes a disk image holding a FilesViewHub folder:
// the .app, its tools and notices inside (`src-tauri/macos-package.conf.json`), and the README, the
// licence and the notices next to it, for the Mac it runs on (arm64 or x64).
// A release runs it through scripts/release.sh from WSL, which first refuses uncommitted changes and runs check.mjs.
import { execFileSync, execSync } from 'node:child_process'
import { chmodSync, cpSync, existsSync, mkdirSync, copyFileSync, readdirSync, rmSync, readFileSync } from 'node:fs'
import { join, resolve } from 'node:path'

if (!['win32', 'linux', 'darwin'].includes(process.platform)) { console.error('Packaging runs on Windows, Linux and macOS.'); process.exit(1) }
// The app's folder in a git checkout. The Linux build folder has no .git; scripts/linux.sh names it in FVH_APP.
const repo = process.env.FVH_APP ?? '.'
const git = (args) => execSync(`git -C "${repo}" ${args}`).toString().trim()
// The build number, as src-tauri/build.rs makes it: the commit count plus the builds before this repository.
const buildNumber = () => Number(git('rev-list --count HEAD')) + Number(readFileSync('src-tauri/build.rs', 'utf8').match(/^const EARLIER_BUILDS: u32 = (\d+);/m)[1])
if (!existsSync('tools/tools.json')) { console.error('No tools: run `npm run fetch-tools` first.'); process.exit(1) }
if (process.platform === 'linux') {
  const version = JSON.parse(readFileSync('src-tauri/tauri.conf.json', 'utf8')).version
  const made = join(process.env.CARGO_TARGET_DIR ?? 'src-tauri/target', 'release', 'bundle', 'appimage')
  const appdir = join(made, 'FilesViewHub.AppDir')
  const findImage = () => existsSync(made) ? readdirSync(made).find(f => f.endsWith('.AppImage') && f.includes(version)) : undefined
  // The notices list the libraries linuxdeploy puts in the AppImage and its runtime, known only
  // once it is made, yet they ship inside it: they are made from the last AppImage, and when the
  // new one differs it is made once more with notices made from it.
  const notices = () => {
    const image = findImage()
    const runtime = image && readFileSync(join(made, image)).subarray(0, 4 << 20).toString('latin1').match(/type2-runtime\/commit\/([0-9a-f]+)/)?.[1]
    // Without a last AppImage of this version (a first build, a new version) the notices leave the
    // AppImage's libraries out, and the second round puts them in.
    execSync(`node scripts/notices.mjs${existsSync(appdir) && runtime ? ` --appdir "${appdir}" --runtime ${runtime}` : ''}`, { stdio: 'inherit' })
    return readFileSync('tools/notices.json', 'utf8')
  }
  // A GPL 2 library (libjbig, through libtiff) is loaded into the app in the AppImage, so the app's
  // own source at the built commit goes inside it: this folder, but its test files.
  if (git('status --porcelain -- .')) {
    console.error('The app has changes not committed; the AppImage carries the source of the commit it is built from. Commit first.'); process.exit(1)
  }
  const commit = git('rev-parse --short=12 HEAD')
  rmSync('tools/source', { recursive: true, force: true })
  mkdirSync('tools/source', { recursive: true })
  execSync(`git -C "${repo}" archive --format=tar.gz --prefix=filesviewhub-${version}-${commit}/ -o "${resolve('tools/source', `filesviewhub-${version}-${commit}.tar.gz`)}" HEAD -- . ":(exclude)tests"`, { stdio: 'inherit' })
  let shipped = notices()
  for (let round = 1; ; round++) {
    execSync('npx tauri build --bundles appimage', { stdio: 'inherit' })
    const now = notices()
    if (now === shipped) break
    if (round === 2) { console.error('The notices still differ from the AppImage after a second build.'); process.exit(1) }
    console.log('\nThe AppImage holds other libraries than its notices list; making it again.\n')
    shipped = now
  }
  const build = buildNumber()
  const image = findImage()
  if (!image) { console.error(`No AppImage in ${made}.`); process.exit(1) }
  const out = join('release', `FilesViewHub-${version}-build-${build}-linux-x86_64.AppImage`)
  mkdirSync('release', { recursive: true })
  copyFileSync(join(made, image), out)
  chmodSync(out, 0o755)
  console.log(`\n${out}`)
  process.exit(0)
}
if (process.platform === 'darwin') {
  rmSync('tools/source', { recursive: true, force: true })
  execSync('node scripts/notices.mjs', { stdio: 'inherit' })
  // Ad-hoc signed only, until the Developer ID (issue #3).
  execSync('npx tauri build --bundles app --config src-tauri/macos-package.conf.json', { stdio: 'inherit' })
  const version = JSON.parse(readFileSync('src-tauri/tauri.conf.json', 'utf8')).version
  const app = join(process.env.CARGO_TARGET_DIR ?? 'src-tauri/target', 'release', 'bundle', 'macos', 'FilesViewHub.app')
  if (!existsSync(app)) { console.error(`No app in ${app}.`); process.exit(1) }
  const name = `FilesViewHub-${version}-build-${buildNumber()}-macos-${process.arch}`
  const stage = join('release', name)
  const dir = join(stage, 'FilesViewHub')
  rmSync(stage, { recursive: true, force: true })
  mkdirSync(dir, { recursive: true })
  // ditto keeps the bundle's links, modes and signature as they are.
  execFileSync('ditto', [app, join(dir, 'FilesViewHub.app')])
  execFileSync('codesign', ['--verify', '--deep', '--strict', join(dir, 'FilesViewHub.app')], { stdio: 'inherit' })
  copyFileSync('README.md', join(dir, 'README.md'))
  cpSync('README-files', join(dir, 'README-files'), { recursive: true })
  copyFileSync('LICENSE', join(dir, 'LICENSE.txt'))
  copyFileSync('tools/THIRD-PARTY-NOTICES.txt', join(dir, 'THIRD-PARTY-NOTICES.txt'))
  const dmg = join('release', `${name}.dmg`)
  rmSync(dmg, { force: true })
  // hdiutil on a busy runner can fail with "Resource busy"; it is tried three times.
  for (let i = 1; ; i++) {
    try { execFileSync('hdiutil', ['create', '-volname', 'FilesViewHub', '-srcfolder', stage, '-ov', '-format', 'UDZO', dmg], { stdio: 'inherit' }); break }
    catch (e) { if (i === 3) throw e; console.log('hdiutil failed; trying again.'); execFileSync('sleep', ['5']) }
  }
  rmSync(stage, { recursive: true, force: true })
  console.log(`\n${dmg}`)
  process.exit(0)
}
// The app's own source ships only in the AppImage (`decisions/2026/10/20/licences-and-notices-made-for-each-package.md`).
rmSync('tools/source', { recursive: true, force: true })
execSync('node scripts/notices.mjs', { stdio: 'inherit' })
execSync('npx tauri build --no-bundle', { stdio: 'inherit' })
const version = JSON.parse(readFileSync('src-tauri/tauri.conf.json', 'utf8')).version
const build = buildNumber()
const name = `FilesViewHub-${version}-build-${build}-windows`
const dir = join('release', name)
rmSync(dir, { recursive: true, force: true })
mkdirSync(dir, { recursive: true })
copyFileSync('src-tauri/target/release/filesviewhub.exe', join(dir, 'FilesViewHub.exe'))
copyFileSync('README.md', join(dir, 'README.md'))
cpSync('README-files', join(dir, 'README-files'), { recursive: true })
copyFileSync('LICENSE', join(dir, 'LICENSE.txt'))
copyFileSync('tools/THIRD-PARTY-NOTICES.txt', join(dir, 'THIRD-PARTY-NOTICES.txt'))
cpSync('tools', join(dir, 'tools'), { recursive: true, filter: f => !f.includes('.download') })
const zip = join('release', `${name}.zip`)
rmSync(zip, { force: true })
// Compress-Archive in Windows PowerShell 5.1 stores entry paths with backslashes, which other unzips
// read as one long file name; the zip spec wants forward slashes, so name each entry here.
const ps = (p) => `'${resolve(p).replaceAll("'", "''")}'`
const zipScript = `
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
Add-Type -AssemblyName System.IO.Compression, System.IO.Compression.FileSystem
$src = ${ps(dir)}
$zip = [IO.Compression.ZipFile]::Open(${ps(zip)}, 'Create')
try {
  Get-ChildItem -LiteralPath $src -Recurse -Force | Sort-Object FullName | ForEach-Object {
    $name = $_.FullName.Substring($src.Length + 1).Replace('\\', '/')
    if (-not $_.PSIsContainer) {
      [void][IO.Compression.ZipFileExtensions]::CreateEntryFromFile($zip, $_.FullName, $name, 'Optimal')
    } elseif (-not (Get-ChildItem -LiteralPath $_.FullName -Force)) {
      [void]$zip.CreateEntry("$name/")
    }
  }
} finally { $zip.Dispose() }
$read = [IO.Compression.ZipFile]::OpenRead(${ps(zip)})
try {
  $files = @(Get-ChildItem -LiteralPath $src -Recurse -Force -File).Count
  $stored = @($read.Entries | Where-Object { -not $_.FullName.EndsWith('/') }).Count
  if (@($read.Entries | Where-Object { $_.FullName.Contains('\\') }).Count) { throw 'A zip entry has a backslash in its path.' }
  if ($stored -ne $files) { throw "The zip holds $stored files, the folder $files." }
} finally { $read.Dispose() }
`
try {
  execSync(`powershell -NoProfile -EncodedCommand ${Buffer.from(zipScript, 'utf16le').toString('base64')}`, { stdio: 'inherit' })
} catch (e) { rmSync(zip, { force: true }); throw e }
console.log(`\n${zip}`)
