# Sorting photos ends in a script that moves files, never one that deletes them

Sorting an Event in the Hub sets aside the Photos a person does not need: the Similar shots they did
not keep and the Junk they agreed with. The page cannot touch files, so it exports two things: a plain
list, and a PowerShell script the person reads and runs from the photo folder. The script only moves
those files into a `Not needed` folder inside it, and never deletes anything. The person looks through
that folder and empties it by hand, so every step can be undone until then. This changes the first
Photos plan, which allowed a report and filters only.

## Considered Options

- A list only: the safest, but nobody deletes 800 files by hand from a list, so the sorting is wasted.
- A script that deletes: one step fewer, but a wrong guess (a dark concert photo marked Junk) is gone
  for good.
- Moving into a folder outside the photo folder: keeps the folder clean, but splits a person's photos
  over two places and makes the move harder to undo.

## Consequences

- The `Not needed` folder sits inside the Section folder, so the next build lists its files as
  Photos again unless the build skips that folder by name.
- The set-aside list lives in the browser like the Marks, so it needs the same export and import to
  survive a browser reset.
