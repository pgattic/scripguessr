defmodule Scripguessr.DataCase do
  use ExUnit.CaseTemplate

  setup do
    :ok = Ecto.Adapters.SQL.Sandbox.checkout(Scripguessr.Repo)
    Ecto.Adapters.SQL.Sandbox.mode(Scripguessr.Repo, {:shared, self()})
    Scripguessr.Corpus.Fake.reset()
    Scripguessr.Corpus.Cache.clear()
    Scripguessr.Accounts.RateLimit.clear()
    Scripguessr.Corpus.Chapter.clear()
    :ok
  end
end
