# Junk alone never waits for Review

This changes the Junk part of `decisions/2026/10/04/photos-take-no-marks-review-is-the-progress.md`. Junk is a guess from a photo's name and numbers (a
screenshot, a WhatsApp picture, a very dark or blurred shot), and a single guess made a photo wait
for Review as a step of its own. On the owner's photo folder, a 20:9 camera photo sent over WhatsApp
(720 by 1600, its camera details taken out) was called a screenshot and made its month say
"Review 2 photos". A lone photo has nothing to compare it with, so Review asks the person about a
guess, not a choice between pictures.

Review now covers only Copies and Similar shots. Junk stays a label: its chip on the photo, the
Probably not needed filter, and Best only hides it. A Junk photo in a set of Copies or Similar
shots waits with its set, as any photo there does. A choice made on a Junk photo before stays, and
Review again in its menu forgets it.

A picture 720 by 1600 with no camera details is no longer called a screenshot by its size, since
WhatsApp sends a 20:9 photo at that size. A real screenshot from a 720 by 1600 phone screen then
gets the label only by its name (`Screenshot_...`); as Junk no longer waits, a missed label costs
only the chip.

## Considered Options

- Leave screenshots out of the count but keep them as Review steps: the steps and the counts would
  disagree.
- Stop only screenshots waiting: a lone very dark or blurred photo is the same single guess.
