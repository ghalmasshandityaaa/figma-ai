alter table users
    add column if not exists is_active boolean not null default true;

create index if not exists idx_users_is_active on users (is_active);
