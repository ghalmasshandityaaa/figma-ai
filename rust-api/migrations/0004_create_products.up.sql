create table if not exists products (
    id uuid primary key,
    owner_id uuid not null references users(id),
    name text not null,
    description text,
    price_cents bigint not null,
    currency text not null,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now()
);

create index if not exists idx_products_owner_id on products (owner_id);
