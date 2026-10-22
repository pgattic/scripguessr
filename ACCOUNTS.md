# Accounts

Accounts use a username and password. There is no mail server, email provider,
external identity provider, or separate development authentication flow.
Anonymous game play and the standalone Atlas remain available without signing in.

## Credentials and recovery

Usernames contain 3–32 ASCII letters, numbers, underscores, or hyphens. A separate
lowercase key enforces case-insensitive uniqueness while retaining display case.
Passwords contain 8–128 characters. Password fields are never repopulated in
responses or returned in authenticated user structs.

Passwords are salted and hashed using [pbkdf2_elixir](https://hexdocs.pm/pbkdf2_elixir/Pbkdf2.html)
with PBKDF2-HMAC-SHA256 and 600,000 iterations, matching the relevant
[OWASP work-factor guidance](https://cheatsheetseries.owasp.org/cheatsheets/Password_Storage_Cheat_Sheet.html).
Unknown users still perform password-hash work and receive the same sign-in error
as an incorrect password.

Registration generates a random 256-bit recovery code, shown once in the response.
Save it in a password manager. Only its SHA-256 digest is stored. The code does not
expire; it is an offline credential, not an emailed link. Recovery requires the
username, code, and a valid new password with confirmation. It atomically replaces
both password and recovery code, revokes all previous sessions, and issues a new
session. The old code cannot be reused. Recovery locks the account row to
serialize concurrent attempts; a sign-in that raced a password reset cannot
create a session using stale password verification.

If both password and recovery code are lost, there is no automated recovery.
Username changes, account deletion, and administrative recovery are outside this
milestone.

## Sessions and game ownership

Sessions are random 256-bit tokens stored as SHA-256 digests. They expire after
30 days and can be revoked independently. Sign-in renews the browser session and
CSRF token, retaining only anonymous browser identity so earlier games can be
attached deliberately. Sign-out revokes the current session and disconnects its
LiveViews; other devices stay signed in. Recovery disconnects every previous
session. Signed-in LiveViews also check session validity before processing events.

Signed-in games have a nullable account foreign key alongside the original
anonymous owner digest. Account-owned games require their account; the old
anonymous cookie grants no fallback access after sign-out. Anonymous games
remain available in their originating browser.

The explicit claim action atomically updates only games whose account is still
null and whose owner digest matches this browser. Request parameters cannot choose
another browser or destination account. History lists only the current account's
games, newest first, in pages of 20.

All account mutations use CSRF-protected POST requests. Pages use
`Cache-Control: no-store` and `Referrer-Policy: no-referrer`. Passwords, password
confirmations, recovery codes, and tokens are filtered from Phoenix parameter logs.

The single account migration creates only the current username/password schema.
Disposable development databases that applied earlier account migrations need to
be recreated to receive the consolidated schema.

## Limits and deployment

Registration, sign-in, and recovery share a bounded per-node limiter allowing ten
attempts per client IP per ten minutes. It resets on application restart and is
not distributed across instances. Production trusts `X-Real-IP` only from its
loopback proxy; the supplied NixOS nginx configuration overwrites that header.
Custom proxies must do the same, with the application bound to localhost.
Development ignores forwarded IP headers.

Production needs only `SECRET_KEY_BASE` in the runtime environment file alongside
the host and database settings supplied by the NixOS module. No account email
credentials are required.
