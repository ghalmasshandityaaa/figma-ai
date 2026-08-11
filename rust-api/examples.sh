#!/usr/bin/env bash
# xh examples exercising every rust-api endpoint, happy-path + edge cases.
#
# Prereqs:
#   cargo install xh
#   docker compose up -d postgres tempo grafana && cargo run -p api
#   (or: docker compose up --build -d)
#
# Usage:
#   ./examples.sh                    # run everything against http://localhost:8080
#   BASE=http://localhost:8080 ./examples.sh
set -euo pipefail

BASE="${BASE:-http://localhost:8080}"
step() { printf '\n\033[1;36m### %s\033[0m\n' "$1"; }

# -----------------------------------------------------------------------------
# Health
# -----------------------------------------------------------------------------
step "GET /healthz — liveness probe"
xh --ignore-stdin GET "$BASE/healthz"

# -----------------------------------------------------------------------------
# Register
# -----------------------------------------------------------------------------
step "POST /api/v1/users — register a new user"
SUFFIX="$(date +%s)"
EMAIL="alice-$SUFFIX@example.com"
REGISTER_JSON=$(xh --ignore-stdin POST "$BASE/api/v1/users" \
  username="alice$SUFFIX" \
  email="$EMAIL" \
  password=S3cretPass)
echo "$REGISTER_JSON"
USER_ID=$(echo "$REGISTER_JSON" | jq -r '.data.user_id')

step "POST /api/v1/users — 422 validation error (bad email, short password)"
xh --ignore-stdin --check-status POST "$BASE/api/v1/users" \
  username=a \
  email=not-an-email \
  password=short \
  || true

step "POST /api/v1/users — 409 conflict (duplicate email)"
xh --ignore-stdin --check-status POST "$BASE/api/v1/users" \
  username=alice02 \
  email="$EMAIL" \
  password=S3cretPass \
  || true

# -----------------------------------------------------------------------------
# Login
# -----------------------------------------------------------------------------
step "POST /api/v1/auth/login — issue a PASETO token"
LOGIN_JSON=$(xh --ignore-stdin POST "$BASE/api/v1/auth/login" \
  email="$EMAIL" \
  password=S3cretPass)
echo "$LOGIN_JSON"
ACCESS_TOKEN=$(echo "$LOGIN_JSON" | jq -r '.data.access_token')

step "POST /api/v1/auth/login — 401 wrong password"
xh --ignore-stdin --check-status POST "$BASE/api/v1/auth/login" \
  email="$EMAIL" \
  password=WrongPass1 \
  || true

# -----------------------------------------------------------------------------
# Profile
# -----------------------------------------------------------------------------
step "GET /api/v1/users/:id — fetch profile"
xh --ignore-stdin GET "$BASE/api/v1/users/$USER_ID" \
  "Authorization:Bearer $ACCESS_TOKEN"

step "GET /api/v1/users/:id — 404 (random uuid)"
xh --ignore-stdin --check-status GET "$BASE/api/v1/users/00000000-0000-0000-0000-000000000000" || true

step "GET /api/v1/users/:id — 422 (malformed uuid)"
xh --ignore-stdin --check-status GET "$BASE/api/v1/users/not-a-uuid" || true

# -----------------------------------------------------------------------------
# Activate / deactivate
# -----------------------------------------------------------------------------
step "POST /api/v1/users/:id/deactivate — suspend the account"
xh --ignore-stdin POST "$BASE/api/v1/users/$USER_ID/deactivate" \
  "Authorization:Bearer $ACCESS_TOKEN"

step "POST /api/v1/auth/login — 403 after deactivation"
xh --ignore-stdin --check-status POST "$BASE/api/v1/auth/login" \
  email="$EMAIL" \
  password=S3cretPass \
  || true

step "POST /api/v1/users/:id/deactivate — 409 already inactive"
xh --ignore-stdin --check-status POST "$BASE/api/v1/users/$USER_ID/deactivate" \
  "Authorization:Bearer $ACCESS_TOKEN" \
  || true

step "POST /api/v1/users/:id/activate — reinstate the account"
xh --ignore-stdin POST "$BASE/api/v1/users/$USER_ID/activate" \
  "Authorization:Bearer $ACCESS_TOKEN"

step "POST /api/v1/users/:id/activate — 409 already active"
xh --ignore-stdin --check-status POST "$BASE/api/v1/users/$USER_ID/activate" \
  "Authorization:Bearer $ACCESS_TOKEN" \
  || true

step "POST /api/v1/users/:id/activate — 422 (malformed uuid)"
xh --ignore-stdin --check-status POST "$BASE/api/v1/users/not-a-uuid/activate" || true

step "POST /api/v1/users/:id/deactivate — 404 (random uuid)"
xh --ignore-stdin --check-status POST "$BASE/api/v1/users/00000000-0000-0000-0000-000000000000/deactivate" || true

# -----------------------------------------------------------------------------
# List users (pagination)
# -----------------------------------------------------------------------------
step "GET /api/v1/users — default pagination"
xh --ignore-stdin GET "$BASE/api/v1/users"

step "GET /api/v1/users?page=1&page_size=5 — explicit pagination"
xh --ignore-stdin GET "$BASE/api/v1/users" page==1 page_size==5

# -----------------------------------------------------------------------------
# Products
# -----------------------------------------------------------------------------
step "POST /api/v1/products — 401 unauthenticated create (auth/missing-token)"
xh --ignore-stdin --check-status POST "$BASE/api/v1/products" \
  name="Unauthorized Widget" \
  price_cents:=999 \
  currency=USD \
  || true

step "POST /api/v1/products — create a product"
CREATE_PRODUCT_JSON=$(xh --ignore-stdin POST "$BASE/api/v1/products" \
  "Authorization:Bearer $ACCESS_TOKEN" \
  name="Vintage Turntable" \
  description="Belt-drive, fully serviced" \
  price_cents:=24999 \
  currency=USD)
echo "$CREATE_PRODUCT_JSON"
PRODUCT_ID=$(echo "$CREATE_PRODUCT_JSON" | jq -r '.data.id')

step "POST /api/v1/products — 422 validation error (empty name, negative price, bad currency)"
xh --ignore-stdin --check-status POST "$BASE/api/v1/products" \
  "Authorization:Bearer $ACCESS_TOKEN" \
  name="" \
  price_cents:=-100 \
  currency=usd \
  || true

step "GET /api/v1/products — list, paginated (public, no token)"
xh --ignore-stdin GET "$BASE/api/v1/products"

step "GET /api/v1/products/:id — fetch the created product (public, no token)"
xh --ignore-stdin GET "$BASE/api/v1/products/$PRODUCT_ID"

step "GET /api/v1/products/:id — 404 (random uuid)"
xh --ignore-stdin --check-status GET "$BASE/api/v1/products/00000000-0000-0000-0000-000000000000" || true

step "GET /api/v1/products/:id — 422 (malformed uuid)"
xh --ignore-stdin --check-status GET "$BASE/api/v1/products/not-a-uuid" || true

# -----------------------------------------------------------------------------
# CORS / not found
# -----------------------------------------------------------------------------
step "OPTIONS /api/v1/users — CORS preflight (disallowed if no origin configured)"
xh --ignore-stdin --check-status OPTIONS "$BASE/api/v1/users" \
  "Origin:http://localhost:5173" \
  "Access-Control-Request-Method:POST" \
  || true

step "GET /no-such-route — 404 route fallback"
xh --ignore-stdin --check-status GET "$BASE/no-such-route" || true

step "done"
