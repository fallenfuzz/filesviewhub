"""Makes one file of each kind the thumbnail code reads, in .scratch/fixtures/, for the
`fixtures` test in src-tauri/src/thumbs.rs. Needs c44, soffice, pandoc and ffmpeg in WSL.
`--books FOLDER` also copies the first mobi and azw3 found there (none can be made here).

    python3 tests/make_fixtures.py [--books FOLDER]
"""
import argparse, base64, io, shutil, subprocess, zipfile
from pathlib import Path
from PIL import Image, ImageDraw
import pillow_heif

pillow_heif.register_heif_opener()
F = Path(__file__).resolve().parents[1] / ".scratch" / "fixtures"
F.mkdir(exist_ok=True)

def art(w, h, colour, label):
    im = Image.new("RGB", (w, h), colour)
    d = ImageDraw.Draw(im)
    for i in range(0, w, 40):
        d.line([(i, 0), (w - i, h)], fill="white", width=3)
    d.rectangle([w // 4, h // 4, w * 3 // 4, h * 3 // 4], outline="black", width=8)
    d.text((20, 20), label, fill="black")
    return im

def jpeg(im):
    b = io.BytesIO(); im.save(b, "JPEG"); return b.getvalue()

# A photo stored landscape with EXIF orientation 6: it must come out portrait.
im = art(800, 600, "#3a7bd5", "rotated")
ex = Image.Exif(); ex[0x0112] = 6
im.save(F / "photo-rotated.jpg", exif=ex)
art(900, 600, "#c2456b", "heic").save(F / "photo.heic")
t = Image.new("RGBA", (400, 400), (0, 0, 0, 0)); ImageDraw.Draw(t).ellipse([50, 50, 350, 350], fill=(200, 30, 30, 255))
t.save(F / "transparent.png")

# A PDF whose first page is blank: the cover is page 2.
Image.new("RGB", (600, 800), "white").save(F / "Книга blank-first.pdf", save_all=True, append_images=[art(600, 800, "#2e9e6a", "page 2")])
(F / "broken.pdf").write_bytes(b"%PDF-1.4\nthis is not a pdf\n")

# DjVu from a picture (c44 needs a PPM).
art(600, 800, "#8a63c2", "djvu").save("/tmp/djvu.ppm")
subprocess.run(["c44", "/tmp/djvu.ppm", str(F / "book.djvu")], check=True)

# epub: the cover named by <meta name="cover">, not the first picture.
with zipfile.ZipFile(F / "book.epub", "w") as z:
    z.writestr("mimetype", "application/epub+zip")
    z.writestr("META-INF/container.xml", '<?xml version="1.0"?><container xmlns="urn:oasis:names:tc:opendocument:xmlns:container"><rootfiles><rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/></rootfiles></container>')
    z.writestr("OEBPS/content.opf", '<?xml version="1.0"?><package xmlns="http://www.idpf.org/2007/opf"><metadata><meta name="cover" content="c1"/></metadata><manifest><item id="i0" href="Images/illustration.jpg" media-type="image/jpeg"/><item id="c1" href="Images/front%20page.jpg" media-type="image/jpeg"/></manifest></package>')
    z.writestr("OEBPS/Images/illustration.jpg", jpeg(Image.new("RGB", (300, 400), "#888888")))
    z.writestr("OEBPS/Images/front page.jpg", jpeg(art(600, 900, "#d9822b", "epub cover")))

# fb2 in windows-1251 with its cover and an illustration.
cover = base64.b64encode(jpeg(art(600, 900, "#2a9ba8", "fb2"))).decode()
ill = base64.b64encode(jpeg(Image.new("RGB", (300, 400), "#888888"))).decode()
fb2 = f'''<?xml version="1.0" encoding="windows-1251"?>
<FictionBook xmlns="http://www.gribuser.ru/xml/fictionbook/2.0" xmlns:l="http://www.w3.org/1999/xlink">
<description><title-info><book-title>Книга</book-title><coverpage><image l:href="#cover.jpg"/></coverpage></title-info></description>
<body><p>Текст</p></body>
<binary id="ill.jpg" content-type="image/jpeg">{ill}</binary>
<binary id="cover.jpg" content-type="image/jpeg">
{cover}
</binary></FictionBook>'''
(F / "book.fb2").write_bytes(fb2.encode("cp1251"))

# A comic: pages sort by number, p1 first.
with zipfile.ZipFile(F / "comic.cbz", "w") as z:
    z.writestr("p10.jpg", jpeg(Image.new("RGB", (300, 400), "#444444")))
    z.writestr("p2.jpg", jpeg(Image.new("RGB", (300, 400), "#444444")))
    z.writestr("p1.jpg", jpeg(art(600, 900, "#9c7b2e", "p1")))
    z.writestr(".hidden.jpg", b"x")

# Office: an odt saved by LibreOffice keeps a preview; a pandoc docx has none.
Path("/tmp/doc.md").write_text("# A handout\n\nSome text on the first page.\n")
subprocess.run(["pandoc", "/tmp/doc.md", "-o", str(F / "no-preview.docx")], check=True)
subprocess.run(["soffice", "--headless", "--convert-to", "odt", "--outdir", str(F), str(F / "no-preview.docx")], check=True, capture_output=True)
shutil.move(F / "no-preview.odt", F / "handout.odt")

# Video: 2 s of black, then a test picture: the 10 % frame is black and is skipped.
subprocess.run(["ffmpeg", "-v", "error", "-y", "-f", "lavfi", "-i", "color=black:s=640x360:d=2", "-f", "lavfi", "-i", "testsrc=s=640x360:d=18",
                "-filter_complex", "[0][1]concat=n=2:v=1", "-pix_fmt", "yuv420p", str(F / "black-start.mp4")], check=True)
art(500, 500, "#5b6bd5", "album").save("/tmp/album.jpg")
subprocess.run(["ffmpeg", "-v", "error", "-y", "-f", "lavfi", "-i", "sine=d=3", "-i", "/tmp/album.jpg", "-map", "0", "-map", "1",
                "-c:v", "copy", "-id3v2_version", "3", "-disposition:v", "attached_pic", str(F / "with-cover.mp3")], check=True)
subprocess.run(["ffmpeg", "-v", "error", "-y", "-f", "lavfi", "-i", "sine=d=3", str(F / "no-cover.mp3")], check=True)
ap = argparse.ArgumentParser()
ap.add_argument("--books", type=Path)
books = ap.parse_args().books
for ext in ("mobi", "azw3") if books else ():
    found = next(books.rglob(f"*.{ext}"), None)
    if found:
        shutil.copy(found, F / f"book.{ext}")
print(sorted(p.name for p in F.iterdir()))
