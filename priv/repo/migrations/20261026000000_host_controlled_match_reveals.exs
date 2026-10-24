defmodule Scripguessr.Repo.Migrations.HostControlledMatchReveals do
  use Ecto.Migration

  def up do
    drop constraint(:matches, :valid_match_state)
    execute "UPDATE matches SET deadline = NULL WHERE status = 'reveal'"

    create constraint(:matches, :valid_match_state,
             check:
               "capacity >= 2 AND capacity <= 8 AND round BETWEEN 0 AND 4 AND status IN ('lobby', 'round', 'reveal', 'finished', 'cancelled') AND ((status = 'round') = (deadline IS NOT NULL))"
           )
  end

  def down do
    drop constraint(:matches, :valid_match_state)

    execute "UPDATE matches SET status = 'finished', deadline = NULL WHERE status = 'reveal' AND round = 4"

    execute "UPDATE matches SET deadline = CURRENT_TIMESTAMP + INTERVAL '12 seconds' WHERE status = 'reveal'"

    create constraint(:matches, :valid_match_state,
             check:
               "capacity >= 2 AND capacity <= 8 AND round BETWEEN 0 AND 4 AND status IN ('lobby', 'round', 'reveal', 'finished', 'cancelled') AND ((status IN ('round', 'reveal')) = (deadline IS NOT NULL))"
           )
  end
end
