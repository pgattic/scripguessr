defmodule Scripguessr.CorpusHTTPTest do
  use ExUnit.Case, async: false
  alias Scripguessr.Corpus.HTTP

  setup do
    Application.put_env(:scripguessr, :corpus_http_options, plug: {Req.Test, __MODULE__})
    on_exit(fn -> Application.delete_env(:scripguessr, :corpus_http_options) end)
    :ok
  end

  test "decodes the release envelope and preserves truncation" do
    Req.Test.stub(__MODULE__, fn conn ->
      Req.Test.json(conn, %{release: "r1", rows: [%{book: "test"}], truncated: true})
    end)

    assert {:ok, %{release: "r1", truncated: true}} = HTTP.query("select 1")
  end

  test "handles rate limits, busy responses, malformed envelopes, and transport errors" do
    for status <- [429, 503] do
      Req.Test.stub(__MODULE__, fn conn -> Plug.Conn.send_resp(conn, status, "unavailable") end)
      assert {:error, {:http, ^status}} = HTTP.query("select 1")
    end

    Req.Test.stub(__MODULE__, fn conn -> Req.Test.json(conn, %{rows: []}) end)
    assert {:error, :invalid_response} = HTTP.query("select 1")
    Req.Test.stub(__MODULE__, fn conn -> Req.Test.transport_error(conn, :timeout) end)
    assert {:error, :unavailable} = HTTP.query("select 1")
  end
end
