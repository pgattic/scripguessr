defmodule Scripguessr.AccountFixtures do
  def password, do: "a correct horse battery staple"

  def credentials(username, password \\ password()) do
    %{username: username, password: password, password_confirmation: password}
  end

  def account(username) do
    {:ok, {user, session, _code}} = Scripguessr.Accounts.register(credentials(username))
    {user, session}
  end
end
