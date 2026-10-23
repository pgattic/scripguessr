defmodule Scripguessr.Progression.Reward do
  use Ecto.Schema
  @primary_key {:game_id, :binary_id, autogenerate: false}

  schema "game_rewards" do
    field :user_id, :binary_id
    field :completion_xp, :integer
    field :accuracy_xp, :integer
    timestamps(type: :utc_datetime_usec, updated_at: false)
  end
end
