defmodule Scripguessr.Repo.Migrations.AddDailyChallenges do
  use Ecto.Migration

  def change do
    create table(:challenges, primary_key: false) do
      add :id, :binary_id, primary_key: true
      add :day, :date, null: false
      add :snapshot, :map, null: false
      timestamps(type: :utc_datetime_usec, updated_at: false)
    end

    create unique_index(:challenges, [:day])

    alter table(:games) do
      add :challenge_id, references(:challenges, type: :binary_id)
      add :score, :integer
    end

    create unique_index(:games, [:challenge_id, :user_id])

    create constraint(:games, :challenge_requires_account,
             check: "challenge_id IS NULL OR user_id IS NOT NULL"
           )

    create constraint(:games, :valid_score, check: "score BETWEEN 0 AND 5000")
  end
end
