defmodule Scripguessr.SavedVerses.Verse do
  use Ecto.Schema

  @primary_key {:id, :binary_id, autogenerate: true}
  schema "saved_verses" do
    field :user_id, :binary_id
    field :edition, :string
    field :book, :string
    field :chapter, :integer
    field :verse, :integer
    field :text, :string
    field :release, :string
    timestamps(type: :utc_datetime_usec, updated_at: false)
  end
end
