defmodule Scripguessr.SavedVerses.Verse do
  use Ecto.Schema
  import Ecto.Changeset

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

  def changeset(verse, attrs) do
    verse
    |> cast(attrs, [:user_id, :edition, :book, :chapter, :verse, :text, :release])
    |> validate_required([:user_id, :edition, :book, :chapter, :verse, :text, :release])
    |> validate_number(:chapter, greater_than: 0)
    |> validate_number(:verse, greater_than: 0)
  end
end
