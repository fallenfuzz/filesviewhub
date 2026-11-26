# Where the app departs from the Hub, on purpose

FilesViewHub does what the Hub page does (`decisions/2026/10/05/filesviewhub-is-a-tauri-app-shipped-portable.md`). In these places it behaves or looks
differently by choice, so a later change does not "fix" them back:

- **Next is yellow, Done is green.** The Hub's Next was blue with white text. In the app, blue is
  the accent for focus, links and selection, so Next takes a colour no control uses.
- **Next up lists the newest Mark first.** The Hub ordered it by Section and path. A list of
  what to do next reads best with the most recent intent on top.
- **A search typed inside a folder searches that folder.** "Search everywhere" sits next to the
  results. The library column's lists (Next up, Done, Recently added, Unlisted) clear the search,
  as the Hub's did.
- **Ctrl+K and / both jump to search.** The search box shows the Ctrl+K hint.
- **Done is its own list,** next to Next up. The Mark filter is offered only where it narrows
  something: not on Next up, Done or Unlisted.
- **One chrome surface around a raised content pane.** The title bar, library column and status
  bar share one colour. The folder columns and cards sit on a lighter pane with a rounded corner,
  as Windows 11 apps draw them. Selection is a tint of the accent, never a black block.
- **Quiet cards.** The Mark badge and the File Explorer button show on hover or keyboard focus
  until a Work has a Mark. The Section's colour band shows only in lists where Sections mix.
  Folders use line icons, not emoji.
- **The typeface is Golos Text, bundled.** The app looks the same on Windows and macOS and reads
  Cyrillic well. Segoe UI Variable would look more native on Windows only.

## Considered Options

- Match the Hub exactly: the blue Next would compete with focus rings and links, and black
  selected states pull the eye from the thumbnails.
- Native Windows controls throughout (Fluent, Mica): ties the look to one system before the macOS
  build exists.

## Consequences

- A Hub Marks backup still imports: Mark colours and list order are display only.
- A reviewer comparing the two finds these differences listed here, not as defects.
