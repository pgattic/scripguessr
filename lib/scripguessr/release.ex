defmodule Scripguessr.Release do
  def migrate do
    Application.load(:scripguessr)

    for repo <- Application.fetch_env!(:scripguessr, :ecto_repos) do
      {:ok, _, _} = Ecto.Migrator.with_repo(repo, &Ecto.Migrator.run(&1, :up, all: true))
    end
  end
end
