# ScripGuessr

A quiet scripture game built with Phoenix LiveView. Read five randomly selected
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
- `Scripguessr.Corpus.Cache` shares verse-pool and book/chapter metadata for 15
  minutes while preparing each game's selection independently. It retries once
  if a release changes mid-fetch.
- `Scripguessr.Corpus.RequestCache` runs fetches in supervised tasks with four
  workers per cache, at most 32 outstanding keys and 64 callers per key, and a
  60-second deadline including queue time. Identical requests share a fetch;
  cached hits bypass waiting work. Failures are not cached.
- `Scripguessr.Corpus.Chapter` shares release-checked chapter text between games,
  matches, and the Atlas. All use `ScripguessrWeb.ChapterReader` for the reader dialog.
- `ScripguessrWeb.GameComponents` shares the verse panel, guess picker, and round
  score reveal between single-player games and matches.
- `Scripguessr.Games.Rules` implements scoring and progression without I/O.
- `Scripguessr.Games` stores immutable prepared prompts and accepted guesses in
  PostgreSQL, using row locks to protect progression across tabs.
- `Scripguessr.Challenges` freezes each daily challenge independently of its
  players' attempts. The first successful preparation saves that day's verses;
  concurrent starts use the same database-enforced challenge and account attempt.
- `ScripguessrWeb.GameLive` renders the game and final summary.
- `Scripguessr.Progression` stores one XP reward per completed account game and
  derives lifetime XP and level progress from those rewards.
- `Scripguessr.Accounts` manages username/password accounts, private recovery
  codes, and expiring, revocable sessions.

Only numbered, nonempty verses from the Book of Mormon (2013) edition are eligible.
Five distinct references are sampled locally and their text fetched in one batch.
A prepared game includes the corpus release and chapter order needed to score
without another upstream request. Text is reconstructed from ordered corpus
words; no editorial changes are made. A guessed chapter earns
`max(0, 1000 - 22 * chapter_distance)` points, up to 5,000 per game.

After submitting a guess, use **Read chapter** to read the answer in context with
the selected verse highlighted. Chapters are also available from the final results.
Reading fetches the chapter on demand and checks the game's saved corpus release;
upstream failures do not affect scoring or progression.

Games belong to an anonymous browser identity in a signed, HttpOnly, SameSite
cookie, valid for 30 days and Secure over production HTTPS. Keep the browser
cookie and game URL to resume anonymously. Clearing the cookie loses anonymous
access; the URL alone does not authorize access. Game rows currently remain in
PostgreSQL until explicitly removed by an operator.

## Accounts

Create an account with a username and password; no email service is required.
Usernames are case insensitive. Save the private recovery code shown after
registration in your password manager. If you forget your password, **Forgot
password?** accepts that code and lets you choose a new password. Recovery
replaces the code and signs out existing sessions.

The account page lists saved games, with **Resume** for unfinished games and
**Review** for completed games. New games started while signed in are saved
automatically. **Add to my account** explicitly attaches this browser's earlier
anonymous games. Attached games require that account even on the original
browser after signing out. Anonymous play remains available.

Sessions last 30 days. Development and production use the same account flow,
without an email provider or development mailbox. Deployment requires only the
application secret described in [DEPLOYING.md](DEPLOYING.md). Authentication
behavior and limits are described in [ACCOUNTS.md](ACCOUNTS.md).

Completed games earn **100 XP plus an accuracy bonus of up to 100 XP** (one XP
per 50 game points). Account and results pages show your level and progress to
the next one. Level 2 starts at 500 XP, level 3 at 1,500 XP, and level 4 at
3,000 XP; each level takes 500 more XP than the previous one. Completed anonymous
games earn their XP when attached
to an account. Refreshing, resuming, or attaching again never awards extra XP.

The public **Leaderboard** shows the top 50 accounts by lifetime XP, alongside
their levels. Accounts appear after earning XP. Equal XP shares a rank (1, 1, 3),
with tied players displayed alphabetically. Only usernames, XP, and levels are
public; games and saved verses remain private. `Scripguessr.Leaderboards` owns
the ranking queries independently of the page, allowing future categories to
use different scoring rules.

## Daily game

Choose **Daily game** from Play for five shared Book of Mormon verses. Daily
games require an account and allow one attempt per account per UTC date. You can
resume that attempt on another device. A day's challenge is prepared on its first
start and frozen with its corpus release, so subsequent players get the same
verses and order even if the corpus changes or becomes unavailable.

Finish all five rounds before **midnight UTC** to enter that day's score board.
Unfinished attempts close at the reset and aren't ranked. Completed results stay
available in your game history. **Leaderboard → Daily game** shows the top 50
completed scores for a selected date, with equal scores sharing a rank. Signed-in
players outside the top 50 can still see their own rank. Daily games earn the
usual completion and accuracy XP once; they cannot be restarted for extra XP.

