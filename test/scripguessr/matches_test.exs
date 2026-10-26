defmodule Scripguessr.MatchesTest do
  use Scripguessr.DataCase
  import Ecto.Query
  import Scripguessr.AccountFixtures
  import Scripguessr.MatchFixtures
  alias Scripguessr.{Matches, Repo, Progression}
  alias Scripguessr.Matches.{Match, Player, Guess}

  test "private rooms require an identity, readiness, and the host to start" do
    {code, [{host, _}, {friend, _}]} = match_lobby()
    {stranger, _} = account("stranger")
    assert {:error, :identity_required} = Matches.create(nil)
    assert {:error, :identity_required} = Matches.join(code, nil)
    assert {:error, :not_found} = Matches.fetch("invalid", host.id)
    assert {:error, :not_ready} = Matches.start(code, host.id)
    assert {:error, :not_a_player} = Matches.start(code, stranger.id)
    assert {:ok, ^code} = Matches.join(code, friend.id)
    assert {:ok, ^code} = Matches.join(code, friend.id)
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
    {code, [{host, _}, {friend, _} | waiting]} = match_lobby(8)
    {other, _} = account("other")

    for {user, _} <- waiting, do: Matches.join(code, user.id)

    results =
      [friend.id, other.id]
      |> Task.async_stream(&Matches.join(code, &1), max_concurrency: 2)
      |> Enum.map(fn {:ok, result} -> result end)

    assert Enum.count(results, &match?({:ok, _}, &1)) == 1
    assert {:error, :full} in results
    assert Repo.aggregate(Player, :count) == 8
    player = Repo.one!(from p in Player, where: p.user_id in ^[friend.id, other.id])
    assert {:ok, _} = Matches.leave(code, player.user_id)
    assert {:ok, _} = Matches.join(code, friend.id)
    assert Repo.aggregate(Player, :count) == 8
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

  defp guest do
    owner = Base.url_encode64(:crypto.strong_rand_bytes(32), padding: false)
    {Scripguessr.Matches.Identity.new(nil, owner), owner}
  end

  test "three guests can host, reconnect, score, and finish without creating accounts" do
    {host, owner} = guest()
    {friend, _} = guest()
    {third, _} = guest()
    {stranger, _} = guest()
    assert {:ok, code} = Matches.create(host, "Avery")
    assert {:ok, _} = Matches.join(code, friend, "Blake")
    assert {:ok, _} = Matches.join(code, third, "Casey")
    assert {:ok, view} = Matches.fetch(code, host)
    assert view.player_id == view.host_id
    assert Enum.all?(view.players, & &1.guest?)
    assert Enum.all?(view.players, &is_nil(&1.user_id))
    refute Enum.any?(view.players, &Map.has_key?(&1, :guest_hash))
    refute inspect(view) =~ owner
    assert Repo.aggregate(Scripguessr.Accounts.User, :count) == 0
    for actor <- [host, friend, third], do: assert({:ok, _} = Matches.ready(code, actor, true))
    assert {:error, :host_only} = Matches.start(code, friend)
    assert {:ok, _} = Matches.start(code, host)
    snapshot = Repo.get_by!(Match, code: code).snapshot
    assert {:error, :not_a_player} = Matches.submit(code, stranger, 0, "Omni", "1")
    # A public participant ID is not an authentication credential.
    assert {:error, :not_a_player} = Matches.ready(code, view.host_id, false)

    reconnected = Scripguessr.Matches.Identity.new(nil, owner)
    assert {:ok, %{player_id: id}} = Matches.fetch(code, reconnected)
    assert id == view.host_id

    for {round, index} <- Enum.with_index(snapshot["rounds"]) do
      for actor <- [host, friend, third] do
        assert {:ok, _} =
                 Matches.submit(code, actor, index, round["book"], to_string(round["chapter"]))
      end

      assert {:error, :host_only} = Matches.advance(code, friend, index)
      assert {:ok, _} = Matches.advance(code, reconnected, index)
    end

    assert {:ok, %{status: "finished", players: players, results: results}} =
             Matches.fetch(code, reconnected)

    assert Enum.all?(players, &(&1.score == 5000))
    assert length(results) == 5
    assert {:ok, %{results: [], result: nil}} = Matches.fetch(code, stranger)
    assert [%{code: ^code}] = Matches.recent(reconnected)
    assert Matches.recent(stranger) == []
  end

  test "guest names are validated and a guest cannot take another participant's name" do
    {host, _} = guest()
    {friend, _} = guest()

    for name <- [
          nil,
          "",
          "   ",
          String.duplicate("x", 33),
          "x" <> String.duplicate("\u0301", 32),
          "Avery\nSmith",
          "Avery" <> <<0>>
        ] do
      assert {:error, :invalid_name} = Matches.create(host, name)
    end

    assert Repo.aggregate(Match, :count) == 0
    assert {:ok, code} = Matches.create(host, " Avery ")
    assert {:error, :name_taken} = Matches.join(code, friend, "avery")
    assert {:ok, _} = Matches.join(code, friend, "Blake")
    assert {:ok, _} = Matches.join(code, friend)
    assert Repo.aggregate(Player, :count) == 2
    assert {:ok, _} = Matches.leave(code, friend)
    assert {:ok, _} = Matches.join(code, friend, "New name")
    assert {:ok, _} = Matches.leave(code, host)
    assert {:ok, %{status: "cancelled"}} = Matches.fetch(code, friend)

    assert {:error, :identity_required} =
             Matches.create(Scripguessr.Matches.Identity.new(nil, "bad"), "Avery")
  end

  test "signing in preserves a browser's guest seat without merging it into an account" do
    {host, owner} = guest()
    {user, _} = account("friend")
    {other, _} = guest()
    assert {:ok, code} = Matches.create(host, "Avery")
    assert {:ok, _} = Matches.join(code, user.id)
    signed_in = Scripguessr.Matches.Identity.new(user.id, owner)
    # If the account already has its own seat, that seat wins over the browser's guest seat.
    assert {:ok, account_view} = Matches.fetch(code, signed_in)
    assert Enum.find(account_view.players, &(&1.id == account_view.player_id)).user_id == user.id
    assert {:ok, _} = Matches.leave(code, user.id)
    assert {:ok, guest_view} = Matches.fetch(code, signed_in)
    assert guest_view.player_id == guest_view.host_id
    assert {:ok, _} = Matches.ready(code, signed_in, true)
    assert {:ok, _} = Matches.join(code, other, "Blake")
    assert {:ok, _} = Matches.ready(code, other, true)
    assert {:ok, _} = Matches.start(code, signed_in)
    assert {:ok, %{joined?: false}} = Matches.fetch(code, user.id)
    assert [%{code: ^code}] = Matches.recent(signed_in)
    assert Progression.for_user(user.id).xp == 0
  end

  test "competing guest joins cannot duplicate a browser's seat" do
    {host, _} = guest()
    {friend, _} = guest()
    {:ok, code} = Matches.create(host, "Avery")

    results =
      Task.async_stream(1..2, fn _ -> Matches.join(code, friend, "Blake") end) |> Enum.to_list()

    assert Enum.all?(results, &match?({:ok, {:ok, _}}, &1))
    assert Repo.aggregate(Player, :count) == 2
  end

  test "a room limit does not require filling every seat but all present players must be ready" do
    {host, _} = guest()
    {friend, _} = guest()
    {third, _} = guest()

    {:ok, code} = Matches.create(host, "Avery")
    Matches.ready(code, host, true)
    assert {:error, :not_ready} = Matches.start(code, host)
    Matches.join(code, friend, "Blake")
    Matches.ready(code, friend, true)
    assert {:ok, %{all_ready?: true}} = Matches.fetch(code, host)
    Matches.join(code, third, "Casey")
    assert {:ok, %{all_ready?: false}} = Matches.fetch(code, host)
    assert {:error, :not_ready} = Matches.start(code, host)
    Matches.ready(code, third, true)
    assert {:ok, _} = Matches.start(code, host)
    assert {:ok, %{capacity: 8, players: players}} = Matches.fetch(code, host)
    assert length(players) == 3
  end

  test "only a lobby's host may remove its other players" do
    {host, _} = guest()
    {friend, _} = guest()
    {other, _} = guest()
    {:ok, code} = Matches.create(host, "Avery")
    {:ok, other_code} = Matches.create(other, "Casey")
    Matches.join(code, friend, "Blake")
    {:ok, room} = Matches.fetch(code, host)
    {:ok, other_room} = Matches.fetch(other_code, other)
    id = Enum.find(room.players, &(&1.id != room.host_id)).id
    assert {:error, :host_only} = Matches.remove(code, friend, id)
    assert {:error, :not_a_player} = Matches.remove(code, other, id)
    assert {:error, :cannot_remove_host} = Matches.remove(code, host, room.host_id)
    assert {:error, :player_not_found} = Matches.remove(code, host, other_room.host_id)
    assert {:ok, _} = Matches.remove(code, host, id)
    assert {:error, :not_a_player} = Matches.ready(code, friend, true)
    assert {:error, :not_a_player} = Matches.submit(code, friend, 0, "Omni", "1")
    assert {:error, :player_not_found} = Matches.remove(code, host, id)
    # Removal frees the seat; reopening the invite permits a fresh join.
    assert {:ok, _} = Matches.join(code, friend, "Blake")
    for actor <- [host, friend], do: Matches.ready(code, actor, true)
    Matches.start(code, host)
    assert {:error, :already_started} = Matches.remove(code, host, room.host_id)
  end

  test "join and start races keep the active roster consistent" do
    {host, _} = guest()
    {friend, _} = guest()
    {late, _} = guest()
    {:ok, code} = Matches.create(host, "Avery")
    Matches.join(code, friend, "Blake")
    for actor <- [host, friend], do: Matches.ready(code, actor, true)

    tasks = [
      Task.async(fn -> Matches.start(code, host) end),
      Task.async(fn -> Matches.join(code, late, "Casey") end)
    ]

    results = Enum.map(tasks, &Task.await/1)
    {:ok, room} = Matches.fetch(code, host)

    case room.status do
      "round" ->
        assert results == [{:ok, code}, {:error, :already_started}]
        assert length(room.players) == 2

      "lobby" ->
        assert results == [{:error, :not_ready}, {:ok, code}]
        assert length(room.players) == 3
    end
  end

  test "eight players can finish five rounds and capacity rejects a ninth" do
    actors = Enum.map(1..9, fn _ -> guest() |> elem(0) end)
    host = hd(actors)
    {:ok, code} = Matches.create(host, "Player 1")

    for {actor, index} <- actors |> Enum.take(8) |> Enum.with_index(1) do
      assert {:ok, _} = Matches.join(code, actor, "Player #{index}")
      assert {:ok, _} = Matches.ready(code, actor, true)
    end

    assert {:error, :full} = Matches.join(code, List.last(actors), "Player 9")
    assert {:ok, _} = Matches.start(code, host)
    snapshot = Repo.get_by!(Match, code: code).snapshot

    for {round, index} <- Enum.with_index(snapshot["rounds"]) do
      for actor <- Enum.take(actors, 8),
          do: Matches.submit(code, actor, index, round["book"], to_string(round["chapter"]))

      assert {:ok, %{status: "reveal", result: %{guesses: guesses}}} = Matches.fetch(code, host)
      assert length(guesses) == 8
      assert {:ok, _} = Matches.advance(code, host, index)
    end

    assert {:ok, %{status: "finished", players: players}} = Matches.fetch(code, host)
    assert Enum.all?(players, &(&1.score == 5000))
    assert Repo.aggregate(Guess, :count) == 40
  end

  test "failed preparation creates no room or memberships" do
    {user, _} = account("reader")
    Scripguessr.Corpus.Fake.reset(:down)
    assert {:error, _} = Matches.create(user.id)
    assert Repo.aggregate(Match, :count) == 0
    assert Repo.aggregate(Player, :count) == 0
  end
end
