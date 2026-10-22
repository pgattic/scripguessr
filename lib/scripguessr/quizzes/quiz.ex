defmodule Scripguessr.Quizzes.Quiz do
  use Ecto.Schema
  @primary_key {:id, :binary_id, autogenerate: true}
  schema "quizzes" do
    field :owner_hash, :binary
    field :snapshot, :map
    timestamps(type: :utc_datetime_usec)
  end
end
