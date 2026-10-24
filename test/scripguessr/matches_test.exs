defmodule Scripguessr.MatchesTest do
  use Scripguessr.DataCase
  import Ecto.Query
  import Scripguessr.AccountFixtures
  import Scripguessr.MatchFixtures
  alias Scripguessr.{Matches, Repo, Progression}
  alias Scripguessr.Matches.{Match, Player, Guess}

  test "private rooms require accounts, readiness, and the host to start" do
    {code, [{host, _}, {friend, _}]} = match_lobby()
    {stranger, _} = account("stranger")
    assert {:error, :account_required} = Matches.create(nil)
    assert {:error, :account_required} = Matches.join(code, nil)
    assert {:error, :not_found} = Matches.fetch("invalid", host.id)
    assert {:error, :not_ready} = Matches.start(code, host.id)
    assert {:error, :not_a_player} = Matches.start(code, stranger.id)
    assert {:ok, ^code} = Matches.join(code, friend.id)
    assert {:ok, ^code} = Matches.join(code, friend.id)
    assert {:error, :full} = Matches.join(code, stranger.id)
    assert {:error, :host_only} = Matches.start(code, friend.id)
    Matches.ready(code, host.id, true)
    assert {:error, :not_ready} = Matches.start(code, host.id)
    Matches.ready(code, friend.id, true)
    assert {:ok, ^code} = Matches.start(code, host.id)
    assert {:error, :already_started} = Matches.ready(code, friend.id, false)
    assert {:error, :already_started} = Matches.leave(code, friend.id)

    assert {:ok, %{status: "round", joined?: true, deadline: %DateTime{}}} =
             Matches.fetch(code, friend.id)

    assert Repo.aggregate(Player, :count) == 2
  end

  test "concurrent joins cannot overfill a room and a leaving friend frees their seat" do
    {code, [{host, _}, {friend, _}]} = match_lobby()
    {other, _} = account("other")

    results =
      [friend.id, other.id]
      |> Task.async_stream(&Matches.join(code, &1), max_concurrency: 2)
      |> Enum.map(fn {:ok, result} -> result end)

    assert Enum.count(results, &match?({:ok, _}, &1)) == 1
    assert {:error, :full} in results
    assert Repo.aggregate(Player, :count) == 2
    player = Repo.one!(from p in Player, where: p.user_id != ^host.id)
    assert {:ok, _} = Matches.leave(code, player.user_id)
    assert {:ok, _} = Matches.join(code, friend.id)
    assert Repo.aggregate(Player, :count) == 2
    assert {:ok, _} = Matches.leave(code, host.id)
    assert {:ok, %{status: "cancelled"}} = Matches.fetch(code, friend.id)
    assert {:error, :already_started} = Matches.join(code, other.id)
  end

  test "open rounds hide answers, future prompts, opponent guesses, and open-round points" do
    {code, [{host, _}, {friend, _}], snapshot} = started_match()
    {stranger, _} = account("stranger")
    first = hd(snapshot["rounds"])
    assert {:ok, view} = Matches.fetch(code, host.id)
    assert view.verse == first["text"]
    refute Map.has_key?(view, :snapshot)
    refute view.result
    assert view.results == []
    assert {:error, :not_a_player} = Matches.submit(code, stranger.id, 0, "Omni", "1")
    assert {:error, :stale_round} = Matches.submit(code, host.id, 1, "Omni", "1")
    assert {:error, :invalid_guess} = Matches.submit(code, host.id, 0, "bogus", "1")
    assert {:ok, _} = Matches.submit(code, host.id, 0, first["book"], to_string(first["chapter"]))
    assert {:ok, pending} = Matches.fetch(code, host.id)
    assert pending.own_guess == %{book: first["book"], chapter: first["chapter"]}
    assert Enum.all?(pending.players, &(&1.score == 0))
    refute pending.result
    assert {:ok, opponent} = Matches.fetch(code, friend.id)
    refute opponent.own_guess
    refute opponent.result
    assert opponent.results == []
    assert {:ok, private} = Matches.fetch(code, stranger.id)
    refute private.verse
    assert private.chapters == []
    refute private.result
    assert private.results == []
    assert Enum.all?(private.players, &is_nil(&1.score))
  end

  test "the first accepted guess is immutable across retries and competing tabs" do
    {code, [{host, _}, _], snapshot} = started_match()
    round = hd(snapshot["rounds"])
    assert {:ok, _} = Matches.submit(code, host.id, 0, round["book"], to_string(round["chapter"]))
    assert {:ok, _} = Matches.submit(code, host.id, 0, "Omni", "1")
    assert [%Guess{book: book, chapter: chapter, points: 1000}] = Repo.all(Guess)
    assert book == round["book"]
    assert chapter == round["chapter"]
  end

  test "both submissions reveal together and only the host can start the next round" do
    {code, [{host, _}, {friend, _}], snapshot} = started_match()
    round = hd(snapshot["rounds"])

    for user <- [host, friend],
        do: Matches.submit(code, user.id, 0, round["book"], to_string(round["chapter"]))

    assert {:ok, %{status: "reveal", round: 0, result: result, players: players}} =
             Matches.fetch(code, host.id)

    assert result.book == round["book"]
    assert Enum.all?(result.guesses, &(&1.points == 1000))
    assert Enum.all?(players, &(&1.score == 1000))
    assert {:error, :stale_round} = Matches.submit(code, friend.id, 0, "Omni", "1")
    Matches.tick()
    assert {:ok, %{status: "reveal", round: 0, deadline: nil}} = Matches.fetch(code, host.id)
    assert {:error, :host_only} = Matches.advance(code, friend.id, 0)
    assert {:error, :stale_round} = Matches.advance(code, host.id, 1)
    assert {:ok, _} = Matches.advance(code, host.id, 0)
    assert {:error, :stale_round} = Matches.advance(code, host.id, 0)

    assert {:ok, %{status: "round", round: 1, verse: text, result: nil}} =
             Matches.fetch(code, friend.id)

    assert text == Enum.at(snapshot["rounds"], 1)["text"]
  end

  test "timeouts score missing answers as zero and reject late guesses even before the clock polls" do
    {code, [{host, _}, {friend, _}], snapshot} = started_match()
    first = hd(snapshot["rounds"])
    Matches.submit(code, host.id, 0, first["book"], to_string(first["chapter"]))
    match = Repo.get_by!(Match, code: code)
    Repo.update!(Ecto.Changeset.change(match, deadline: DateTime.add(DateTime.utc_now(), -1)))

    assert {:error, :stale_round} =
             Matches.submit(code, friend.id, 0, first["book"], to_string(first["chapter"]))

    assert {:ok, %{status: "reveal", result: result}} = Matches.fetch(code, friend.id)
    assert [%{points: 1000}, %{points: 0, book: nil}] = result.guesses
    assert Repo.aggregate(Guess, :count) == 1
  end

  test "reconnect closes an expired round and waits for the host instead of skipping ahead" do
    {code, [{host, _}, _], _} = started_match()
    match = Repo.get_by!(Match, code: code)
    Repo.update!(Ecto.Changeset.change(match, deadline: DateTime.add(DateTime.utc_now(), -1000)))

    assert {:ok, %{status: "reveal", round: 0, deadline: nil, results: [], players: players}} =
             Matches.fetch(code, host.id)

    assert Enum.all?(players, &(&1.score == 0))
    Matches.tick()
    assert Repo.get_by!(Match, code: code).status == "reveal"
    assert {:ok, _} = Matches.advance(code, host.id, 0)
    assert {:ok, %{status: "round", round: 1, deadline: deadline}} = Matches.fetch(code, host.id)
    assert DateTime.diff(deadline, DateTime.utc_now()) in 80..90
    assert Progression.for_user(host.id).xp == 0
  end

  test "all five rounds finish with persistent totals, and outsiders cannot view results" do
    {code, accounts, snapshot} = started_match()
    {host, _} = hd(accounts)

    for {round, index} <- Enum.with_index(snapshot["rounds"]) do
      for {user, _} <- accounts,
          do: Matches.submit(code, user.id, index, round["book"], to_string(round["chapter"]))

      assert {:ok, %{status: "reveal", round: ^index, deadline: nil, results: []}} =
               Matches.fetch(code, host.id)

      if index < 4, do: Matches.advance(code, host.id, index)
    end

    assert {:ok, %{status: "reveal", round: 4, result: %{round: 4}}} =
             Matches.fetch(code, host.id)

    Matches.tick()
    assert Repo.get_by!(Match, code: code).status == "reveal"
    assert {:ok, _} = Matches.advance(code, host.id, 4)

    assert {:ok, %{status: "finished", results: results, players: players}} =
             Matches.fetch(code, host.id)

    assert length(results) == 5
    assert Enum.all?(players, &(&1.score == 5000))
    assert Repo.aggregate(Guess, :count) == 10
    assert {:error, :stale_round} = Matches.submit(code, host.id, 4, "Omni", "1")
    assert {:ok, %{results: [], result: nil, verse: nil}} = Matches.fetch(code, nil)
    assert [%{code: ^code, status: "finished"}] = Matches.recent(host.id)
    assert Progression.for_user(host.id).xp == 0
  end

  test "competing host tabs cannot advance twice or skip a round" do
    {code, [{host, _}, _], _} = started_match()
    assert {:error, :stale_round} = Matches.advance(code, host.id, 0)
    expire_match(code)

    results =
      1..2
      |> Task.async_stream(fn _ -> Matches.advance(code, host.id, 0) end)
      |> Enum.map(fn {:ok, result} -> result end)

    assert Enum.count(results, &match?({:ok, _}, &1)) == 1
    assert {:error, :stale_round} in results
    assert {:ok, %{status: "round", round: 1}} = Matches.fetch(code, host.id)
  end

  test "capacity and readiness work for three players without changing round logic" do
    {code, accounts, snapshot} = started_match(3)
    [{host, _}, {second, _}, {third, _}] = accounts
    round = hd(snapshot["rounds"])

    for user <- [host, second],
        do: Matches.submit(code, user.id, 0, round["book"], to_string(round["chapter"]))

    assert {:ok, %{status: "round", result: nil}} = Matches.fetch(code, host.id)
    Matches.submit(code, third.id, 0, round["book"], to_string(round["chapter"]))
    assert {:ok, %{status: "reveal", result: %{guesses: guesses}}} = Matches.fetch(code, host.id)
    assert length(guesses) == 3
    assert Enum.all?(guesses, &(&1.points == 1000))
  end

  test "failed preparation creates no room or memberships" do
    {user, _} = account("reader")
    Scripguessr.Corpus.Fake.reset(:down)
    assert {:error, _} = Matches.create(user.id)
    assert Repo.aggregate(Match, :count) == 0
    assert Repo.aggregate(Player, :count) == 0
  end
end
