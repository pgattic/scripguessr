# ScripGuessr Next Steps

## Near Term

- Move scripture selection and round generation to a real backend before opening up larger public play.
- Keep the scoring model explicit and tested, including exact guesses, close guesses, far guesses, and cross-book adjacency.

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

## Current Priority

Move scripture selection and round generation to a backend.
