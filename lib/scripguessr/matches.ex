defmodule Scripguessr.Matches do
  @moduledoc "Private invite matches with database-authoritative rounds and deadlines."
  import Ecto.Query
  alias Scripguessr.{Repo, Corpus.Cache}
  alias Scripguessr.Games.Rules
  alias Scripguessr.Matches.{Match, Player, Guess}

  @round_seconds 90

  def topic(id), do: "match:" <> id
  def round_seconds, do: @round_seconds
  def create(user_id, capacity \\ 2)
  def create(nil, _capacity), do: {:error, :account_required}

  def create(user_id, capacity) when is_binary(user_id) and capacity in 2..8 do
    with {:ok, snapshot} <- Cache.prepare() do
      Repo.transaction(fn ->
        match =
          Repo.insert!(%Match{
            host_id: user_id,
            capacity: capacity,
            snapshot: snapshot,
            code: Base.url_encode64(:crypto.strong_rand_bytes(18), padding: false)
          })

        Repo.insert!(%Player{match_id: match.id, user_id: user_id, seat: 0})
        match.code
      end)
    end
  end

  def recent(nil), do: []

  def recent(user_id) do
    Repo.all(
      from m in Match,
        join: p in Player,
        on: p.match_id == m.id,
        where: p.user_id == ^user_id,
        order_by: [desc: m.inserted_at, desc: m.id],
        limit: 20,
        select: %{code: m.code, status: m.status, inserted_at: m.inserted_at}
    )
  end

  def fetch(code, user_id) do
    case find(code) do
      nil ->
        {:error, :not_found}

      match ->
        if due?(match, DateTime.utc_now()), do: expire(match.id)
        {:ok, find(code) |> load_players() |> present(user_id)}
    end
  end

  def join(code, nil), do: account_required(code)

  def join(code, user_id) do
    mutate(code, fn match ->
      cond do
        member(match, user_id) ->
          {:ok, match.code}

        match.status != "lobby" ->
          {:error, :already_started}

        length(match.players) >= match.capacity ->
          {:error, :full}

        true ->
          seats = Enum.map(match.players, & &1.seat)
          seat = Enum.find(0..(match.capacity - 1), &(&1 not in seats))
          Repo.insert!(%Player{match_id: match.id, user_id: user_id, seat: seat})
          {:ok, match.code}
      end
    end)
  end

  def ready(code, user_id, ready?) when is_boolean(ready?) do
    mutate_member(code, user_id, fn match, player ->
      if match.status == "lobby" do
        player |> Ecto.Changeset.change(ready: ready?) |> Repo.update!()
        {:ok, match.code}
      else
        {:error, :already_started}
      end
    end)
  end

  def start(code, user_id) do
    mutate_member(code, user_id, fn match, _ ->
      cond do
        match.host_id != user_id ->
          {:error, :host_only}

        match.status != "lobby" ->
          {:error, :already_started}

        length(match.players) != match.capacity or !Enum.all?(match.players, & &1.ready) ->
          {:error, :not_ready}

        true ->
          change(match,
            status: "round",
            deadline: DateTime.add(DateTime.utc_now(), @round_seconds)
          )

          {:ok, match.code}
      end
    end)
  end

  def leave(code, user_id) do
    mutate_member(code, user_id, fn match, player ->
      if match.status == "lobby" do
        if match.host_id == user_id,
          do: change(match, status: "cancelled"),
          else: Repo.delete!(player)

        {:ok, match.code}
      else
        {:error, :already_started}
      end
    end)
  end

  def advance(code, user_id, index) do
    mutate_member(code, user_id, fn match, _ ->
      cond do
        match.host_id != user_id ->
          {:error, :host_only}

        match.status != "reveal" or match.round != index ->
          {:error, :stale_round}

        match.round == 4 ->
          change(match, status: "finished")
          {:ok, match.code}

        true ->
          change(match,
            status: "round",
            round: match.round + 1,
            deadline: DateTime.add(DateTime.utc_now(), @round_seconds)
          )

          {:ok, match.code}
      end
    end)
  end

  def submit(code, user_id, index, book, chapter) do
    mutate_member(code, user_id, fn match, player ->
      cond do
        match.status != "round" or match.round != index ->
          {:error, :stale_round}

        guess_for(player, index) ->
          {:ok, match.code}

        true ->
          snapshot = Map.put(match.snapshot, "current", index)

          case Rules.submit(snapshot, index, book, chapter) do
            {:ok, scored} ->
              guess = Enum.at(scored["rounds"], index)["guess"]

              Repo.insert!(%Guess{
                player_id: player.id,
                round: index,
                book: guess["book"],
                chapter: guess["chapter"],
                points: guess["points"],
                distance: guess["distance"]
              })

              match = load_players(match)

              if Enum.all?(match.players, &guess_for(&1, index)),
                do: close_round(match)

              {:ok, match.code}

            _ ->
              {:error, :invalid_guess}
          end
      end
    end)
  end

  # The clock and reconnects close elapsed rounds without extending guessing time.
  # Reveals have no deadline; only the host can start another round or the summary.
  def tick do
    now = DateTime.utc_now()

    Repo.all(
      from(m in Match,
        where: m.status == "round" and m.deadline <= ^now,
        order_by: m.deadline,
        limit: 100,
        select: m.id
      ),
      log: false
    )
    |> Enum.each(&expire/1)
  end

  defp expire(id) do
    {:ok, match} =
      Repo.transaction(fn ->
        Repo.one!(from m in Match, where: m.id == ^id, lock: "FOR UPDATE")
        |> load_players()
        |> settle(DateTime.utc_now())
      end)

    broadcast(match.id)
    :ok
  end

  defp settle(match, now) do
    if due?(match, now) do
      close_round(match)
    else
      match
    end
  end

  defp close_round(match) do
    Enum.each(match.players, fn player ->
      score = Enum.reduce(player.guesses, 0, &(&1.points + &2))
      player |> Ecto.Changeset.change(score: score) |> Repo.update!()
    end)

    change(match, status: "reveal", deadline: nil)
  end

  defp mutate_member(code, user_id, action) do
    mutate(code, fn match ->
      case member(match, user_id) do
        nil -> {:error, :not_a_player}
        player -> action.(match, player)
      end
    end)
  end

  defp mutate(code, action) do
    case Repo.transaction(fn ->
           case find(code, true) do
             nil ->
               {:error, :not_found}

             match ->
               match = match |> load_players() |> settle(DateTime.utc_now()) |> load_players()
               # Return errors normally so any elapsed-deadline transitions still commit.
               {match.id, action.(match)}
           end
         end) do
      {:ok, {id, result}} ->
        broadcast(id)
        result

      {:ok, result} ->
        result

      {:error, reason} ->
        {:error, reason}
    end
  end

  defp find(code, lock? \\ false) do
    if is_binary(code) and Regex.match?(~r/\A[A-Za-z0-9_-]{24}\z/, code) do
      query = from m in Match, where: m.code == ^code
      Repo.one(if lock?, do: from(m in query, lock: "FOR UPDATE"), else: query)
    end
  end

  defp load_players(match) do
    Repo.preload(
      match,
      [
        players:
          {from(p in Player, order_by: p.seat),
           [:guesses, user: from(u in Scripguessr.Accounts.User, select: [:id, :username])]}
      ],
      force: true
    )
  end

  defp member(_match, nil), do: nil
  defp member(match, user_id), do: Enum.find(match.players, &(&1.user_id == user_id))
  defp guess_for(player, round), do: Enum.find(player.guesses, &(&1.round == round))

  defp due?(%Match{status: "round", deadline: %DateTime{} = deadline}, now),
    do: DateTime.compare(deadline, now) != :gt

  defp due?(_match, _now), do: false
  defp change(match, attrs), do: match |> Ecto.Changeset.change(attrs) |> Repo.update!()

  defp broadcast(id),
    do: Phoenix.PubSub.broadcast(Scripguessr.PubSub, topic(id), {:match_updated, id})

  defp account_required(_code), do: {:error, :account_required}

  # This is the only player-facing projection. Never return the frozen snapshot,
  # opponents' pending guesses, scores for an open round, or future answers.
  defp present(match, user_id) do
    player = member(match, user_id)
    visible? = player != nil and match.status in ["reveal", "finished"]
    own_guess = player && guess_for(player, match.round)

    %{
      id: match.id,
      code: match.code,
      host_id: match.host_id,
      capacity: match.capacity,
      status: match.status,
      round: match.round,
      deadline: match.deadline,
      joined?: player != nil,
      all_ready?:
        length(match.players) == match.capacity and Enum.all?(match.players, & &1.ready),
      players:
        Enum.map(match.players, fn p ->
          %{
            user_id: p.user_id,
            username: p.user.username,
            ready: p.ready,
            score: if(player, do: p.score),
            submitted?: guess_for(p, match.round) != nil
          }
        end),
      chapters: if(player && match.status == "round", do: match.snapshot["chapters"], else: []),
      verse:
        if(player && match.status == "round",
          do: Enum.at(match.snapshot["rounds"], match.round)["text"]
        ),
      own_guess: if(own_guess, do: %{book: own_guess.book, chapter: own_guess.chapter}),
      result: if(visible?, do: round_result(match, match.round)),
      results:
        if(player && match.status == "finished",
          do: Enum.map(0..4, &round_result(match, &1)),
          else: []
        )
    }
  end

  defp round_result(match, index) do
    round = Enum.at(match.snapshot["rounds"], index)

    %{
      round: index,
      book: round["book"],
      chapter: round["chapter"],
      verse: round["verse"],
      text: round["text"],
      guesses:
        Enum.map(match.players, fn player ->
          guess = guess_for(player, index)

          %{
            user_id: player.user_id,
            username: player.user.username,
            book: guess && guess.book,
            chapter: guess && guess.chapter,
            points: if(guess, do: guess.points, else: 0),
            distance: guess && guess.distance
          }
        end)
    }
  end
end
