# Review moves the photos set aside itself, with Undo

A person who is not technical will not run a PowerShell script on their family photos, so the sorting
in Review went unused. The app now moves the photos set aside itself when the person asks and
confirms, and keeps an Undo. This is the one place where the app writes into a checked folder,
which `decisions/2026/10/05/app-data-local-cache-movable.md` forbids and `decisions/2026/10/08/the-app-saves-the-move-script-for-its-own-system.md` avoided by saving a script. Nothing is ever deleted outright.

The default place is `Not needed` inside the photo folder, as the script uses. The person can pick
another folder instead, outside every checked folder, and the app remembers it for that Section. On
the same drive a photo is renamed into place. On another drive it is copied, the copy is checked by
size and hash, and only then is the original removed. Every move is written to a journal in StoredData
before the file moves, so Undo can put each photo back and a move cut short by a crash is finished or
reversed on the next start. Undo puts a photo back only while it still matches the journal (same size
and date) and its old place is free. A photo only in the cloud is skipped and counted, as moving it
might download it. The move script stays, as Save as script, for a person who wants to read it first.

## Considered Options

- The Recycle Bin: the easiest to understand, but files on network shares and USB sticks are deleted
  for good, and a delete in Dropbox or OneDrive removes the photo from every device.
- The script only (`decisions/2026/10/08/the-app-saves-the-move-script-for-its-own-system.md`): safe and readable, but most people never run it.
- Any folder, inside a checked folder too: the moved photos would be read again as photos of that
  Section, or of another one.

## Consequences

- A folder outside Dropbox or OneDrive makes the move a deletion to the sync client, so the photos go
  from the person's other devices; the dialog says so when the chosen folder is outside the sync folder.
- What this app does lists the move, the journal and the folder each Section moves into (`decisions/2026/10/05/nothing-hidden-no-network-by-default.md`).
