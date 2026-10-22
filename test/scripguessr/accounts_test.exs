defmodule Scripguessr.AccountsTest do
  use Scripguessr.DataCase
  import Scripguessr.AccountFixtures
  alias Scripguessr.{Accounts, Repo}
  alias Scripguessr.Accounts.{User, Token}

  test "registration hashes passwords and stores only recovery and session digests" do
    {:ok, {user, session, code}} = Accounts.register(credentials(" Reader "))
    assert user.username == "Reader"
    assert user.username_normalized == "reader"
    assert is_nil(user.password)
    assert user.hashed_password =~ "$pbkdf2-sha256$600000$"
    assert Pbkdf2.verify_pass(password(), user.hashed_password)
    refute inspect(user) =~ user.hashed_password
    assert user.recovery_hash == :crypto.hash(:sha256, Base.url_decode64!(code, padding: false))

    assert Repo.one!(Token).token ==
             :crypto.hash(:sha256, Base.url_decode64!(session, padding: false))

    assert Accounts.user_for_session(session).id == user.id
    assert is_nil(Accounts.user_for_session(code))
    assert {:error, %Ecto.Changeset{}} = Accounts.register(credentials("READER"))
    assert Repo.aggregate(User, :count) == 1
  end

  test "verification is case insensitive and failed sign-in does not create a session" do
    {user, _} = account("Reader")
    assert {:ok, {same_user, session}} = Accounts.sign_in("READER", password())
    assert same_user.id == user.id
    assert Accounts.user_for_session(session)

    for {name, pass} <- [
          {"Reader", "wrong"},
          {"missing", password()},
          {nil, nil},
          {%{}, %{}},
          {"Reader", String.duplicate("a", 1000)}
        ] do
      assert {:error, :invalid_credentials} = Accounts.sign_in(name, pass)
    end

    assert Repo.aggregate(Token, :count) == 2
  end

  test "registration requires a valid username, a long password and matching confirmation" do
    for attrs <- [
          credentials("a"),
          credentials("bad name"),
          credentials("a@b.com"),
          credentials("reader", "short"),
          credentials("reader", "1234567"),
          %{username: "reader", password: password()},
          Map.put(credentials("reader"), :password_confirmation, "different"),
          credentials("reader", String.duplicate("a", 129)),
          credentials("reader", String.duplicate("a" <> String.duplicate("\u0301", 30), 15))
        ] do
      assert {:error, %Ecto.Changeset{}} = Accounts.register(attrs)
    end

    assert Repo.aggregate(User, :count) == 0
  end

  test "eight-character passwords work for registration, sign-in and recovery" do
    {:ok, {user, _, code}} = Accounts.register(credentials("reader", "12345678"))
    assert {:ok, {signed_in, _}} = Accounts.sign_in("reader", "12345678")
    assert signed_in.id == user.id

    assert {:error, %Ecto.Changeset{}} =
             Accounts.recover("reader", code, credentials("reader", "1234567"))

    assert {:ok, _} = Accounts.recover("reader", code, credentials("reader", "87654321"))
    assert {:ok, _} = Accounts.sign_in("reader", "87654321")
  end

  test "expired and revoked sessions cannot be used" do
    {user, first} = account("reader")
    {:ok, {_, second}} = Accounts.sign_in("reader", password())
    ScripguessrWeb.Endpoint.subscribe(Accounts.session_topic(first))
    Accounts.revoke_session(first)
    assert_receive %Phoenix.Socket.Broadcast{event: "disconnect"}
    assert is_nil(Accounts.user_for_session(first))
    assert Accounts.user_for_session(second).id == user.id
    Repo.update_all(Token, set: [expires_at: DateTime.add(DateTime.utc_now(), -1)])
    assert is_nil(Accounts.user_for_session(second))

    for token <- [nil, "", "invalid", %{}, 123],
        do: assert(is_nil(Accounts.user_for_session(token)))
  end

  test "recovery is single-use, replaces the password, and revokes existing sessions" do
    {:ok, {user, first, code}} = Accounts.register(credentials("reader"))
    {:ok, {_, second}} = Accounts.sign_in("reader", password())
    ScripguessrWeb.Endpoint.subscribe(Accounts.session_topic(first))

    assert {:ok, {recovered, session, replacement}} =
             Accounts.recover(
               "READER",
               code,
               credentials("ignored", "a completely different password")
             )

    assert recovered.id == user.id
    refute code == replacement
    assert_receive %Phoenix.Socket.Broadcast{event: "disconnect"}
    assert is_nil(Accounts.user_for_session(first))
    assert is_nil(Accounts.user_for_session(second))
    assert Accounts.user_for_session(session).id == user.id
    assert {:error, :invalid_credentials} = Accounts.sign_in("reader", password())
    assert {:ok, _} = Accounts.sign_in("reader", "a completely different password")
    assert {:error, :invalid_recovery} = Accounts.recover("reader", code, credentials("reader"))
    assert {:ok, _} = Accounts.recover("reader", replacement, credentials("reader"))
  end

  test "invalid recovery attempts leave credentials and sessions unchanged" do
    {:ok, {user, session, code}} = Accounts.register(credentials("reader"))

    for raw <- [
          nil,
          %{},
          "invalid",
          Base.url_encode64(:crypto.strong_rand_bytes(32), padding: false)
        ] do
      assert {:error, :invalid_recovery} = Accounts.recover("reader", raw, credentials("reader"))
    end

    assert {:error, %Ecto.Changeset{}} =
             Accounts.recover("reader", code, credentials("reader", "short"))

    assert Accounts.user_for_session(session).id == user.id
    assert Repo.get!(User, user.id).recovery_hash == user.recovery_hash
    assert {:ok, _} = Accounts.sign_in("reader", password())
  end

  test "authentication rate limiting is bounded and independent across clients" do
    for _ <- 1..10, do: assert(Accounts.RateLimit.allow?({127, 0, 0, 1}))
    refute Accounts.RateLimit.allow?({127, 0, 0, 1})
    assert Accounts.RateLimit.allow?({127, 0, 0, 2})
  end
end
