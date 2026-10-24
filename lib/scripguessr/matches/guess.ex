defmodule Scripguessr.Matches.Guess do
  use Ecto.Schema
  @primary_key {:id, :binary_id, autogenerate: true}
  schema "match_guesses" do
    belongs_to :player, Scripguessr.Matches.Player, type: :binary_id
    field :round, :integer
    field :book, :string
    field :chapter, :integer
    field :distance, :integer
    field :points, :integer
    timestamps(type: :utc_datetime_usec, updated_at: false)
  end
end
