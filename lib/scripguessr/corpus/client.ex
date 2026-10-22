defmodule Scripguessr.Corpus.Client do
  @callback query(String.t()) ::
              {:ok, %{release: String.t(), rows: list(), truncated: boolean()}} | {:error, term()}
end
