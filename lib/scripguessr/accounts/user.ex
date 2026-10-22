defmodule Scripguessr.Accounts.User do
  use Ecto.Schema
  import Ecto.Changeset
  @primary_key {:id, :binary_id, autogenerate: true}

  schema "users" do
    field :username, :string
    field :username_normalized, :string
    field :hashed_password, :string, redact: true
    field :recovery_hash, :binary, redact: true
    field :password, :string, virtual: true, redact: true
    field :password_confirmation, :string, virtual: true, redact: true
    timestamps(type: :utc_datetime_usec)
  end

  def registration_changeset(user, attrs) do
    user
    |> cast(attrs, [:username])
    |> update_change(:username, &String.trim/1)
    |> validate_required([:username])
    |> validate_length(:username, min: 3, max: 32)
    |> validate_format(:username, ~r/^[A-Za-z0-9_-]+$/,
      message: "must use letters, numbers, underscores, or hyphens"
    )
    |> normalize_username()
    |> unique_constraint(:username, name: :users_username_normalized_index)
    |> password_changeset(attrs)
  end

  def password_changeset(user, attrs) do
    user
    |> cast(attrs, [:password, :password_confirmation])
    |> validate_required([:password])
    |> validate_length(:password, min: 8, max: 128)
    |> validate_length(:password, max: 512, count: :bytes)
    |> validate_confirmation(:password, required: true)
  end

  defp normalize_username(changeset) do
    case get_change(changeset, :username) do
      nil -> changeset
      username -> put_change(changeset, :username_normalized, String.downcase(username))
    end
  end
end
