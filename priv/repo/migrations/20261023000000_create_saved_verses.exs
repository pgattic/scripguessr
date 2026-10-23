defmodule Scripguessr.Repo.Migrations.CreateSavedVerses do
  use Ecto.Migration

  def change do
    create table(:saved_verses, primary_key: false) do
      add :id, :binary_id, primary_key: true
      add :user_id, references(:users, type: :binary_id, on_delete: :delete_all), null: false
      add :edition, :string, null: false
      add :book, :string, null: false
      add :chapter, :integer, null: false
      add :verse, :integer, null: false
      add :text, :text, null: false
      add :release, :string, null: false
      timestamps(type: :utc_datetime_usec, updated_at: false)
    end

    create unique_index(:saved_verses, [:user_id, :edition, :book, :chapter, :verse])
    create index(:saved_verses, [:user_id, :inserted_at])
    create constraint(:saved_verses, :positive_reference, check: "chapter > 0 AND verse > 0")
  end
end
