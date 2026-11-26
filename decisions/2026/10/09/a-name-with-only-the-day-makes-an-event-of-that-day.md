# A name with only the day makes an Event of that day

A Photo's date comes from its file name, then EXIF (`decisions/2026/10/08/photos-is-a-section-kind-in-the-app.md`). WhatsApp names its files
`IMG-20200705-WA0012.jpg`: the day, no time, and it strips EXIF, so such a Photo had no date and went
to Undated although its day is in its name. Now a Photo whose name gives the day but whose name and
EXIF give no time joins an Event of its own for that day: one per day, with the day as its title and
"Time not known" where the times go. It never joins a timed Event of the same day.

## Considered Options

- Undated, as before: the day is in the name, so the Photo is lost from the calendar for no reason.
- A made-up time (midnight or noon): the Photo would join whatever Event runs at that time, a false
  Event, which is why `decisions/2026/10/08/photos-is-a-section-kind-in-the-app.md` never invents a time.
- The file's modified time: never used for a date (`decisions/2026/10/08/photos-is-a-section-kind-in-the-app.md`); a synced or copied file has the copy's time.
