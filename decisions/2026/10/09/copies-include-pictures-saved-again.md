# Copies include a picture saved again

A photo sent through a messenger, or exported again, comes back as a new file: smaller, or at
another JPEG quality, often with a later date. Copies found only the same bytes, so Review never
offered these. On a real photo folder of a few thousand JPEGs, about 2 in every 100 pictures had such a pair
more than 10 minutes apart, where Similar shots never looks.

A picture now counts as a Copy of another when, at any date, both have the same shape (width over
height within 0.02), their fingerprints differ by at most 3 of 64 bits, and their 9 by 8 grey
pictures differ by at most 1.0 per pixel on average (0 to 255). On that folder, pictures saved again
differ by under 0.6 and different pictures by over 1.8; the fingerprint alone is not enough, as two
different photos there share one exactly. The one kept is the largest that nothing else suggests
setting aside, then the first. Two shots of one set of Similar shots stay in that set.

The 9 by 8 grey picture is kept beside each fingerprint in `photo-shots.json`; a picture measured
before it existed is read once more. Still no AI: plain arithmetic on pixels (`decisions/2026/10/04/sorting-ends-in-a-move-script-never-a-delete.md`).
