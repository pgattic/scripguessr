defmodule Scripguessr.Games.Game do
  use Ecto.Schema
  @primary_key {:id, :binary_id, autogenerate: true}
  schema "games" do
    belongs_to :user, Scripguessr.Accounts.User, type: :binary_id
    field :owner_hash, :binary
    field :snapshot, :map
    timestamps(type: :utc_datetime_usec)
  end
end
