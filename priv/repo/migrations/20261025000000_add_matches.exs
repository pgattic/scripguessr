defmodule Scripguessr.Repo.Migrations.AddMatches do
  use Ecto.Migration

  def change do
    create table(:matches, primary_key: false) do
      add :id, :binary_id, primary_key: true
      add :code, :string, null: false
      add :host_id, references(:users, type: :binary_id), null: false
      add :capacity, :integer, null: false, default: 2
      add :status, :string, null: false, default: "lobby"
      add :round, :integer, null: false, default: 0
      add :deadline, :utc_datetime_usec
      add :snapshot, :map, null: false
      timestamps(type: :utc_datetime_usec)
    end

    create unique_index(:matches, [:code])
    create index(:matches, [:status, :deadline])

    create constraint(:matches, :valid_match_state,
             check:
               "capacity >= 2 AND capacity <= 8 AND round BETWEEN 0 AND 4 AND status IN ('lobby', 'round', 'reveal', 'finished', 'cancelled') AND ((status IN ('round', 'reveal')) = (deadline IS NOT NULL))"
           )

    create table(:match_players, primary_key: false) do
      add :id, :binary_id, primary_key: true
      add :match_id, references(:matches, type: :binary_id, on_delete: :delete_all), null: false
      add :user_id, references(:users, type: :binary_id), null: false
      add :seat, :integer, null: false
      add :ready, :boolean, null: false, default: false
      add :score, :integer, null: false, default: 0
      timestamps(type: :utc_datetime_usec)
    end

    create unique_index(:match_players, [:match_id, :user_id])
    create unique_index(:match_players, [:match_id, :seat])
    create index(:match_players, [:user_id, :inserted_at])

    create constraint(:match_players, :valid_match_player,
             check: "seat >= 0 AND score BETWEEN 0 AND 5000"
           )

    create table(:match_guesses, primary_key: false) do
      add :id, :binary_id, primary_key: true

      add :player_id, references(:match_players, type: :binary_id, on_delete: :delete_all),
        null: false

      add :round, :integer, null: false
      add :book, :string, null: false
      add :chapter, :integer, null: false
      add :distance, :integer, null: false
      add :points, :integer, null: false
      timestamps(type: :utc_datetime_usec, updated_at: false)
    end

    create unique_index(:match_guesses, [:player_id, :round])

    create constraint(:match_guesses, :valid_match_guess,
             check:
               "round BETWEEN 0 AND 4 AND chapter > 0 AND distance >= 0 AND points BETWEEN 0 AND 1000"
           )
  end
end
