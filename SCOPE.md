# ScripGuessr rewrite scope

Status: initial implementation scope  
Date: 2026-10-21

## Purpose

Rebuild ScripGuessr in Elixir, Phoenix, and LiveView, using Standard Works Corpus
as the source of scripture content. Keep development, verification, packaging,
and deployment Nix-first.

The Rust implementation at `../scripguessr-rust` is a reference for behavior and
lessons learned. Its implementation, appearance, and feature set are not a
requirement for this milestone. This repository starts with fresh Git history;
do not copy the vendored scripture payloads into it.

## First milestone

Deploy a playable, anonymous session of five randomly selected Book of Mormon
verses, backed by the corpus API.

The player can:

1. Start a five-round game without creating an account.
2. Read one verse at a time without seeing its reference.
3. Guess its book and chapter.
4. Submit the guess and see the correct reference, score, and comparison.
5. Advance through all five rounds and see a final summary.
6. Refresh or return to the session and continue it.

The five verses must be distinct, numbered, and nonempty. Use the Book of Mormon
2013 edition. Initially, all verses satisfying those criteria are eligible;
more elaborate difficulty and prompt-quality filters are deferred.

## Product and presentation

Use Phoenix LiveView for the game. Preserve the useful rhythm of the existing
game: a focused prompt, clear reference selection, deliberate submission, and
an understandable answer reveal.

Explore a new visual direction during implementation. Matching the Rust app's
layout or styling is not required. Make the game usable on mobile and desktop,
with keyboard-accessible controls, visible focus, and clear loading and error
states. Keep purely visual interactions responsive without unnecessary server
round trips.

## Application boundaries

- **Corpus integration:** upstream queries, response decoding, reference
  normalization, metadata, and caching.
- **Game behavior:** verse selection, guessing, scoring, progression, and
  persisted sessions.
- **Presentation:** LiveViews and components that expose those behaviors.

Keep upstream SQL and corpus response formats inside the corpus integration.
Game rules should be testable without a network connection or a running
LiveView. Do not introduce a second frontend application or retain the Rust
backend.

## Corpus data

Use `https://api.standardworkscorpus.org/v1` as the upstream source. Verify the
live API contract during integration; do not assume the neighboring corpus
checkout always matches production.

- Cache the eligible reference pool and book/chapter metadata server-side.
  Select five distinct references locally and fetch their text in batches.
- Handle pagination, truncation, timeouts, rate limits, and upstream errors
  explicitly. Avoid an upstream request for every UI interaction.
- Store edition, book, chapter, and verse references rather than depending
  exclusively on upstream numeric IDs remaining stable.
- Record corpus release information with prepared sessions. Validate that
  metadata and text used to prepare a session belong to a consistent release;
  retry preparation if a release changes between requests.
- Persist the five prompts, answer references, and metadata needed for scoring.
  Once prepared, a game must be playable through completion without further
  corpus requests, including during an upstream outage.
- Keep corpus content out of source control and build inputs. Runtime caches
  and session snapshots are allowed; they are not a second maintained corpus.
- Include corpus attribution and preserve the applicable credits required by
  the data license.

If session preparation fails, show a useful retry state. Do not expose a partial
game as a successfully prepared session.

## Scoring and session persistence

Guess by book and chapter. Award 1,000 points for an exact chapter; otherwise
award `max(0, 1000 - 22 * chapter_distance)`. Measure chapter distance using
canonical Book of Mormon book/chapter order, including across book boundaries.
The maximum session score is 5,000 points. Freeze the relevant chapter ordering
for a prepared session so corpus updates do not change its scoring.

Use PostgreSQL for durable game state and an anonymous browser session to
authorize access. No account is required. A refresh or application restart
must preserve prepared verses, submitted guesses, scores, and current progress.
Repeated submissions must not change an accepted guess or count it twice.

Send only the current prompt and information the player is allowed to see.
Keep unrevealed answers server-side, and verify session ownership for reads and
mutations. Sharing a game URL must not grant access to someone else's session.

## Nix-first workflow

The canonical workflow uses a committed flake and lock file. Document exact
commands as each entry point is implemented.

