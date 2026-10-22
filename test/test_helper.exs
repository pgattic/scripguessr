ExUnit.start()
Ecto.Adapters.SQL.Sandbox.mode(Scripguessr.Repo, :manual)
{:ok, _} = Scripguessr.Corpus.Fake.start_link()
