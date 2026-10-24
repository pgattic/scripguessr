defmodule ScripguessrWeb.LeaderboardLive do
  use ScripguessrWeb, :live_view
  import ScripguessrWeb.Layouts, only: [avatar: 1]
  alias Scripguessr.Leaderboards

  @impl true
  def mount(_, _, socket), do: {:ok, socket}

  @impl true
  def handle_params(_, _, socket) do
    {:noreply, assign(socket, :players, Leaderboards.xp())}
  end

  @impl true
  def render(assigns) do
    ~H"""
    <section class="leaderboard-page" aria-labelledby="leaderboard-heading">
      <div class="leaderboard-heading">
        <div>
          <h1 id="leaderboard-heading">Leaderboard</h1>
          <p class="muted">Top 50 players · All-time XP</p>
        </div>
        <a href="/" class="text-button">Play a game →</a>
      </div>
      <div :if={@players == []} id="leaderboard-empty" class="panel saved-empty">
        <h2>The first spot is open.</h2>
        <p class="muted">Finish a game with an account to join the board.</p>
        <a :if={!@current_user} href="/account/register" class="text-button">Create an account →</a>
      </div>
      <div :if={@players != []} class="panel leaderboard-panel">
        <table id="leaderboard" class="leaderboard-table">
          <caption class="sr-only">Top 50 players by lifetime XP. Equal XP shares a rank.</caption>
          <thead>
            <tr>
              <th scope="col">Rank</th><th scope="col">Player</th><th scope="col">Level</th><th scope="col">
                XP
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
              <td>{player.level}</td>
              <td class="leaderboard-xp">{format_xp(player.xp)}</td>
            </tr>
          </tbody>
        </table>
      </div>
    </section>
    """
  end

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