The results' **Share result** disclosure offers a copyable score and five colored
squares: green for an exact chapter, yellow for 750–999 points, orange for
250–749, and white for fewer than 250. The shared text links to `/daily` and
contains no verse text, answer references, or private game URL. Rankings publish
only usernames and scores. Random games remain separate and available without
an account; the Atlas is independent of both modes.

## Friendly matches

From Play, choose **Play with a friend**, create a room, and copy its invite link.
Both players need accounts. The friend joins the room, both mark themselves ready,
and the host starts. An invite opened while signed out returns to the room after
sign-in or registration; new accounts still receive their recovery code first.

Each player gets the same five frozen verses in the same order. Rounds last
**90 seconds**, or close early once everyone has submitted. Guesses lock on their
first accepted submission. The answer, both guesses, round points, and updated
totals appear together. The host chooses **Next round** when everyone has had time
to review the answer. The fifth round also has its own reveal; the host then
chooses **View results** to open the final summary for everyone.
Missing answers score zero. After five rounds, the highest total wins; equal
totals tie. During each shared reveal, players can read the chapter in the same
dialog used in regular games and save verses. Open readers close when the host
advances. Chapters remain available from the completed match summary; unrevealed
chapters cannot be opened before the round ends.
Friendly matches currently award no XP and do not affect daily rankings.

Reopen the invite link or use **Your rooms** on `/matches` to reconnect or review
results. Active round timers continue through disconnects; reveals wait for the
host to return and continue. Online indicators reflect open tabs,
and another tab or device can resume the same account's locked attempt. Before
starting, a friend can leave to free a seat, and the host can cancel the room.
Rooms are private invitations, with no public matchmaking or spectators.

Matches have their own `matches`, `match_players`, and `match_guesses` tables;
they have no daily-challenge or single-player game foreign keys. They share
corpus preparation, pure scoring rules, and presentation components. Player rows
and capacity-based rules support larger rooms; the first UI creates two-player rooms. Every mutation locks
the match row, and player-facing projections exclude future prompts, unrevealed
answers, and opponents' pending guesses. Scores update only when rounds close.

A supervised clock polls persisted round deadlines once per second. An expired
round closes into its reveal, so reconnecting or restarting cannot grant extra
guessing time or skip over results. Reveals have no deadline; only the host can
advance them. Reconnects and submissions also settle elapsed round deadlines.
Phoenix PubSub updates connected players and Presence supplies online indicators.
The displayed countdown runs in the browser from a server-provided deadline and
clock reading; open pages do not poll sessions or re-render every second.
Session validity is still checked on player actions and before pushed match
content. The account menu keeps its open state across LiveView updates.
The current deployment uses one Phoenix instance; multiple application nodes
would also need a configured distributed PubSub cluster.

The standalone [Atlas](http://localhost:4000/atlas) is available from the main
navigation. Search for topics, people, places, and other corpus entries, select
up to eight to compare their chapters, and click a chapter to read it. Descriptions
distinguish people and places with the same name. Colored bands mark overlaps;
the chapter reader explains each match as a reference, speech, or scene setting.
The Atlas has no game or study-set actions.

Categories follow the corpus entity-type hierarchy. Whole-book columns and
five-column chapter grids retain the original Atlas geometry. The current
corpus edition includes three front-matter sections, so its map has 18 books
and 242 chapter units. No chapter counts, category lists, or annotations are
vendored. Selection and reader URLs are shareable and restore on refresh.

`Scripguessr.Corpus.Atlas` owns the queries, `Scripguessr.Atlas.Model` expands
passage ranges across chapters, and `Scripguessr.Atlas.Cache` independently
caches catalog and evidence for 15 minutes with a 160-entry limit. A separate
160-entry chapter cache serves both readers. Chapter text loads independently of
highlights and stays visible when filters change. Detail cache keys include the
release, and mixed-release results require reloading.
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

Signed-in readers can save verses from revealed game answers or chapter readers.
**Saved verses**, available from the account menu and the account-page tabs,
lists them newest first in pages of 20, with links to read each chapter in the Atlas. Saves are private and retain their corpus text and release
so they remain readable during API outages.

The main navigation offers **Play**, **Atlas**, and **Leaderboard**. The account tag shows the
username, level, and lifetime XP, and opens account, saved-verse, and sign-out
actions. The landing page offers the most recently active unfinished game that
the current browser or account can access. **Play again** starts directly from
the results page. Bar fills and brief surface transitions respect the browser’s
reduced-motion preference. The header and favicon use the same SVG asset.
