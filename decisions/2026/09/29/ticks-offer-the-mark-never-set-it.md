# Ticks inside a Work offer its Mark but never set it

A Work with lessons, tracks or titles inside (a course, an audiobook, a collection) has a Tick on
each item and one Mark of its own. Ticking every item does not mark the Work Done: the dialog
then offers "Mark ... Watched" as one click, and shows the Work's Mark next to the progress so
it can be set there by hand at any time. The dialog opens at the first item not yet ticked.

The Mark means "I decided this is next" or "I am done with it", which a count of ticks cannot
know: people skip lessons they already know, or finish a course without ticking anything. Keeping
the two apart also keeps the glossary's rule that a Mark is never set automatically, so an
imported set of ticks never changes what Next up and the Done filter show.

## Considered Options

- The last tick sets Done and the first tick sets Next: the Mark follows the ticks, but a skipped
  lesson keeps a finished course out of Done, and the Mark stops being the person's own choice.
- Separate and silent (the earlier behaviour): a fully ticked course still reads "+ Mark", and the
  dialog had no way to set the Mark at all.
