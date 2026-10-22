defmodule Scripguessr.Accounts do
  @moduledoc "Username/password accounts with offline recovery and revocable sessions."
  import Ecto.Query
  alias Scripguessr.Repo
  alias Scripguessr.Accounts.{User, Token}

  @session_seconds 30 * 24 * 60 * 60
  @hash_opts [digest: :sha256, length: 32]

  def register(attrs) do
    changeset = User.registration_changeset(%User{}, attrs)

    if changeset.valid? do
      password = Ecto.Changeset.get_change(changeset, :password)
      {code, recovery_hash} = recovery_code()

      changeset =
        Ecto.Changeset.change(changeset,
          hashed_password: Pbkdf2.hash_pwd_salt(password, @hash_opts),
          recovery_hash: recovery_hash
        )

      Repo.transaction(fn ->
        case Repo.insert(changeset) do
          {:ok, user} ->
            {session, _} = insert_token(user.id, "session", @session_seconds)
            {scrub(user), session, code}

          {:error, changeset} ->
            Repo.rollback(changeset)
        end
      end)
    else
      {:error, %{changeset | action: :insert}}
    end
  end

  def sign_in(username, password) do
    user = user_by_username(username)

    if valid_password?(user, password) do
      Repo.transaction(fn ->
        current = Repo.one!(from u in User, where: u.id == ^user.id, lock: "FOR UPDATE")
        # A reset that raced password verification must invalidate that verification.
        if current.hashed_password != user.hashed_password,
          do: Repo.rollback(:invalid_credentials)

        {session, _} = insert_token(current.id, "session", @session_seconds)
        {scrub(current), session}
      end)
    else
      {:error, :invalid_credentials}
    end
  end

  def recover(username, code, attrs) do
    changeset = User.password_changeset(%User{}, attrs)

    with true <- changeset.valid?,
         %User{} = user <- user_by_username(username),
         {:ok, hash} <- token_hash(code) do
      result =
        Repo.transaction(fn ->
          user = Repo.one!(from u in User, where: u.id == ^user.id, lock: "FOR UPDATE")

          if is_nil(user.recovery_hash) or !Plug.Crypto.secure_compare(hash, user.recovery_hash),
            do: Repo.rollback(:invalid_recovery)

          password = Ecto.Changeset.get_change(changeset, :password)
          {new_code, recovery_hash} = recovery_code()

          user =
            user
            |> Ecto.Changeset.change(
              hashed_password: Pbkdf2.hash_pwd_salt(password, @hash_opts),
              recovery_hash: recovery_hash
            )
            |> Repo.update!()

          sessions =
            Repo.all(from t in Token, where: t.user_id == ^user.id and t.context == "session")

          Repo.delete_all(from t in Token, where: t.user_id == ^user.id)
          {session, _} = insert_token(user.id, "session", @session_seconds)
          {scrub(user), session, new_code, sessions}
        end)

      case result do
        {:ok, {user, session, new_code, sessions}} ->
          Enum.each(
            sessions,
            &ScripguessrWeb.Endpoint.broadcast(topic_for_hash(&1.token), "disconnect", %{})
          )

          {:ok, {user, session, new_code}}

        error ->
          error
      end
    else
      false -> {:error, %{changeset | action: :update}}
      _ -> {:error, :invalid_recovery}
    end
  end

  def user_for_session(raw) do
    with {:ok, hash} <- token_hash(raw),
         %User{hashed_password: password} = user when not is_nil(password) <-
           Repo.one(
             from t in valid_tokens(hash, "session"),
               join: u in User,
               on: u.id == t.user_id,
               select: u
           ) do
      scrub(user)
    else
      _ -> nil
    end
  end

  defp user_by_username(username) when is_binary(username) and byte_size(username) <= 32,
    do: Repo.get_by(User, username_normalized: String.downcase(String.trim(username)))

  defp user_by_username(_), do: nil

  defp valid_password?(%User{hashed_password: hash}, password)
       when is_binary(hash) and is_binary(password) and byte_size(password) in 1..512,
       do: Pbkdf2.verify_pass(password, hash)

  defp valid_password?(_, _) do
    Pbkdf2.no_user_verify(@hash_opts)
    false
  end

  defp scrub(user), do: %{user | password: nil, password_confirmation: nil}

  defp recovery_code do
    raw = :crypto.strong_rand_bytes(32) |> Base.url_encode64(padding: false)
    {:ok, hash} = token_hash(raw)
    {raw, hash}
  end

  def revoke_session(raw) do
    with {:ok, hash} <- token_hash(raw) do
      Repo.delete_all(from t in Token, where: t.token == ^hash and t.context == "session")
      ScripguessrWeb.Endpoint.broadcast(session_topic(raw), "disconnect", %{})
    end

    :ok
  end

  def session_topic(raw) do
    {:ok, hash} = token_hash(raw)
    topic_for_hash(hash)
  end

  defp topic_for_hash(hash), do: "account_sessions:" <> Base.url_encode64(hash)

  defp insert_token(user_id, context, seconds) do
    raw = :crypto.strong_rand_bytes(32) |> Base.url_encode64(padding: false)
    {:ok, hash} = token_hash(raw)

    token =
      Repo.insert!(%Token{
        user_id: user_id,
        token: hash,
        context: context,
        expires_at: DateTime.add(DateTime.utc_now(), seconds, :second)
      })

    {raw, token}
  end

  defp token_hash(raw) when is_binary(raw) and byte_size(raw) == 43 do
    case Base.url_decode64(raw, padding: false) do
      {:ok, bytes} when byte_size(bytes) == 32 -> {:ok, :crypto.hash(:sha256, bytes)}
      _ -> :error
    end
  end

  defp token_hash(_), do: :error

  defp valid_tokens(hash, context) do
    now = DateTime.utc_now()
    from t in Token, where: t.token == ^hash and t.context == ^context and t.expires_at > ^now
  end
end
