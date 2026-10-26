defmodule Scripguessr.Matches.Match do
  use Ecto.Schema
  @primary_key {:id, :binary_id, autogenerate: true}
  schema "matches" do
    field :code, :string
    field :capacity, :integer, default: 8
    field :status, :string, default: "lobby"
    field :round, :integer, default: 0
    field :deadline, :utc_datetime_usec
    field :snapshot, :map
    has_many :players, Scripguessr.Matches.Player
    timestamps(type: :utc_datetime_usec)
  end
end
