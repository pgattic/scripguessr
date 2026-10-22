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
        <title>ScripGuessr · A little time in the word</title>
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
      <header class="site-header">
        <a href="/" class="wordmark" aria-label="ScripGuessr home"><span class="brand-mark">S</span>
        ScripGuessr<span class="edition-label">A SCRIPTURE QUIZ</span></a>
        <span class="header-note">A little time in the word.</span>
      </header>
      <div
        id="connection-status"
        role="status"
        class="connection-status phx-client-error:hidden phx-server-error:hidden"
      >
        Connection interrupted. Reconnecting to your saved quiz…
      </div>
      <main id="main">{@inner_content}</main>
      <footer>
        <span>Read closely. Find your bearings.</span>
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
end
