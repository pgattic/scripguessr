defmodule ScripguessrWeb.MatchPresence do
  use Phoenix.Presence, otp_app: :scripguessr, pubsub_server: Scripguessr.PubSub
end
