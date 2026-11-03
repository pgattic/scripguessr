Application.ensure_all_started(:exqlite)
Application.put_env(:scripguessr, :corpus_client, Scripguessr.Corpus.SQLite)
{:ok, _} = Scripguessr.Corpus.SQLite.start_link()
{:ok, _} = Task.Supervisor.start_link(name: Scripguessr.Corpus.Tasks)
{:ok, _} = Supervisor.start_link([Scripguessr.Corpus.Chapter], strategy: :one_for_one)

Code.require_file("smoke.exs", __DIR__)
Code.require_file("atlas_smoke.exs", __DIR__)
