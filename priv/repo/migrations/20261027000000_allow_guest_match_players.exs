defmodule Scripguessr.Repo.Migrations.AllowGuestMatchPlayers do
  use Ecto.Migration

  def up do
    alter table(:match_players) do
      modify :user_id, :binary_id, null: true
      add :guest_hash, :binary
      add :guest_name, :string
      add :host, :boolean, null: false, default: false
    end

    execute """
    UPDATE match_players AS p SET host = true
    FROM matches AS m WHERE p.match_id = m.id AND p.user_id = m.host_id
    """

    alter table(:matches) do
      remove :host_id
    end

    create unique_index(:match_players, [:match_id, :guest_hash])
    create unique_index(:match_players, [:match_id], where: "host", name: :match_players_one_host)

    create constraint(:match_players, :match_player_identity,
             check: """
             (user_id IS NOT NULL AND guest_hash IS NULL AND guest_name IS NULL) OR
             (user_id IS NULL AND guest_hash IS NOT NULL AND octet_length(guest_hash) = 32
              AND guest_name IS NOT NULL AND char_length(guest_name) BETWEEN 1 AND 32)
             """
           )
  end

  def down do
    # Guest-hosted rooms cannot be represented by the account-only schema.
    execute """
    DELETE FROM matches AS m WHERE EXISTS
    (SELECT 1 FROM match_players AS p WHERE p.match_id = m.id AND p.host AND p.user_id IS NULL)
    """

    execute "DELETE FROM match_players WHERE user_id IS NULL"

    alter table(:matches) do
      add :host_id, references(:users, type: :binary_id)
    end

    execute """
    UPDATE matches AS m SET host_id = p.user_id
    FROM match_players AS p WHERE p.match_id = m.id AND p.host
    """

    alter table(:matches) do
      modify :host_id, :binary_id, null: false
    end

    drop constraint(:match_players, :match_player_identity)
    drop index(:match_players, [:match_id], name: :match_players_one_host)
    drop index(:match_players, [:match_id, :guest_hash])

    alter table(:match_players) do
      remove :host
      remove :guest_name
      remove :guest_hash
      modify :user_id, :binary_id, null: false
    end
  end
end
