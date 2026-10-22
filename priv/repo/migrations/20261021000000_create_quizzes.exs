defmodule Scripguessr.Repo.Migrations.CreateQuizzes do
  use Ecto.Migration

  def change do
    create table(:quizzes, primary_key: false) do
      add :id, :binary_id, primary_key: true
      add :owner_hash, :binary, null: false
      add :snapshot, :map, null: false
      timestamps(type: :utc_datetime_usec)
    end

    create index(:quizzes, [:owner_hash])
  end
end
