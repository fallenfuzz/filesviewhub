# The app links to its own source on GitHub, at the release tag

## Context

The app's code is GPL 3.0 or later. In the Linux AppImage, libjbig (jbigkit, GPL 2.0 or later,
through libtiff) is loaded into the app, so the AppImage must give the app's own source. While
the repository was private, the AppImage carried it as a `git archive` of the built commit in
`tools/source/` (`decisions/2026/10/20/licences-and-notices-made-for-each-package.md`). The
repository `fallenfuzz/filesviewhub` is now public, and a tag ruleset stops anyone from moving or
deleting a `filesviewhub-*` tag.

## Investigation

- GPL 3 section 6(d) lets object code offered from a network server give its source from another
  server, with clear directions next to the object code, for as long as the object code is
  offered. GPL 2 section 3 counts equivalent access from the same place as giving the source.
  The packages and the source both live on `github.com/fallenfuzz/filesviewhub`.
- A release is the tag `filesviewhub-<version>-build-<n>`, and `release.yml` builds its packages
  from that tag. A package made at a commit with no such tag (a local test) has no tag to link to.
- The package names a commit. With uncommitted changes in it, no link would match what was built.

## Decision

- `scripts/notices.mjs` puts "FilesViewHub itself" in the notices of every package (Windows,
  Linux, macOS): the version and build, GPL-3.0-or-later, the licence text, and two links. "The
  exact source" is `<repository>/tree/filesviewhub-<version>-build-<n>` when that tag points at
  the built commit, else `<repository>/tree/<commit>`. "Repository" is the repository itself. The
  repository comes from `repository` in `src-tauri/Cargo.toml`.
- The AppImage carries no copy of the app's source. `tools/source/` is gone.
- `npm run package` on Linux and on macOS refuses uncommitted changes in the app's folder. On
  Windows, `scripts/release.sh` checks with WSL git first, since Windows git sees line-ending and
  file-mode differences in clean files.
- A release links to its tag. On GitHub, a tag build stops when the tag is not the
  `filesviewhub-<version>-build-<n>` of its commit. `scripts/release.sh` sets `FVH_RELEASE=1`, and
  the build stops unless that tag is at HEAD and on origin, so a release is tagged and pushed
  before its packages are made.
- The build number is the one `src-tauri/build.rs` takes: `FVH_BUILD`, else the commit count plus
  the builds before this repository. Without a git checkout of the app, the notices stop the build.
- Release tags are never moved or deleted, so each link keeps working. A mistaken release gets a
  new build number.
- libjbig in the AppImage comes from Ubuntu, and its notice links to the Ubuntu source package on
  Launchpad. jbigkit is GPL 2.0 or later; the AppImage takes it under GPL 3, whose section 6(d)
  allows the source on a third party's server.

## Risks

- A package made from a commit that was never pushed links to a page that does not exist. Such a
  package is a local test; a release is built only from a pushed tag.
- If Ubuntu or Launchpad stops serving the jbigkit source package named, the AppImage must carry
  it, as the Windows zip carries its own.
- If the repository moves or is made private again, every released package's link breaks, and
  the Linux packages then give no source. Before either, the AppImage must carry the source again.

## Alternatives considered

- Keep the source in the AppImage and add the link: about 1 MB more for a copy the link already
  gives. The owner chose the link alone.
- Link to the commit always: it works for every build, but the tag is what the release page and
  the package names show, so a person can match them.
