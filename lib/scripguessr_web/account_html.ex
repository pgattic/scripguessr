defmodule ScripguessrWeb.AccountHTML do
  use ScripguessrWeb, :html
  import ScripguessrWeb.Layouts, only: [personal_nav: 1]
  import ScripguessrWeb.ProgressionComponents
  alias Scripguessr.Games.Rules
  defp form_action(:sign_in), do: "/account/sign-in"
  defp form_action(:register), do: "/account/register"
  defp form_action(:recover), do: "/account/recover"
  defp form_id(:sign_in), do: "sign-in-form"
  defp form_id(:register), do: "registration-form"
  defp form_id(:recover), do: "recovery-form"
  embed_templates "account_html/*"
end
