import Config

config :scripguessr,
  corpus_url: System.get_env("CORPUS_URL", "https://api.standardworkscorpus.org/v1/sql")

if config_env() == :prod do
  secret = System.fetch_env!("SECRET_KEY_BASE")
  host = System.fetch_env!("PHX_HOST")

  config :scripguessr, Scripguessr.Repo,
    database: System.get_env("PGDATABASE", "scripguessr"),
    username: System.get_env("PGUSER", "scripguessr"),
    socket_dir: System.get_env("PGHOST", "/run/postgresql"),
    port: String.to_integer(System.get_env("PGPORT", "5432")),
    pool_size: 10

  config :scripguessr, ScripguessrWeb.Endpoint,
    server: System.get_env("PHX_SERVER") == "true",
    url: [host: host, scheme: "https", port: 443],
    http: [ip: {127, 0, 0, 1}, port: String.to_integer(System.get_env("PORT", "4000"))],
    secret_key_base: secret,
    check_origin: ["https://#{host}"]
end
