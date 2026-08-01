create table if not exists outbox_events (
    id uuid primary key,
    aggregate_id uuid not null,
    event_type text not null,
    payload jsonb not null,
    created_at timestamptz not null default now(),
    processed_at timestamptz
);

create index if not exists idx_outbox_unprocessed
    on outbox_events (created_at)
    where processed_at is null;
