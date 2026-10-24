defmodule Scripguessr.Matches.Identity do
  @moduledoc "Trusted match identity assembled from the account and signed browser session."
  defstruct [:user_id, :guest_hash]

  def new(user_id, owner) do
    hash =
      if is_binary(owner) && Regex.match?(~r/\A[A-Za-z0-9_-]{43}\z/, owner),
        do: :crypto.hash(:sha256, owner)

    %__MODULE__{user_id: user_id, guest_hash: hash}
  end
end
