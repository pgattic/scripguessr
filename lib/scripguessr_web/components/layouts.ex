defmodule ScripguessrWeb.Layouts do
  use ScripguessrWeb, :html

  def root(assigns) do
    ~H"""
    <!DOCTYPE html>
    <html lang="en">
      <head>
        <meta charset="utf-8" />
        <meta name="viewport" content="width=device-width, initial-scale=1" />
        <meta name="csrf-token" content={get_csrf_token()} />
        <meta name="theme-color" content="#294b3d" />
        <title>ScripGuessr</title>
        <link rel="icon" href="/favicon.svg" type="image/svg+xml" />
        <link rel="stylesheet" href="/assets/app.css" />
        <script defer src="/assets/app.js">
        </script>
      </head>
      <body>{@inner_content}</body>
    </html>
    """
  end

  def app(assigns) do
    ~H"""
    <div class="site-shell">
      <a href="#main" class="skip-link">Skip to content</a>
      <header class="site-header">
        <a href="/" class="wordmark" aria-label="ScripGuessr home">
          <img class="brand-mark" src="/favicon.svg" width="38" height="38" alt="" />
          <span>ScripGuessr</span>
        </a>
        <nav class="site-nav" aria-label="Main navigation">
          <a href="/" class="home-link" aria-current={if @active_section == :play, do: "page"}>Play</a>
          <a href="/atlas" class="home-link" aria-current={if @active_section == :atlas, do: "page"}>Atlas</a>
        </nav>
        <details :if={@current_user} id="account-menu" class="account-menu">
          <summary
            id="user-tag"
            class="user-tag"
            aria-label={"Account menu for #{@current_user.username}, level #{@current_progress.level}, #{@current_progress.xp} XP"}
          >
            <.avatar />
            <span class="user-tag-copy">
              <strong title={@current_user.username}>{@current_user.username}</strong>
              <span class="user-tag-stats">Level {@current_progress.level}<span aria-hidden="true"> · </span>{@current_progress.xp} XP</span>
              <span
                class="user-tag-meter"
                data-meter={@current_progress.percent}
                data-meter-stage={@current_progress.level}
                aria-hidden="true"
              >
                <span data-meter-fill style={"width: #{@current_progress.percent}%"}></span>
              </span>
            </span>
            <svg
              class="user-tag-chevron"
              viewBox="0 0 16 16"
              width="14"
              height="14"
              aria-hidden="true"
            ><path
              d="m4 6 4 4 4-4"
              fill="none"
              stroke="currentColor"
              stroke-width="1.5"
              stroke-linecap="round"
              stroke-linejoin="round"
            /></svg>
          </summary>
          <div class="account-popover">
            <nav aria-label="Account">
              <a href="/account" aria-current={if @active_section == :account, do: "page"}>Your account
              <span aria-hidden="true">→</span></a>
              <a href="/saved-verses" aria-current={if @active_section == :saved, do: "page"}>Saved verses
              <span aria-hidden="true">→</span></a>
            </nav>
            <.form for={%{}} action="/account/sign-out" id="header-sign-out">
              <button type="submit">Sign out</button>
            </.form>
          </div>
        </details>
        <a :if={!@current_user} href="/account/sign-in" class="header-sign-in">Sign in
        <span aria-hidden="true">↗</span></a>
      </header>
      <div
        id="connection-status"
        role="status"
        class="connection-status phx-client-error:hidden phx-server-error:hidden"
      >
        Reconnecting…
      </div>
      <main id="main" tabindex="-1">
        <p :if={Phoenix.Flash.get(@flash, :info)} class="account-flash panel" role="status">
          {Phoenix.Flash.get(@flash, :info)}
        </p>
        {@inner_content}
      </main>
      <footer>
        <span>
          Text from <a href="https://standardworkscorpus.org">Standard Works Corpus</a>
          ·
          <a href="https://github.com/mikaelweiss/standard-works-corpus/blob/main/CREDITS.md">Credits</a>
          · <a href="https://creativecommons.org/licenses/by/4.0/">CC BY 4.0</a>
        </span>
      </footer>
    </div>
    """
  end

  def avatar(assigns) do
    ~H"""
    <span class="user-avatar" aria-hidden="true">
      <svg
        viewBox="0 0 24 24"
        width="22"
        height="22"
        fill="none"
        stroke="currentColor"
        stroke-width="1.6"
        stroke-linecap="round"
      >
        <circle cx="12" cy="8" r="3.5" /><path d="M5 21v-2a7 7 0 0 1 14 0v2" />
      </svg>
    </span>
    """
  end

  attr :active, :atom, required: true

  def personal_nav(assigns) do
    ~H"""
    <nav class="personal-nav" aria-label="Your library">
      <a href="/account" aria-current={if @active == :account, do: "page"}>Overview &amp; games</a>
      <a href="/saved-verses" aria-current={if @active == :saved, do: "page"}>Saved verses</a>
    </nav>
    """
  end
end
