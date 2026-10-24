defmodule ScripguessrWeb.LeaderboardLive do
  use ScripguessrWeb, :live_view
  import ScripguessrWeb.Layouts, only: [avatar: 1]
  alias Scripguessr.Leaderboards

  @impl true
  def mount(_, _, socket), do: {:ok, socket}

  @impl true
  def handle_params(params, _, socket) do
    daily? = params["board"] == "daily"
    today = Date.utc_today()

    day =
      case parse_date(params["date"]) do
        {:ok, day} -> if Date.compare(day, today) == :gt, do: today, else: day
        _ -> today
      end

    user_id = socket.assigns.current_user && socket.assigns.current_user.id

    {:noreply,
     assign(socket,
       daily?: daily?,
       day: day,
       today: today,
       players: if(daily?, do: Leaderboards.daily(day), else: Leaderboards.xp()),
       own_rank: if(daily?, do: Leaderboards.daily_rank(day, user_id))
     )}
  end

  @impl true
  def render(assigns) do
    ~H"""
    <section class="leaderboard-page" aria-labelledby="leaderboard-heading">
      <div class="leaderboard-heading">
        <div>
          <h1 id="leaderboard-heading">Leaderboard</h1>
          <p class="muted">Top 50 players · {if @daily?, do: "#{@day} · UTC", else: "All-time XP"}</p>
        </div>
        <a href="/" class="text-button">Play a game →</a>
      </div>
      <nav class="personal-nav" aria-label="Leaderboard category">
        <.link patch="/leaderboard" aria-current={if !@daily?, do: "page"}>Lifetime XP</.link>
        <.link patch="/leaderboard?board=daily" aria-current={if @daily?, do: "page"}>Daily game</.link>
      </nav>
      <form
        :if={@daily?}
        id="daily-board-date"
        phx-change="date"
        phx-submit="date"
        class="daily-board-date"
      >
        <label for="daily-date">Day <span class="muted">(UTC)</span></label>
        <input id="daily-date" type="date" name="date" value={@day} max={@today} required />
        <button class="text-button">View</button>
      </form>
      <div :if={@players == []} id="leaderboard-empty" class="panel saved-empty">
        <h2>{if @daily?, do: "No scores for this day.", else: "The first spot is open."}</h2>
        <p class="muted">
          {if @daily?,
            do: "No completed daily games for this date.",
            else: "Finish a game with an account to join the board."}
        </p>
        <a :if={@daily? && @day == @today} href="/daily" class="text-button">Play today’s game →</a>
        <a :if={!@current_user} href="/account/register" class="text-button">Create an account →</a>
      </div>
      <div :if={@players != []} class="panel leaderboard-panel">
        <table id="leaderboard" class="leaderboard-table">
          <caption class="sr-only">
            Top 50 players by {if @daily?, do: "daily score", else: "lifetime XP"}. Equal scores share a rank.
          </caption>
          <thead>
            <tr>
              <th scope="col">Rank</th><th scope="col">Player</th><th :if={!@daily?} scope="col">
                Level
              </th><th scope="col">
                {if @daily?, do: "Score", else: "XP"}
              </th>
            </tr>
          </thead>
          <tbody>
            <tr
              :for={player <- @players}
              class={if @current_user && @current_user.id == player.user_id, do: "leaderboard-self"}
            >
              <td class="leaderboard-rank">{player.rank}</td>
              <th scope="row">
                <span class="leaderboard-player">
                  <.avatar />
                  <span class="leaderboard-username" title={player.username}>{player.username}</span>
                  <span
                    :if={@current_user && @current_user.id == player.user_id}
                    class="leaderboard-you"
                  >You</span>
                </span>
              </th>
              <td :if={!@daily?}>{player.level}</td>
              <td class="leaderboard-xp">
                {format_xp(if @daily?, do: player.score, else: player.xp)}
              </td>
            </tr>
          </tbody>
        </table>
      </div>
      <p
        :if={@own_rank && !Enum.any?(@players, &(&1.user_id == @own_rank.user_id))}
        id="daily-own-rank"
        class="panel saved-card"
      >
        Your rank: <strong>#{@own_rank.rank}</strong> · {format_xp(@own_rank.score)} points
      </p>
    </section>
    """
  end

  @impl true
  def handle_event("date", %{"date" => date}, socket) do
    case parse_date(date) do
      {:ok, day} ->
        if Date.compare(day, socket.assigns.today) != :gt do
          {:noreply,
           push_patch(socket,
             to:
               "/leaderboard?" <> URI.encode_query(%{board: "daily", date: Date.to_iso8601(day)})
           )}
        else
          {:noreply, socket}
        end

      _ ->
        {:noreply, socket}
    end
  end

  def handle_event(_, _, socket), do: {:noreply, socket}
  defp parse_date(value) when is_binary(value), do: Date.from_iso8601(value)
  defp parse_date(_), do: :error

  defp format_xp(xp) do
    xp
    |> Integer.to_charlist()
    |> Enum.reverse()
    |> Enum.chunk_every(3)
    |> Enum.intersperse(~c",")
    |> List.flatten()
    |> Enum.reverse()
    |> List.to_string()
  end
end
