# Decision records - where the old numbered names went

Records used to be `docs/adr/NNNN-slug.md`, cited as "ADR NNNN". They are now
`decisions/YYYY/MM/DD/slug.md`, dated by the day the record was first committed. The slug is the
identity; the number is gone.

**This table is the bridge for old citations.** A commit message, an issue or a comment that says
"ADR 0033" resolves here in one search. Numbers 0001 and 0003 were records of a retired tool that
is not part of this repository, so they have no row. Examples in the records written before 2026-11-25 that named real
folders or counted a real library were replaced with made-up ones when the repository was made public.

Why this shape: [`2026/11/25/decision-records-dated-not-numbered.md`](2026/11/25/decision-records-dated-not-numbered.md).

**Writing a new record?** `decisions/YYYY/MM/DD/your-slug.md`, dated the day you write it, never a
number. Sections: Context, Investigation, Decision, Risks, Alternatives considered. Do not add
rows to this table: it is a frozen map of what was renamed once, not a registry.

**Changing behavior an existing record describes?** Add a supersede note at its top -
"Superseded on YYYY-MM-DD by `decisions/YYYY/MM/DD/<new-slug>.md`: <one line on what changed>" -
and write the new record, in the same commit as the code change.

| Old name | New path |
|---|---|
| `0002-folder-shows-own-works-and-one-box-shape.md` | [`2026/09/29/folder-shows-own-works-and-one-box-shape.md`](2026/09/29/folder-shows-own-works-and-one-box-shape.md) |
| `0004-ticks-offer-the-mark-never-set-it.md` | [`2026/09/29/ticks-offer-the-mark-never-set-it.md`](2026/09/29/ticks-offer-the-mark-never-set-it.md) |
| `0005-hub-lists-every-file.md` | [`2026/09/30/hub-lists-every-file.md`](2026/09/30/hub-lists-every-file.md) |
| `0006-a-folder-is-a-work-or-a-place-never-both.md` | [`2026/09/30/a-folder-is-a-work-or-a-place-never-both.md`](2026/09/30/a-folder-is-a-work-or-a-place-never-both.md) |
| `0007-work-id-is-its-path.md` | [`2026/09/30/work-id-is-its-path.md`](2026/09/30/work-id-is-its-path.md) |
| `0008-settings-in-one-versioned-store.md` | [`2026/10/01/settings-in-one-versioned-store.md`](2026/10/01/settings-in-one-versioned-store.md) |
| `0009-marks-and-ticks-carry-when-they-were-set.md` | [`2026/10/01/marks-and-ticks-carry-when-they-were-set.md`](2026/10/01/marks-and-ticks-carry-when-they-were-set.md) |
| `0010-an-event-is-the-work-in-photos.md` | [`2026/10/04/an-event-is-the-work-in-photos.md`](2026/10/04/an-event-is-the-work-in-photos.md) |
| `0011-photo-ai-is-an-optional-separate-index.md` | [`2026/10/04/photo-ai-is-an-optional-separate-index.md`](2026/10/04/photo-ai-is-an-optional-separate-index.md) |
| `0012-sorting-ends-in-a-move-script-never-a-delete.md` | [`2026/10/04/sorting-ends-in-a-move-script-never-a-delete.md`](2026/10/04/sorting-ends-in-a-move-script-never-a-delete.md) |
| `0013-photos-take-no-marks-review-is-the-progress.md` | [`2026/10/04/photos-take-no-marks-review-is-the-progress.md`](2026/10/04/photos-take-no-marks-review-is-the-progress.md) |
| `0014-a-move-script-for-each-system.md` | [`2026/10/04/a-move-script-for-each-system.md`](2026/10/04/a-move-script-for-each-system.md) |
| `0015-filesviewhub-is-a-tauri-app-shipped-portable.md` | [`2026/10/05/filesviewhub-is-a-tauri-app-shipped-portable.md`](2026/10/05/filesviewhub-is-a-tauri-app-shipped-portable.md) |
| `0016-checked-folders-on-any-drive.md` | [`2026/10/05/checked-folders-on-any-drive.md`](2026/10/05/checked-folders-on-any-drive.md) |
| `0017-nothing-hidden-no-network-by-default.md` | [`2026/10/05/nothing-hidden-no-network-by-default.md`](2026/10/05/nothing-hidden-no-network-by-default.md) |
| `0018-app-data-local-cache-movable.md` | [`2026/10/05/app-data-local-cache-movable.md`](2026/10/05/app-data-local-cache-movable.md) |
| `0019-section-kind-guessed-and-changeable.md` | [`2026/10/05/section-kind-guessed-and-changeable.md`](2026/10/05/section-kind-guessed-and-changeable.md) |
| `0020-app-data-in-storeddata-next-to-the-app.md` | [`2026/10/07/app-data-in-storeddata-next-to-the-app.md`](2026/10/07/app-data-in-storeddata-next-to-the-app.md) |
| `0021-where-the-app-departs-from-the-hub.md` | [`2026/10/08/where-the-app-departs-from-the-hub.md`](2026/10/08/where-the-app-departs-from-the-hub.md) |
| `0022-a-folder-shows-its-subfolders-as-tiles.md` | [`2026/10/08/a-folder-shows-its-subfolders-as-tiles.md`](2026/10/08/a-folder-shows-its-subfolders-as-tiles.md) |
| `0023-photos-is-a-section-kind-in-the-app.md` | [`2026/10/08/photos-is-a-section-kind-in-the-app.md`](2026/10/08/photos-is-a-section-kind-in-the-app.md) |
| `0024-the-app-saves-the-move-script-for-its-own-system.md` | [`2026/10/08/the-app-saves-the-move-script-for-its-own-system.md`](2026/10/08/the-app-saves-the-move-script-for-its-own-system.md) |
| `0025-a-name-with-only-the-day-makes-an-event-of-that-day.md` | [`2026/10/09/a-name-with-only-the-day-makes-an-event-of-that-day.md`](2026/10/09/a-name-with-only-the-day-makes-an-event-of-that-day.md) |
| `0026-review-moves-set-aside-photos-itself-with-undo.md` | [`2026/10/09/review-moves-set-aside-photos-itself-with-undo.md`](2026/10/09/review-moves-set-aside-photos-itself-with-undo.md) |
| `0027-the-app-imports-nothing-from-the-hub.md` | [`2026/10/09/the-app-imports-nothing-from-the-hub.md`](2026/10/09/the-app-imports-nothing-from-the-hub.md) |
| `0028-records-matched-to-the-built-app.md` | [`2026/10/09/records-matched-to-the-built-app.md`](2026/10/09/records-matched-to-the-built-app.md) |
| `0029-copies-include-pictures-saved-again.md` | [`2026/10/09/copies-include-pictures-saved-again.md`](2026/10/09/copies-include-pictures-saved-again.md) |
| `0030-a-folder-shows-its-subfolders-as-rows-that-open.md` | [`2026/10/14/a-folder-shows-its-subfolders-as-rows-that-open.md`](2026/10/14/a-folder-shows-its-subfolders-as-rows-that-open.md) |
| `0031-each-section-picks-its-view-at-each-level.md` | [`2026/10/14/each-section-picks-its-view-at-each-level.md`](2026/10/14/each-section-picks-its-view-at-each-level.md) |
| `0032-a-folder-takes-its-own-kind-or-becomes-a-section.md` | [`2026/10/14/a-folder-takes-its-own-kind-or-becomes-a-section.md`](2026/10/14/a-folder-takes-its-own-kind-or-becomes-a-section.md) |
| `0033-licences-and-notices-made-for-each-package.md` | [`2026/10/20/licences-and-notices-made-for-each-package.md`](2026/10/20/licences-and-notices-made-for-each-package.md) |
| `0034-a-whole-drive-can-be-a-section-except-the-system-drive.md` | [`2026/10/21/a-whole-drive-can-be-a-section-except-the-system-drive.md`](2026/10/21/a-whole-drive-can-be-a-section-except-the-system-drive.md) |
| `0035-junk-alone-never-waits-for-review.md` | [`2026/10/21/junk-alone-never-waits-for-review.md`](2026/10/21/junk-alone-never-waits-for-review.md) |
| `0036-the-view-menu-sets-each-folders-view.md` | [`2026/10/21/the-view-menu-sets-each-folders-view.md`](2026/10/21/the-view-menu-sets-each-folders-view.md) |
| `0037-covers-are-chosen-never-fetched-by-themselves.md` | [`2026/10/22/covers-are-chosen-never-fetched-by-themselves.md`](2026/10/22/covers-are-chosen-never-fetched-by-themselves.md) |
| `0038-review-has-its-own-window.md` | [`2026/10/27/review-has-its-own-window.md`](2026/10/27/review-has-its-own-window.md) |
