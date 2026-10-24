defmodule Scripguessr.Application do
  use Application

  def start(_type, _args) do
    children = [
      Scripguessr.Repo,
      Scripguessr.Accounts.RateLimit,
      {Phoenix.PubSub, name: Scripguessr.PubSub},
      ScripguessrWeb.MatchPresence,
      {Task.Supervisor, name: Scripguessr.Corpus.Tasks},
      Scripguessr.Corpus.Cache,
      Scripguessr.Corpus.Chapter,
      Scripguessr.Atlas.Cache,
      ScripguessrWeb.Endpoint
    ]

    children =
      if Application.get_env(:scripguessr, :match_clock, true),
        do: children ++ [Scripguessr.Matches.Clock],
        else: children

    Supervisor.start_link(children, strategy: :one_for_one, name: Scripguessr.Supervisor)
  end

  def config_change(changed, _new, removed),
    do: ScripguessrWeb.Endpoint.config_change(changed, removed)
end
