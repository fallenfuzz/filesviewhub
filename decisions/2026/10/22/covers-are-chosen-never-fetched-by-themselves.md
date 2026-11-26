# Covers are chosen, never fetched by themselves

This changes the network part of `decisions/2026/10/05/nothing-hidden-no-network-by-default.md`. A book with no cover in its files was looked up on
Open Library and FantLab by itself once a source's switch was on, by the title and author read
from its file name. On the owner's folders that sent requests for things that are not books or
were read wrongly: a magazine issue ("esli 2015 02 nopic"), a title searched as its author
("Python Crash Course ... - Eric Matthes" asked Open Library for a book called "Eric Matthes"),
and the leftovers of a double extension ("Caliban-s-War.524185.fb2.mobi" searched with "fb2").
A guess from a file name cannot tell those apart, so the guess goes.

The app now goes online only when the person asks, on one card or folder tile:

- **Choose a cover…** is in every card's menu and every folder tile's menu, including a
  Section's tile. It opens one picker with two parts: a picture of one's own (a file, or one
  pasted) and covers from the web.
- The web part shows search words made from the name, which the person can change. Nothing is
  sent until Search is pressed. Search asks both Open Library and FantLab and shows each one's
  covers apart, with their small pictures. Only the cover picked is fetched in full.
- A chosen cover, from a file, the clipboard or the web, is kept as a picture in StoredData
  (`Covers/`, listed in `covers.json` by Work id or folder), so it survives a restart and the
  cache being deleted. It replaces the cover the files give.
- Remove this cover (in the picker, and Not this cover on a web cover's chip) goes back to the
  cover the files give. A picture no cover uses any more is deleted at the next start.

The two switches in Settings are gone: pressing Search is the consent, each time. Every request,
and every redirect it is sent on, is still written to the network log before it is sent, and
"What this app does" names each host. With no Search pressed, the app makes no network call.

Covers found by the old lookups stay, as chosen covers from their site, made from the thumbnail
already in the cache. One whose thumbnail was never made, and a cover the person had refused,
are dropped.

## Considered Options

- Keep the automatic lookup and tighten the guess (skip magazines, read "Title - Author" too):
  each rule fixes the names seen so far, and the next folder brings new ones.
- Keep a switch per source, or one switch for both, on top of the picker: a second consent for
  the same click, which only hides the picker's web part.
- Search as soon as the picker opens: one click less, but opening the picker only to pick a file
  of one's own would send the name to two sites.