- `nix develop`: supplies the Elixir/Erlang toolchain and every required asset,
  database, formatting, and verification tool. No separately installed Mix,
  Node, PostgreSQL, or global tooling is required.
- `nix run .#dev`: starts the development application and a project-local
  PostgreSQL instance, with documented initialization and cleanup behavior.
- `nix fmt`: formats application and Nix sources.
- `nix flake check`: runs formatting checks and meaningful automated tests,
  including database-backed checks where needed, without the live corpus API.
- `nix build`: produces a production release containing the runtime and built
  assets. The target host must not need a compiler or development toolchain.
- Provide a NixOS service module for the release, PostgreSQL provisioning,
  runtime configuration, and service lifecycle. Document HTTPS reverse proxy
  configuration, health checks, migrations, and upgrades.

Pin dependencies and make all build-time downloads explicit Nix dependencies;
production builds must not fetch corpus data or depend on mutable upstream
responses. CI uses the same Nix entry points as local development. Keep secrets
in runtime configuration outside the Nix store and repository.

Start with Linux deployment and document supported development platforms.
Docker and non-Nix deployment workflows are outside this milestone.

## Acceptance criteria

- A deployed HTTPS URL serves the complete five-round game.
- The running application obtains its verse pool and text from the corpus API,
  with no bundled scripture dataset.
- Five distinct eligible verses are prepared, and guessing and answer reveals
  work for every round.
- Exact, nearby, cross-book, and distant guesses produce the specified scores;
  the final summary agrees with the accepted round results.
- Refreshing and restarting the application preserve the same session.
- Prepared sessions finish while the corpus API is unavailable; failed new
  session preparation offers retry without creating a broken game.
- Another browser session cannot read or modify the game using its URL alone.
- The documented Nix development, checks, build, and NixOS deployment workflows
  work without manually installed language or database tools.
- Automated checks cover selection invariants, scoring, ownership, persistence,
  repeated submissions, and representative corpus failures using small fixtures
  or a fake upstream. Verify the real integration separately with a smoke test.

## Delivery sequence

1. Establish the flake, Phoenix skeleton, production release, and NixOS module;
   deploy the skeleton before investing in game features.
2. Implement the corpus interface and verify real reference/text queries,
   release handling, and caching.
3. Implement persisted session preparation and independently tested game rules.
4. Build the LiveView game, answer reveal, and final summary.
5. Verify the deployed experience, persistence, and upstream failure behavior;
   finish operational documentation.

## Deferred work

Accounts, leaderboards, custom study sets, review history, other scripture
works, adjustable round counts, and difficulty settings
are outside the first milestone. Capture discoveries without adding these
features to the release gate.

## Standalone Atlas milestone

Build the Atlas as a separate study tool with no game integration, highlighted
chapter practice, or study-set creation. Retain the original compact book flow:
whole books in balanced columns, five-column grids, and 33px chapter tiles.

Discover categories, entities, and passage membership from Standard Works Corpus.
Use its entity-type hierarchy and reference, speech, and scene-setting annotations
to highlight chapters. Provide readable entry descriptions, search, overlapping
selections, and a chapter reader that explains the evidence. Preserve selections
and open chapters in shareable URLs. Cache runtime data and handle upstream
failures and release changes without affecting the separate game feature.

Keep missing annotation coverage in the corpus, with no application-owned
category or chapter annotation lists. Research findings and design decisions
are documented in [ATLAS.md](ATLAS.md).

## Accounts milestone

The subsequent accounts milestone adds optional username/password registration
and sign-in, sign-out, offline recovery codes, paginated game history, cross-device
resumption, and explicit attachment of this browser's anonymous games. New games
belong to the signed-in account. Anonymous play remains available. Accounts require
no email delivery service. Legacy Rust data migration is not a release requirement;
see [ACCOUNTS.md](ACCOUNTS.md) for the implemented boundaries and access model.

## Lessons to retain

- Prefer a single documented Nix workflow over manually coordinated toolchains.
- Let scripture data evolve independently of application builds.
- Keep game rules separate from data access and UI state.
- Preserve concrete behavioral examples instead of porting code mechanically.
- Validate deployment early and keep the replacement's first release small.
