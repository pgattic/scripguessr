defmodule Scripguessr.Repo.Migrations.CreateGames do
  use Ecto.Migration

  def change do
    create table(:games, primary_key: false) do
      add :id, :binary_id, primary_key: true
      add :owner_hash, :binary, null: false
      add :snapshot, :map, null: false
      timestamps(type: :utc_datetime_usec)
    end

    create index(:games, [:owner_hash])
  end
end
