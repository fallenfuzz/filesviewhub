"""Makes small Insta360 files in tests/insta360/ for the photos.rs tests, from real ones: each
keeps the first 64 bytes and the trailer up to the info record, the record holding the time taken.
A record over 64 KB (the preview picture) is cut to 64 zero bytes and every other record before the
info record is zeroed. The info record keeps only the serial (as X), the model and the capture time,
set to a made-up one; every other field is zeroed at its length (file paths, UTC times, lens
calibration, GPS). The names hold no date, so the tests read the trailer.

    python3 tests/make_insta360_fixtures.py PHOTO.insp VIDEO.insv PREVIEW.lrv
    python3 tests/make_insta360_fixtures.py --scrub     scrubs the files already in tests/insta360/
"""
import struct, sys
from pathlib import Path

OUT = Path(__file__).resolve().parent / "insta360"
MAGIC = b"8db42d694ccc418790edff439fe026bf"
NAMES = {".insp": "photo.insp", ".insv": "video.insv", ".lrv": "preview.lrv"}

# Made-up capture times, written into field 7 of each file's info record (photos.rs tests read them).
DATES = {".insp": 20210304050607, ".insv": 20210304060708, ".lrv": 20210304070809}

def scrub(body: bytes, date: int) -> bytes:
    """Keeps only what the tests read from the info record (protobuf): field 1 (the serial, masked
    as X), field 2 (the camera model) and field 7, the capture time, set to `date`. Every other
    field keeps its length with its value zeroed: file paths, UTC times, lens calibration, GPS."""
    def varint(i: int) -> tuple[int, int]:
        v = shift = 0
        while True:
            c = body[i]; i += 1; v |= (c & 0x7F) << shift; shift += 7
            if c < 0x80:
                return v, i
    def same_length(v: int, n: int) -> bytes:
        out = bytearray()
        for k in range(n):
            out.append((v & 0x7F) | (0x80 if k < n - 1 else 0)); v >>= 7
        if v:
            raise ValueError("the made-up date does not fit in the field's length")
        return bytes(out)
    out, i = bytearray(body), 0
    while i < len(body):
        key, i = varint(i)
        field, wire = key >> 3, key & 7
        if wire == 0:
            start = i; _, i = varint(i)
            out[start:i] = same_length(date if field == 7 else 0, i - start)
        elif wire in (1, 5):
            n = 8 if wire == 1 else 4
            out[i:i + n] = bytes(n); i += n
        elif wire == 2:
            n, i = varint(i)
            if field == 1:
                out[i:i + n] = b"X" * n
            elif field != 2:
                out[i:i + n] = bytes(n)
            i += n
        else:
            raise ValueError(f"wire type {wire} in the info record")
    return bytes(out)

def scrub_file(path: Path) -> None:
    """Scrubs a fixture in place: the info record as above, every other trailer record zeroed."""
    d = bytearray(path.read_bytes())
    at = len(d) - 78
    while True:
        rid, n = struct.unpack("<HI", d[at:at + 6])
        body = bytes(d[at - n:at])
        d[at - n:at] = scrub(body, DATES[path.suffix.lower()]) if rid == 0x101 else bytes(n)
        if rid == 0x101:
            break
        at -= n + 6
    path.write_bytes(bytes(d))

def cut(src: Path) -> bytes:
    data_len = src.stat().st_size
    with src.open("rb") as f:
        head = f.read(64)
        f.seek(data_len - 78)
        foot = f.read(78)
        assert foot[46:] == MAGIC, f"{src} has no Insta360 trailer"
        records, at = [], data_len - 78
        while True:
            f.seek(at)
            rid, n = struct.unpack("<HI", f.read(6))
            f.seek(at - n)
            body = f.read(n) if n <= 65536 else bytes(64)
            if rid == 0x101:
                serial = body[2:2 + body[1]]
                assert body[0] == 0x0A and serial.isalnum(), "field 1 is not the serial number"
                body = scrub(body, DATES[src.suffix.lower()])
            elif n <= 65536:
                body = bytes(len(body))
            records.insert(0, body + struct.pack("<HI", rid, len(body)))
            if rid == 0x101:
                break
            at -= n + 6
    trailer = b"".join(records) + foot[6:38]
    trailer += struct.pack("<I", len(trailer) + 40) + foot[42:]
    return head + trailer

if sys.argv[1:2] == ["--scrub"]:
    for f in sorted(OUT.iterdir()):
        scrub_file(f)
        print("scrubbed", f.name)
    sys.exit()
OUT.mkdir(exist_ok=True)
for a in sys.argv[1:]:
    p = Path(a)
    (OUT / NAMES[p.suffix.lower()]).write_bytes(cut(p))
    print(p.name, "->", NAMES[p.suffix.lower()])
