defmodule Scripguessr.Challenges do
  @moduledoc "Frozen shared prompts, independent of each player's game progress."
  alias Scripguessr.{Repo, Corpus.Cache}
  alias Scripguessr.Challenges.Challenge

  def daily do
    day = Date.utc_today()

    case Repo.get_by(Challenge, day: day) do
      nil ->
        with {:ok, snapshot} <- Cache.prepare() do
          # Preparation happens outside a database transaction. The unique day
          # chooses one immutable prompt set even across simultaneous app nodes.
          Repo.insert!(%Challenge{day: day, snapshot: snapshot},
            on_conflict: :nothing,
            conflict_target: :day
          )

          {:ok, Repo.get_by!(Challenge, day: day)}
        end

      challenge ->
        {:ok, challenge}
    end
  end
end
