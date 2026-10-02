CREATE TABLE users (
    id uuid PRIMARY KEY,
    username text NOT NULL,
    username_normalized text NOT NULL UNIQUE,
    password_hash text NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE games (
    id uuid PRIMARY KEY,
    user_id uuid REFERENCES users(id) ON DELETE SET NULL,
    game_kind text NOT NULL,
    leaderboard_preset text,
    difficulty jsonb NOT NULL,
    scope jsonb NOT NULL,
    playable_verse_count integer NOT NULL,
    round_count integer NOT NULL,
    current_round_index integer NOT NULL DEFAULT 0,
    finished boolean NOT NULL DEFAULT false,
    score integer,
    possible_score integer,
    created_at timestamptz NOT NULL DEFAULT now(),
    last_seen_at timestamptz NOT NULL DEFAULT now(),
    completed_at timestamptz
);

CREATE INDEX games_user_completed_idx
    ON games (user_id, completed_at DESC)
    WHERE finished;
CREATE INDEX games_unfinished_last_seen_idx
    ON games (last_seen_at)
    WHERE NOT finished;

CREATE TABLE game_rounds (
    game_id uuid NOT NULL REFERENCES games(id) ON DELETE CASCADE,
    round_index integer NOT NULL,
    text text NOT NULL,
    passage jsonb NOT NULL,
    source_passage jsonb NOT NULL,
    guess jsonb,
    PRIMARY KEY (game_id, round_index)
);

CREATE TABLE review_items (
    id uuid PRIMARY KEY,
    user_id uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    passage jsonb NOT NULL,
    text text NOT NULL,
    score integer NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX review_items_user_idx ON review_items (user_id, created_at);

CREATE TABLE custom_study_sets (
    id uuid PRIMARY KEY,
    user_id uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name text NOT NULL,
    passages jsonb NOT NULL,
    guess_scope jsonb NOT NULL,
    prompt_policy jsonb NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX custom_study_sets_user_idx ON custom_study_sets (user_id, updated_at DESC);
