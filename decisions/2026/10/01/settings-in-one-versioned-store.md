# Settings live in one versioned object, exported with the Marks

The Hub's preferences (card size, autohide, show Unlisted files, folder column widths, what a
folder with subfolders shows) are one localStorage object, `hub-settings`, shaped
`{v: 1, view, autohide, unlisted, colw, folders}`. The Marks export carries it as `settings`, and
importing a backup that has it replaces the current settings, while Marks and Ticks are merged as
before. When `hub-settings` is missing, the page reads the older per-setting `hub-*` keys once and
writes the object; the old keys stay, for an older page still open in another tab.

The app will import what people already have in the page. One object with a version is one thing to
read and migrate, where five keys each with their own encoding ("1", a word, a JSON map) are five.
A backup that restores Marks but not the setup around them is half a backup.

## Considered Options

- Keep a key per setting: nothing to migrate now, but the app inherits every encoding and the
  export still leaves settings behind.
- Merge settings on import, keeping the current value where the backup has none: every field is
  always present in an export, so this is the same as replacing, with more code.
- Delete the old keys after carrying them over: tidier storage, but an older page open in another
  tab would fall back to its defaults.
