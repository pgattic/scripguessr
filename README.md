# ScripGuessr

A quiet scripture quiz built with Phoenix LiveView. Read five randomly selected
Book of Mormon verses, guess their book and chapter, and discover how close you
came. Scripture content comes from [Standard Works Corpus](https://standardworkscorpus.org).

The rewrite's goals and first milestone are in [SCOPE.md](SCOPE.md).

## Develop with Nix

With flakes enabled, run from this repository:

```sh
nix run .#dev
```

Open <http://localhost:4000>. This command supplies all tools, fetches locked Mix
dependencies, starts a local PostgreSQL instance, applies migrations, builds the
browser assets, and starts Phoenix. Database files and tool caches live in
`.nix/`, which is ignored by Git. Stopping the command stops the database if the
command started it. Existing data is retained.

`nix develop` opens a shell with Elixir 1.19, Erlang, Hex, Rebar, PostgreSQL,
esbuild, and formatting/build tools. There is no separate Node/npm installation
or frontend dependency set. The Phoenix clients ship with the locked Hex packages.

Elixir modules reload during development. After changing JavaScript, run
`mix assets.build` in another `nix develop` shell; refresh after CSS changes.
Use `PORT` to change the HTTP port. `PGPORT`, `PGHOST`, and `SCRIPGUESSR_PGDATA`
can change the local PostgreSQL port, Unix socket directory, and data directory.
The database listens only on its Unix socket. If a checkout path is too long for
a Unix socket, set `PGHOST` to a short, private directory.

The flake declares x86_64 Linux, aarch64 Linux, and Apple Silicon macOS development
environments. Initial verification was performed on aarch64 Linux.

## Verify

```sh
nix run .#test             # PostgreSQL-backed tests; no live corpus requests
nix fmt                   # Format Elixir, HEEx, and Nix after fetching Mix deps
nix flake check           # Sandboxed tests, formatting, and production build
nix build                 # Production release at ./result
nix run .#smoke            # Separately check the real corpus API; needs network
nix run .#atlas-smoke      # Check real Atlas categories, highlights, and reading
```

The development and test commands fetch dependencies automatically. For a fresh
checkout where you only want to format, first run `nix develop -c mix deps.get`.
CI uses the same flake checks and build. The real corpus smoke test is deliberately
outside CI's offline checks.

Nix pins nixpkgs through the nixexprs channel input and uses flake-parts. Mix
packages are fixed-output Nix dependencies. The test HTML parser's native source
is separately pinned so checks do not download native binaries during compilation.
After changing `mix.lock`, update the dependency hash in `nix/package.nix`:
replace it with `pkgs.lib.fakeHash`, run `nix build .#deps`, and use the reported
hash. Do not commit build outputs or corpus payloads.

## Architecture

- `Scripguessr.Corpus` owns upstream SQL and converts responses to application data.
- `Scripguessr.Corpus.Cache` serializes preparation and caches the verse pool and
  chapter metadata for 15 minutes. It retries once if a release changes mid-fetch.
- `Scripguessr.Quizzes.Rules` implements scoring and progression without I/O.
- `Scripguessr.Quizzes` stores immutable prepared prompts and accepted guesses in
  PostgreSQL, using row locks to protect progression across tabs.
- `ScripguessrWeb.QuizLive` renders the quiz and final summary.
- `Scripguessr.Accounts` manages username/password accounts, private recovery
  codes, and expiring, revocable sessions.

Only numbered, nonempty verses from the Book of Mormon (2013) edition are eligible.
Five distinct references are sampled locally and their text fetched in one batch.
A prepared quiz includes the corpus release and chapter order needed to score
without another upstream request. Text is reconstructed from ordered corpus
words; no editorial changes are made. A guessed chapter earns
`max(0, 1000 - 22 * chapter_distance)` points, up to 5,000 per quiz.

After submitting a guess, use **Read chapter** to read the answer in context with
the quizzed verse highlighted. Chapters are also available from the final results.
Reading fetches the chapter on demand and checks the quiz's saved corpus release;
upstream failures do not affect scoring or progression.

Quizzes belong to an anonymous browser identity in a signed, HttpOnly, SameSite
cookie, valid for 30 days and Secure over production HTTPS. Keep the browser
cookie and quiz URL to resume anonymously. Clearing the cookie loses anonymous
access; the URL alone does not authorize access. Quiz rows currently remain in
PostgreSQL until explicitly removed by an operator.

## Accounts

Create an account with a username and password; no email service is required.
Usernames are case insensitive. Save the private recovery code shown after
registration in your password manager. If you forget your password, **Forgot
password?** accepts that code and lets you choose a new password. Recovery
replaces the code and signs out existing sessions.

The account page lists saved quizzes, with **Resume** for unfinished games and
**Review** for completed games. New quizzes started while signed in are saved
automatically. **Add to my account** explicitly attaches this browser's earlier
anonymous quizzes. Attached quizzes require that account even on the original
browser after signing out. Anonymous play remains available.

Sessions last 30 days. Development and production use the same account flow,
without an email provider or development mailbox. Deployment requires only the
application secret described in [DEPLOYING.md](DEPLOYING.md). Authentication
behavior and limits are described in [ACCOUNTS.md](ACCOUNTS.md).

The standalone [Atlas](http://localhost:4000/atlas) is available from the main
navigation. Search for topics, people, places, and other corpus entries, select
up to eight to compare their chapters, and click a chapter to read it. Descriptions
distinguish people and places with the same name. Colored bands mark overlaps;
the chapter reader explains each match as a reference, speech, or scene setting.
The Atlas has no quiz or study-set actions.

Categories follow the corpus entity-type hierarchy. Whole-book columns and
five-column chapter grids retain the original Atlas geometry. The current
corpus edition includes three front-matter sections, so its map has 18 books
and 242 chapter units. No chapter counts, category lists, or annotations are
vendored. Selection and reader URLs are shareable and restore on refresh.

`Scripguessr.Corpus.Atlas` owns the queries, `Scripguessr.Atlas.Model` expands
passage ranges across chapters, and `Scripguessr.Atlas.Cache` independently
caches catalog, evidence, and chapter text for 15 minutes with a 160-entry limit.
Cache keys include the release, and mixed-release results require reloading.
URL identities use entry type, name, and description rather than numeric IDs;
changed or removed entries are reported rather than silently matched elsewhere.
See [ATLAS.md](ATLAS.md) for the data investigation and design decisions.

## Deploy

See [DEPLOYING.md](DEPLOYING.md) for the NixOS module, runtime secrets, HTTPS,
health checks, and upgrade procedure.

## Scripture attribution

Scripture data is provided by [Standard Works Corpus](https://standardworkscorpus.org),
licensed under [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/).
Preserve its [source credits](https://github.com/mikaelweiss/standard-works-corpus/blob/main/CREDITS.md)
when redistributing data. Attribution and credits are linked in the application.
