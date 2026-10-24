defmodule Scripguessr.Matches.Player do
  use Ecto.Schema
  @primary_key {:id, :binary_id, autogenerate: true}
  schema "match_players" do
    belongs_to :match, Scripguessr.Matches.Match, type: :binary_id
    belongs_to :user, Scripguessr.Accounts.User, type: :binary_id
    field :guest_hash, :binary
    field :guest_name, :string
    field :host, :boolean, default: false
    field :seat, :integer
    field :ready, :boolean, default: false
    field :score, :integer, default: 0
    has_many :guesses, Scripguessr.Matches.Guess
    timestamps(type: :utc_datetime_usec)
  end
end
