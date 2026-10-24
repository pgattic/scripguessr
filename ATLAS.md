# Atlas data and interaction decisions

## What the live corpus supports

Investigation against API v1, release `data-2026-10-11-143950`, found 1,709
Book of Mormon entities with usable annotations. The entity-type hierarchy
organizes them into seven populated families: people, groups, topics, places,
objects, events, and offices. Places include cities, lands, water, mountains,
and wildernesses; objects include records. The application discovers these
families recursively rather than maintaining a category list.

The same edition has 941 speech annotations and 484 scene-setting annotations.
Those provide a useful distinction between someone being referenced, someone
speaking or narrating, and a place being the setting. These are included in an entry's
chapter coverage and are identified separately in the reader.

Names alone are insufficient: the corpus contains multiple people named Nephi,
Alma, and Lehi, and cities and lands can share a name. Entries always carry their
upstream descriptions. Search considers names, descriptions, and types, with
exact names ranked ahead of descriptive matches.

The edition has 18 books and 242 chapter units, including its title page and
witness testimonies. Book order and chapter numbers are read from the edition.

## Interface

The Atlas is a separate study feature at `/atlas`. It has no dependency on game
rules, account ownership, or study-set generation. The only shared application
boundary is the corpus HTTP client and site navigation.

The map retains the original geometry: 181px-wide book blocks containing five
columns of 33px chapter buttons with 4px gaps; CSS columns balance whole books
without splitting their chapter grids. Canonical order flows down each column.
The filter sidebar is independent and sticky on desktop, with an accessible
drawer on narrow screens.

Selecting an entry highlights every chapter touched by its annotations. Up to
eight entries can be compared; matches are combined as a union, colored bands
show individual coverage, and count badges identify overlaps. Filtering and
category expansion are local operations and make no upstream requests.

Click any chapter to read it, including without selecting filters. Selected
entries' descriptions and verse ranges explain why the chapter is highlighted.
The reader separates references, speech, and setting evidence, and marks the
matching verses. A First match control jumps to the first relevant passage.
Unnumbered headings are available in an expandable section.

The corpus's own annotations determine relevance. A highlighted reference can
include pronouns or a passage about an entity; it is not necessarily a literal
name match. Unmarked chapters are not evidence that a subject is absent. This
is explained in the interface's About highlights disclosure.

## Data handling

The initial catalog requires book/chapter metadata and annotated entity metadata.
Selecting an entry fetches its passage ranges; opening a chapter fetches its text.
All queries are read-only, and pagination follows actual returned row counts.

Range endpoints are joined through words and verses. SQL uses word `sequence`
to validate order, never numerical word IDs. The model expands a span into every
intersected chapter in edition order and clips its verse boundaries correctly,
including across books. Disjoint mentions remain disjoint; they do not turn the
intervening verses into matches. Adjacent or overlapping ranges of the same
kind are merged for display.

The Atlas cache keeps catalog and normalized coverage for 15 minutes with a
160-entry bound. A separate 160-entry chapter cache is shared with game readers.
Each cache runs at most four supervised fetches, admits at most 32 outstanding
keys and 64 callers per key, and enforces a 60-second deadline including queue
time. Identical requests share a fetch; cached hits remain available during slow
requests. Failed requests are not cached and have retry states.

Chapter text and highlights load independently. Changing filters keeps the open
chapter visible, and switching or closing chapters cancels obsolete LiveView
loads. Both views render the same chapter-reader component, with Atlas evidence
and game answer highlighting supplied by their respective views. Every result
must match the catalog release. A corpus update mid-request produces an explicit
reload path instead of mixing editions.

URLs carry semantic entry keys derived from type, name, and description, plus
optional book/chapter state. Numeric entity IDs are used only to query against
the matching release. If an entry's identity changes upstream, old links report
the missing selection rather than selecting a different entity.

No corpus payloads or application-owned chapter annotations are committed.
Additional relationship browsing, geographic maps, and chronology can build on
this boundary later; none are required to use the chapter Atlas.

## Verification

The regular Nix test suite includes dynamic taxonomy, namesakes, semantic keys,
cross-chapter and cross-book ranges, disjoint evidence, cache reuse, pagination,
release changes, failures, direct links, overlap indicators, and reader behavior.
`nix run .#atlas-smoke` exercises the real API separately from offline checks.
Desktop and mobile browser checks verify column geometry, actual corpus
highlights, refresh recovery, the filter drawer, and the chapter reader.
