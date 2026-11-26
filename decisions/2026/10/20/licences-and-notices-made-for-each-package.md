# Licences and notices are made for each package

The app's own code is GPL 3.0 or later. Each package also carries parts under other licences. The
Windows zip has FFmpeg with the libraries its build links in, PDFium, DjVuLibre and its DLLs, the
Rust crates, the npm packages in the window and the Golos font. The Linux AppImage has FFmpeg,
PDFium, the Ubuntu libraries linuxdeploy copies into it and the AppImage runtime. Most of their
licences ask for their text and copyright notice to go with a binary. The GPL and LGPL parts also
ask for a way to get their source.

`scripts/notices.mjs` makes `tools/notices.json` and `tools/THIRD-PARTY-NOTICES.txt` on every
`npm run package`, from what that build holds. The Rust crates come from `cargo metadata` for that
system, the npm packages from the modules Vite put in the page, and the tools from `tools.json`.
FFmpeg's libraries come from `scripts/ffmpeg-licences.json`, collected at the commit of BtbN's
build scripts. On Linux, each file in the AppDir is matched to its Ubuntu package, and each
package's copyright file comes from the system. A part with no licence text stops the build, and
so does a file that no package holds. The AppImage carries its own notices, so the AppImage is
built a second time when its libraries differ from the notices made from the last one.

The user chose how the source is given. The zip carries the exact MSYS2 source packages of
DjVuLibre and jbigkit, the GPL parts of ddjvu. The AppImage carries the app's own source at the
built commit (`git archive` of the app's folder, now the repository's root, without its tests), since libjbig (GPL 2) is loaded into the
app there, so `npm run package` on Linux refuses to run with uncommitted changes in it.
FFmpeg, PDFium and the Linux libraries get exact links: the commit, the release, or the Ubuntu
source package and version. The package has no written offer. BtbN's LGPL FFmpeg build links in
FFTW (GPL 2.0 or later, through Chromaprint), so its label says it is GPL as a whole. `ddjvu.exe`
is changed (its manifest asks for UTF-8), and `tools/djvulibre/CHANGES.txt` says how.

The FFmpeg libraries listed are those whose build scripts run for that system, and every library
the shipped `ffprobe -buildconf` names must be one of them, or the build stops.

Settings has Help, which renders `README.md` (made into HTML at build time, so there is one source
and no network), and Licences, which shows the app's licence and `notices.json`. A source link
opens in the web browser, and the app opens only links that `notices.json` holds.
