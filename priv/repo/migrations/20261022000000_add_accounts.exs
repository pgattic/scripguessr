defmodule Scripguessr.Repo.Migrations.AddAccounts do
  use Ecto.Migration

  def change do
    create table(:users, primary_key: false) do
      add :id, :binary_id, primary_key: true
      add :username, :string, null: false
      add :username_normalized, :string, null: false
      add :hashed_password, :string, null: false
      add :recovery_hash, :binary, null: false
      timestamps(type: :utc_datetime_usec)
    end

    create unique_index(:users, [:username_normalized])

    create table(:user_tokens, primary_key: false) do
      add :id, :binary_id, primary_key: true
      add :user_id, references(:users, type: :binary_id, on_delete: :delete_all), null: false
      add :token, :binary, null: false
      add :context, :string, null: false
      add :expires_at, :utc_datetime_usec, null: false
      timestamps(type: :utc_datetime_usec, updated_at: false)
    end

    create unique_index(:user_tokens, [:token])
    create index(:user_tokens, [:user_id, :context])
    create index(:user_tokens, [:expires_at])

    alter table(:games) do
      add :user_id, references(:users, type: :binary_id, on_delete: :delete_all)
    end

    create index(:games, [:user_id, :inserted_at])

    create table(:game_rewards, primary_key: false) do
      add :game_id, references(:games, type: :binary_id, on_delete: :delete_all),
        primary_key: true

      add :user_id, references(:users, type: :binary_id, on_delete: :delete_all), null: false
      add :completion_xp, :integer, null: false
      add :accuracy_xp, :integer, null: false
      timestamps(type: :utc_datetime_usec, updated_at: false)
    end

    create index(:game_rewards, [:user_id])

    create constraint(:game_rewards, :nonnegative_xp,
             check: "completion_xp >= 0 AND accuracy_xp >= 0"
           )
  end
end
