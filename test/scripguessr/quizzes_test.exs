defmodule Scripguessr.QuizzesTest do
  use Scripguessr.DataCase
  alias Scripguessr.{Quizzes, Repo}
  alias Scripguessr.Quizzes.Quiz

  test "persists prepared verses and progress independently of upstream availability" do
    assert {:ok, quiz} = Quizzes.create("owner")
    Scripguessr.Corpus.Fake.reset(:down)
    Scripguessr.Corpus.Cache.clear()
    round = hd(quiz.snapshot["rounds"])

    assert {:ok, guessed} =
             Quizzes.submit(quiz.id, "owner", 0, round["book"], to_string(round["chapter"]))

    assert {:ok, repeated} = Quizzes.submit(quiz.id, "owner", 0, "2 Nephi", "33")
    assert repeated.snapshot == guessed.snapshot
    assert {:ok, _} = Quizzes.advance(quiz.id, "owner", 0)
    assert {:ok, resumed} = Quizzes.fetch(quiz.id, "owner")
    assert resumed.snapshot["rounds"] == guessed.snapshot["rounds"]
    assert resumed.snapshot["current"] == 1
    assert {:error, :unavailable} = Quizzes.create("owner")
    assert Repo.aggregate(Quiz, :count) == 1
  end

  test "another browser cannot read, guess, or advance a quiz" do
    {:ok, quiz} = Quizzes.create("owner")
    assert {:error, :not_found} = Quizzes.fetch(quiz.id, "stranger")
    assert {:error, :not_found} = Quizzes.submit(quiz.id, "stranger", 0, "1 Nephi", "1")
    assert {:error, :not_found} = Quizzes.advance(quiz.id, "stranger", 0)
    assert {:error, :not_found} = Quizzes.fetch("invalid", "owner")
  end
end
