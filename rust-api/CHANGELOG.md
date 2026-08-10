# Changelog

Auto-generated from Conventional Commits history. Do not hand-edit — regenerate instead.

## Features
- **api**: use layer-prefixed span names, instrument password hasher (`71f364d`)
- **api**: show METHOD /path as the trace name in Tempo/Grafana (`e5edf7a`)
- **api**: instrument use-case and repository layers with per-span start/end logs (`eb5e613`)
- **api**: support local cargo-run via dotenvy and document docker dev workflow (`6b133de`)
- **api**: scaffold rust-api with axum, postgres, paseto, and observability (`3c119ab`)

## Fixes
- **grafana**: repair broken traces dashboard (`f189a6b`)

## Refactoring
- **api**: restructure into per-domain feature modules (`a88c7cc`)
- **api**: group handlers and dto by domain module (`675a1d4`)
- **api**: rename db.* spans to repository.* for terminology consistency (`d6fdd15`)

## Documentation
- **env**: add section dividers to .env.example (`5166799`)

## Tests
- **api**: add xh examples.sh covering every endpoint (`173e2b8`)
