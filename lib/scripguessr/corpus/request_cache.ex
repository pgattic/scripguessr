defmodule Scripguessr.Corpus.RequestCache do
  @moduledoc "Bounded, single-flight cache. Network work runs in supervised tasks, never in the server."
  use GenServer

  def start_link(opts), do: GenServer.start_link(__MODULE__, opts, Keyword.take(opts, [:name]))

  def fetch(server, key, fetch, opts \\ []),
    do: GenServer.call(server, {:fetch, key, fetch, opts}, :infinity)

  def invalidate(server, key), do: GenServer.call(server, {:invalidate, key})
  def clear(server), do: GenServer.call(server, :clear)

  @impl true
  def init(opts) do
    # Run terminate/2 on supervisor shutdown so in-flight workers cannot outlive the cache.
    Process.flag(:trap_exit, true)

    {:ok,
     %{
       entries: %{},
       pending: %{},
       refs: %{},
       sequence: 0,
       concurrency: Keyword.get(opts, :concurrency, 4),
       max_pending: Keyword.get(opts, :max_pending, 32),
       max_waiters: Keyword.get(opts, :max_waiters, 64),
       timeout: Keyword.get(opts, :timeout, 60_000),
       ttl: Keyword.get(opts, :ttl, :timer.minutes(15)),
       limit: Keyword.get(opts, :limit, 160)
     }}
  end

  @impl true
  def handle_call({:fetch, key, fetch, opts}, from, state) do
    case state.entries[key] do
      {value, expires} ->
        if expires > now() do
          {:reply, {:ok, value}, state}
        else
          enqueue(key, fetch, opts, from, %{state | entries: Map.delete(state.entries, key)})
        end

      _ ->
        enqueue(key, fetch, opts, from, state)
    end
  end

  def handle_call({:invalidate, key}, _, state),
    do: {:reply, :ok, %{state | entries: Map.delete(state.entries, key)}}

  def handle_call(:clear, _, state) do
    Enum.each(state.pending, fn {_, job} ->
      stop(job)
      Enum.each(job.waiters, &GenServer.reply(&1, {:error, :cancelled}))
    end)

    {:reply, :ok, %{state | entries: %{}, pending: %{}, refs: %{}}}
  end

  @impl true
  def handle_info({ref, result}, state) when is_reference(ref) do
    case state.refs[ref] do
      nil ->
        {:noreply, state}

      key ->
        job = state.pending[key]
        Process.demonitor(ref, [:flush])
        Process.cancel_timer(job.timer)
        state = finish(state, key, result)
        {:noreply, dispatch(state)}
    end
  end

  def handle_info({:DOWN, ref, :process, _, _reason}, state) do
    case state.refs[ref] do
      nil ->
        {:noreply, state}

      key ->
        Process.cancel_timer(state.pending[key].timer)
        {:noreply, state |> finish(key, {:error, :unavailable}) |> dispatch()}
    end
  end

  def handle_info({:deadline, key, token}, state) do
    case state.pending[key] do
      %{token: ^token} = job ->
        stop(job)
        {:noreply, state |> finish(key, {:error, :timeout}) |> dispatch()}

      _ ->
        {:noreply, state}
    end
  end

  @impl true
  def terminate(_reason, state) do
    Enum.each(state.pending, fn {_, job} -> stop(job) end)
  end

  defp enqueue(key, fetch, opts, from, state) do
    case state.pending[key] do
      %{waiters: waiters} = job when length(waiters) < state.max_waiters ->
        {:noreply, put_in(state.pending[key], %{job | waiters: [from | waiters]})}

      nil when map_size(state.pending) < state.max_pending ->
        token = make_ref()

        job = %{
          fetch: fetch,
          ttl: Keyword.get(opts, :ttl, state.ttl),
          waiters: [from],
          token: token,
          timer: Process.send_after(self(), {:deadline, key, token}, state.timeout),
          task: nil,
          sequence: state.sequence
        }

        state = %{state | pending: Map.put(state.pending, key, job), sequence: state.sequence + 1}
        {:noreply, dispatch(state)}

      _ ->
        {:reply, {:error, :busy}, state}
    end
  end

  defp dispatch(state) when map_size(state.refs) >= state.concurrency, do: state

  defp dispatch(state) do
    queued = Enum.filter(state.pending, fn {_, job} -> is_nil(job.task) end)

    case Enum.min_by(queued, fn {_, job} -> job.sequence end, fn -> nil end) do
      nil ->
        state

      {key, job} ->
        task = Task.Supervisor.async_nolink(Scripguessr.Corpus.Tasks, job.fetch)

        state = %{
          state
          | pending: Map.put(state.pending, key, %{job | task: task}),
            refs: Map.put(state.refs, task.ref, key)
        }

        dispatch(state)
    end
  end

  defp finish(state, key, result) do
    {job, pending} = Map.pop(state.pending, key)
    Enum.each(job.waiters, &GenServer.reply(&1, result))
    refs = if job.task, do: Map.delete(state.refs, job.task.ref), else: state.refs

    entries =
      case result do
        {:ok, value} when job.ttl > 0 -> store(state.entries, key, value, job.ttl, state.limit)
        _ -> state.entries
      end

    %{state | pending: pending, refs: refs, entries: entries}
  end

  defp store(entries, key, value, ttl, limit) do
    entries =
      if map_size(entries) >= limit do
        {oldest, _} = Enum.min_by(entries, fn {_, {_, expires}} -> expires end)
        Map.delete(entries, oldest)
      else
        entries
      end

    Map.put(entries, key, {value, now() + ttl})
  end

  defp stop(job) do
    Process.cancel_timer(job.timer)

    if job.task do
      Process.demonitor(job.task.ref, [:flush])
      Process.exit(job.task.pid, :kill)
    end
  end

  defp now, do: System.monotonic_time(:millisecond)
end
