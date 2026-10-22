defmodule Scripguessr.Accounts.RateLimit do
  @moduledoc "Bounded per-node rate limit for registration, sign-in, and recovery attempts."
  use GenServer
  @window 600
  @limit 10
  @max_keys 10_000

  def start_link(opts), do: GenServer.start_link(__MODULE__, opts, name: __MODULE__)
  def allow?(key), do: GenServer.call(__MODULE__, {:allow, key})
  def clear, do: GenServer.call(__MODULE__, :clear)
  @impl true
  def init(_), do: {:ok, %{}}
  @impl true
  def handle_call(:clear, _, _), do: {:reply, :ok, %{}}

  def handle_call({:allow, key}, _, state) do
    now = System.monotonic_time(:second)
    state = Map.reject(state, fn {_, {until, _}} -> until <= now end)
    {until, count} = Map.get(state, key, {now + @window, 0})
    allowed = count < @limit and (Map.has_key?(state, key) or map_size(state) < @max_keys)
    state = if allowed, do: Map.put(state, key, {until, count + 1}), else: state
    {:reply, allowed, state}
  end
end
