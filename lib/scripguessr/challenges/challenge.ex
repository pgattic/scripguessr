defmodule Scripguessr.Challenges.Challenge do
  use Ecto.Schema
  @primary_key {:id, :binary_id, autogenerate: true}

  schema "challenges" do
    field :day, :date
    field :snapshot, :map
    timestamps(type: :utc_datetime_usec, updated_at: false)
  end
end
