defmodule Scripguessr.Corpus.RequestCacheTest do
  use ExUnit.Case, async: true
  import ExUnit.CaptureLog
  alias Scripguessr.Corpus.RequestCache

  defp cache(opts \\ []), do: start_supervised!({RequestCache, opts})

  defp blocked(parent, key) do
    fn ->
      send(parent, {:started, key, self()})

      receive do
        :continue -> {:ok, key}
      end
    end
  end

  test "duplicate misses share a worker while cached hits and other keys stay responsive" do
    cache = cache()
    parent = self()
    assert {:ok, :hot} = RequestCache.fetch(cache, :hot, fn -> {:ok, :hot} end)

    callers =
      for _ <- 1..8,
          do: Task.async(fn -> RequestCache.fetch(cache, :slow, blocked(parent, :slow)) end)

    assert_receive {:started, :slow, worker}
    refute_receive {:started, :slow, _}, 50

    assert {:ok, :hot} =
             RequestCache.fetch(cache, :hot, fn -> flunk("must use cached result") end)

    assert {:ok, :other} = RequestCache.fetch(cache, :other, fn -> {:ok, :other} end)
    send(worker, :continue)
    assert Enum.all?(callers, &(Task.await(&1) == {:ok, :slow}))
  end

  test "concurrency and waiting work are bounded" do
    cache = cache(concurrency: 1, max_pending: 2)
    parent = self()
    first = Task.async(fn -> RequestCache.fetch(cache, :first, blocked(parent, :first)) end)
    assert_receive {:started, :first, worker}
    second = Task.async(fn -> RequestCache.fetch(cache, :second, blocked(parent, :second)) end)
    assert Task.yield(second, 50) == nil
    refute_receive {:started, :second, _}, 20
    assert {:error, :busy} = RequestCache.fetch(cache, :overflow, fn -> flunk("must not run") end)
    send(worker, :continue)
    assert Task.await(first) == {:ok, :first}
    assert_receive {:started, :second, next_worker}
    send(next_worker, :continue)
    assert Task.await(second) == {:ok, :second}
  end

  test "duplicate waiters are bounded too" do
    cache = cache(max_waiters: 1)
    parent = self()
    first = Task.async(fn -> RequestCache.fetch(cache, :slow, blocked(parent, :slow)) end)
    assert_receive {:started, :slow, worker}
    assert {:error, :busy} = RequestCache.fetch(cache, :slow, fn -> flunk("must not run") end)
    send(worker, :continue)
    assert Task.await(first) == {:ok, :slow}
  end

  test "deadlines kill stalled workers, return an error, and allow a retry" do
    cache = cache(timeout: 100)
    parent = self()
    caller = Task.async(fn -> RequestCache.fetch(cache, :slow, blocked(parent, :slow)) end)
    assert_receive {:started, :slow, worker}
    ref = Process.monitor(worker)
    assert Task.await(caller) == {:error, :timeout}
    assert_receive {:DOWN, ^ref, :process, ^worker, :killed}
    assert {:ok, :recovered} = RequestCache.fetch(cache, :slow, fn -> {:ok, :recovered} end)
  end

  test "clear cancels outstanding work and prevents old results from refilling the cache" do
    cache = cache()
    parent = self()
    caller = Task.async(fn -> RequestCache.fetch(cache, :slow, blocked(parent, :slow)) end)
    assert_receive {:started, :slow, worker}
    ref = Process.monitor(worker)
    assert :ok = RequestCache.clear(cache)
    assert Task.await(caller) == {:error, :cancelled}
    assert_receive {:DOWN, ^ref, :process, ^worker, :killed}
    assert {:ok, :fresh} = RequestCache.fetch(cache, :slow, fn -> {:ok, :fresh} end)
  end

  test "worker crashes are contained and failures are not cached" do
    cache = cache()

    capture_log(fn ->
      assert {:error, :unavailable} =
               RequestCache.fetch(cache, :crash, fn -> raise "upstream failure" end)
    end)

    assert Process.alive?(cache)
    assert {:ok, :retry} = RequestCache.fetch(cache, :crash, fn -> {:ok, :retry} end)

    assert {:error, :unavailable} =
             RequestCache.fetch(cache, :error, fn -> {:error, :unavailable} end)

    assert {:ok, :retry} = RequestCache.fetch(cache, :error, fn -> {:ok, :retry} end)
  end

  test "stopping a cache stops its outstanding workers" do
    cache = cache()
    parent = self()

    caller =
      Task.async(fn -> catch_exit(RequestCache.fetch(cache, :slow, blocked(parent, :slow))) end)

    assert_receive {:started, :slow, worker}
    ref = Process.monitor(worker)
    stop_supervised!(RequestCache)
    assert_receive {:DOWN, ^ref, :process, ^worker, :killed}
    assert {:shutdown, _} = Task.await(caller)
  end

  test "cache capacity, expiration, and uncached work are respected" do
    cache = cache(limit: 1, ttl: 30)
    assert {:ok, :a} = RequestCache.fetch(cache, :a, fn -> {:ok, :a} end)
    assert {:ok, :b} = RequestCache.fetch(cache, :b, fn -> {:ok, :b} end)
    assert {:ok, :new_a} = RequestCache.fetch(cache, :a, fn -> {:ok, :new_a} end)
    Process.sleep(40)
    assert {:ok, :expired} = RequestCache.fetch(cache, :a, fn -> {:ok, :expired} end)
    assert {:ok, 1} = RequestCache.fetch(cache, :uncached, fn -> {:ok, 1} end, ttl: 0)
    assert {:ok, 2} = RequestCache.fetch(cache, :uncached, fn -> {:ok, 2} end, ttl: 0)
  end
end
