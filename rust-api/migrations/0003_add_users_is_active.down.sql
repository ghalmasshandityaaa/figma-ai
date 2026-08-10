drop index if exists idx_users_is_active;
alter table users drop column if exists is_active;
