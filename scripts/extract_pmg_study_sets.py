#!/usr/bin/env python3
"""Extract Scripture Study citations from Preach My Gospel, 2nd edition."""

import argparse
import json
import re
import subprocess
import tempfile
from pathlib import Path

CHAPTERS = [
    (403, 1807, "Fulfill Your Missionary Purpose"),
    (1807, 2493, "Search the Scriptures and Put on the Armor of God"),
    (2493, 9147, "Study and Teach the Gospel of Jesus Christ"),
    (9147, 10174, "Seek and Rely on the Spirit"),
    (10174, 11085, "Use the Power of the Book of Mormon"),
    (11085, 12540, "Seek Christlike Attributes"),
    (12540, 13134, "Learn Your Mission Language"),
    (13134, 14310, "Accomplish the Work through Goals and Plans"),
    (14310, 15969, "Find People to Teach"),
    (15969, 18001, "Teach to Build Faith in Jesus Christ"),
    (18001, 18718, "Help People Make and Keep Commitments"),
    (18718, 19604, "Help People Prepare for Baptism and Confirmation"),
    (19604, 10**9, "Unite with Leaders and Members to Establish the Church"),
]

STOP_PREFIXES = (
    "Personal Study",
    "Companion Study",
    "Personal or Companion Study",
    "Activity",
    "Teaching Insight",
    "Learn More",
    "Remember This",
    "Ideas for Study",
    "Invitations to",
)


def load_catalog(root: Path):
    files = {
        "OldTestament": root / "assets/data/old-testament-flat.json",
        "NewTestament": root / "assets/data/new-testament-flat.json",
        "BookOfMormon": root / "assets/data/book-of-mormon-flat.json",
        "DoctrineAndCovenants": root / "assets/data/doctrine-and-covenants-flat.json",
        "PearlOfGreatPrice": root / "assets/data/pearl-of-great-price-flat.json",
    }
    catalog = {}
    for canon, path in files.items():
        for item in json.loads(path.read_text())["verses"]:
            match = re.match(r"(.+) (\d+):(\d+)$", item["reference"])
            book, chapter, verse = match.group(1), int(match.group(2)), int(match.group(3))
            catalog.setdefault(book, {"canon": canon, "chapters": {}})["chapters"].setdefault(
                chapter, []
            ).append(verse)
    return catalog


def source_chapter(line_number):
    for number, (start, end, title) in enumerate(CHAPTERS, 1):
        if start <= line_number < end:
            return number, title
    raise ValueError(f"Scripture Study box outside known chapters at line {line_number}")


def study_blocks(lines):
    for index, line in enumerate(lines):
        if line.strip() != "Scripture Study":
            continue
        block = []
        found_reference = False
        # Raw PDF text sometimes emits the notes column before the references
        # on the facing column, so a box can span roughly one page of lines.
        for candidate in lines[index + 1 : index + 121]:
            text = candidate.strip()
            if text.startswith(STOP_PREFIXES):
                break
            if found_reference and len(text) > 90 and "?" not in text:
                break
            block.append(text)
            if re.search(r"\d+(?::\d+)?", text):
                found_reference = True
        yield index + 1, block


def book_pattern(catalog):
    aliases = {"Doctrine and Covenants": "D&C", "Psalm": "Psalms"}
    names = sorted(set(catalog) | set(aliases), key=len, reverse=True)
    return re.compile(r"(?<![A-Za-z])(" + "|".join(re.escape(name) for name in names) + r")\s+"), aliases


def reference_segments(line, pattern, aliases, inherited_book):
    matches = list(pattern.finditer(line))
    segments = []
    for index, match in enumerate(matches):
        end = matches[index + 1].start() if index + 1 < len(matches) else len(line)
        book = aliases.get(match.group(1), match.group(1))
        segments.append((book, line[match.end() : end]))
    if not matches and inherited_book and re.match(r"^\d+(?::|\s*[;–-])", line):
        segments.append((inherited_book, line))
    return segments


def parse_segment(book, segment, catalog):
    clean = segment.replace("–", "-")
    clean = re.split(r"[^0-9:;,\-\s]", clean, maxsplit=1)[0].strip(" ;,")
    if not clean:
        return []
    passages = []
    for part in [part.strip() for part in clean.split(";") if part.strip()]:
        if ":" in part:
            chapter_text, verses_text = part.split(":", 1)
            if not chapter_text.strip().isdigit():
                continue
            chapter = int(chapter_text)
            verses = []
            for item in verses_text.split(","):
                item = item.strip()
                if not item:
                    continue
                if "-" in item:
                    start, end = [int(value.strip()) for value in item.split("-", 1)]
                    verses.extend(range(start, end + 1))
                elif item.isdigit():
                    verses.append(int(item))
            passages.append(make_passage(book, chapter, verses, catalog))
        elif re.fullmatch(r"\d+", part):
            chapter = int(part)
            passages.append(make_passage(book, chapter, None, catalog))
        elif re.fullmatch(r"\d+\s*-\s*\d+", part):
            start, end = [int(value.strip()) for value in part.split("-", 1)]
            for chapter in range(start, end + 1):
                passages.append(make_passage(book, chapter, None, catalog))
    return passages


def make_passage(book, chapter, verses, catalog):
    if book not in catalog or chapter not in catalog[book]["chapters"]:
        raise ValueError(f"Unknown scripture chapter: {book} {chapter}")
    available = catalog[book]["chapters"][chapter]
    selected = available if verses is None else verses
    missing = sorted(set(selected) - set(available))
    if missing:
        raise ValueError(f"Unknown verses in {book} {chapter}: {missing}")
    return {
        "canon": catalog[book]["canon"],
        "book": book,
        "chapter": chapter,
        "verses": selected,
    }


def extract(root, pdf):
    catalog = load_catalog(root)
    pattern, aliases = book_pattern(catalog)
    with tempfile.NamedTemporaryFile(suffix=".txt") as text_file:
        subprocess.run(["pdftotext", "-raw", str(pdf), text_file.name], check=True)
        lines = Path(text_file.name).read_text(errors="replace").splitlines()

    chapter_passages = {number: [] for number in range(1, 14)}
    audit = []
    for line_number, block in study_blocks(lines):
        chapter_number, chapter_title = source_chapter(line_number)
        inherited_book = None
        extracted = []
        for line in block:
            segments = reference_segments(line, pattern, aliases, inherited_book)
            for book, segment in segments:
                inherited_book = book
                extracted.extend(parse_segment(book, segment, catalog))
        if not extracted:
            raise ValueError(f"No scripture references found near line {line_number}: {block}")
        audit.append({"line": line_number, "chapter": chapter_number, "passages": len(extracted)})
        for passage in extracted:
            if passage not in chapter_passages[chapter_number]:
                chapter_passages[chapter_number].append(passage)

    sets = []
    all_passages = []
    for number, (_, _, title) in enumerate(CHAPTERS, 1):
        passages = chapter_passages[number]
        if not passages:
            continue
        all_passages.extend(passage for passage in passages if passage not in all_passages)
        sets.append(
            {
                "id": f"preach-my-gospel-chapter-{number}",
                "name": f"PMG {number}: {title}",
                "passages": passages,
            }
        )
    return {
        "source": pdf.name,
        "boxes": audit,
        "unique_passage_count": len(all_passages),
        "sets": sets,
    }


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("pdf", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[1]
    result = extract(root, args.pdf)
    args.output.write_text(json.dumps(result, indent=2) + "\n")
    print(
        f"Extracted {len(result['boxes'])} boxes, "
        f"{result['unique_passage_count']} unique passages"
    )


if __name__ == "__main__":
    main()
