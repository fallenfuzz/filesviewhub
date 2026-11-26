# Nothing is hidden: every store, tool and network call is listed in the app

Settings has a "What this app does" screen listing every place FilesViewHub stores data (with its
size and an Open folder button), every bundled tool (version, licence, what it is used for),
and every network feature with the exact host it contacts. Every network feature is off by
default; with all of them off the app makes no network call at all. A network log lists each
request made, with its time, address and reason. A feature that is not on that screen does not
exist. No telemetry; a local log file and a Copy log button in About.

The two network features today are book covers from Open Library (English titles) and FantLab
(Cyrillic titles), each its own switch. Goodreads is not used: its API closed in 2020 and scraping
breaks its terms.

## Consequences

- Every new feature that stores data, runs a tool or uses the network is added to that screen in
  the same change, or it does not ship.
