create table if not exists users (
    id uuid primary key,
    username text not null unique,
    email text not null unique,
    password_hash text not null,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now()
);

create index if not exists idx_users_email on users (email);
create index if not exists idx_users_username on users (username);
