defmodule Scripguessr.Accounts.Token do
  use Ecto.Schema
  @primary_key {:id, :binary_id, autogenerate: true}
  @foreign_key_type :binary_id

  schema "user_tokens" do
    belongs_to :user, Scripguessr.Accounts.User
    field :token, :binary
    field :context, :string
    field :expires_at, :utc_datetime_usec
    timestamps(type: :utc_datetime_usec, updated_at: false)
  end
end
