defmodule Scripguessr.Matches.Clock do
  use GenServer
  require Logger
  def start_link(_), do: GenServer.start_link(__MODULE__, nil, name: __MODULE__)
  @impl true
  def init(_) do
    send(self(), :tick)
    {:ok, nil}
  end

  @impl true
  def handle_info(:tick, state) do
    try do
      Scripguessr.Matches.tick()
    rescue
      error -> Logger.error("Match clock failed: #{Exception.message(error)}")
    after
      Process.send_after(self(), :tick, 1000)
    end

    {:noreply, state}
  end
end
