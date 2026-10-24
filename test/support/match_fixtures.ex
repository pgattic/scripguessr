defmodule Scripguessr.MatchFixtures do
  import Scripguessr.AccountFixtures
  alias Scripguessr.{Matches, Repo}
  alias Scripguessr.Matches.Match

  def match_lobby(capacity \\ 2) do
    accounts = Enum.map(1..capacity, &account("player#{&1}"))
    {host, _} = hd(accounts)
    {:ok, code} = Matches.create(host.id, capacity)
    {code, accounts}
  end

  def started_match(capacity \\ 2) do
    {code, accounts} = match_lobby(capacity)

    for {user, _} <- accounts do
      {:ok, _} = Matches.join(code, user.id)
      {:ok, _} = Matches.ready(code, user.id, true)
    end

    {host, _} = hd(accounts)
    {:ok, _} = Matches.start(code, host.id)
    {code, accounts, Repo.get_by!(Match, code: code).snapshot}
  end

  def expire_match(code, seconds \\ 1) do
    Repo.get_by!(Match, code: code)
    |> Ecto.Changeset.change(deadline: DateTime.add(DateTime.utc_now(), -seconds))
    |> Repo.update!()

    Matches.tick()
  end
end
