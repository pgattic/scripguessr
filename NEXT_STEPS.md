# ScripGuessr Next Steps

## Near Term

- Keep the scoring model explicit and tested, including exact guesses, close guesses, far guesses, and cross-book adjacency.
- Add lightweight server-side game cleanup so abandoned in-memory games do not live forever.
- Consider persistent/shared stats once friends are using the hosted version.
- Extend URL navigation into the Atlas with shareable layer and chapter state. Setup, review, study sets, and the Atlas have stable routes with browser history and direct study-set links; games now use versioned URLs backed by resumable server snapshots.

## Done

- Added a default playable verse pool that skips very short, repetitive, or low-signal verses.
- Added a game setup flow for round count, canon, and difficulty.
- Added local stats for best score, average score, rounds played, and weakest books.
- Split the app into focused modules for UI, game state, scoring, scripture data, and stats.
- Added a NixOS service module and deployment notes for hosting behind a reverse proxy.
- Polished round review with exact references, guess/answer comparison, chapter distance, and a containing-chapter reader.
- Moved scripture JSON into lazy-loaded static assets so the initial client bundle does not include every canon.
- Added mixed-canon game modes for Bible, restoration scripture, and all standard works.
- Reworked game setup so presets set canon toggles, and skipped single-option chooser levels.
- Moved scripture metadata, round generation, guess scoring, and chapter-reader payloads to a real backend.
- Added built-in doctrinal mastery study sets and locally persisted custom passage lists.

## Current Priority

Harden the hosted backend for longer-lived friend testing.
