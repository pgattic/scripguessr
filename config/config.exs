import Config
config :scripguessr, ecto_repos: [Scripguessr.Repo], corpus_client: Scripguessr.Corpus.HTTP

config :scripguessr, Scripguessr.Repo,
  migration_primary_key: [type: :binary_id],
  template: "template0"

config :scripguessr, ScripguessrWeb.Endpoint,
  adapter: Bandit.PhoenixAdapter,
  url: [host: "localhost"],
  render_errors: [formats: [html: ScripguessrWeb.ErrorHTML], layout: false],
  pubsub_server: Scripguessr.PubSub,
  live_view: [signing_salt: "quiz-live-v1"]

config :phoenix, :json_library, Jason

config :logger, :default_formatter,
  format: "$time $metadata[$level] $message\n",
  metadata: [:request_id]

import_config "#{config_env()}.exs"
