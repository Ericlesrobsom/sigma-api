# sigma-api

API em **Rust** para operar a **Sigma Reseller API v1** de ponta a ponta:
perfil, clientes (criar, editar, renovar, trocar pacote/conexoes, cortesia,
resync, migracao), pacotes, servidores e relatorios.

O painel Sigma publica um contrato oficial (OpenAPI 3.1.0, 39 paths / 51
operacoes). Este projeto **nao faz engenharia reversa**: consome o contrato
oficial com Bearer token.

## Pacotes

| Pacote | Binario | O que e |
|---|---|---|
| `sigma-core` | — | Cliente HTTP tipado, modelos, erros e rate limit |
| `sigma-server` | `sigma-serve` | API REST local em JSON, documentada em OpenAPI |
| `sigma-cli` | `sigma` | CLI para operacao manual e diagnostico |

## Inicio rapido

Pre-requisitos: Rust stable (edition 2021) e um token de revenda.

```bash
cp .env.example .env
# edite SIGMA_API_TOKEN (token do painel) e SIGMA_SERVER_TOKEN (token local)
cargo run -p sigma-server --bin sigma-serve
```

O servidor sobe em `http://127.0.0.1:8787`.

```bash
curl -s http://127.0.0.1:8787/health

curl -s -H "Authorization: Bearer $SIGMA_SERVER_TOKEN" \
  http://127.0.0.1:8787/v1/me
```

CLI (fala direto com o painel, sem passar pelo servidor local):

```bash
cargo run -p sigma-cli -- me
cargo run -p sigma-cli -- customers list --page 1 --per-page 20
cargo run -p sigma-cli -- packages
cargo run -p sigma-cli -- reports customers-count
```

## Endpoints do servidor

Todas as rotas `/v1/*` exigem `Authorization: Bearer $SIGMA_SERVER_TOKEN`.
O token do painel **nunca** sai do servidor.

| Metodo | Rota | Funcao |
|---|---|---|
| GET | `/health` | health check publico |
| GET | `/openapi.json` | contrato OpenAPI |
| GET | `/v1/me` | perfil da conta |
| GET/POST | `/v1/customers` | lista/cria clientes |
| GET/PUT/DELETE | `/v1/customers/{id}` | detalhe/edita/remove |
| POST | `/v1/customers/{id}/renew` | renova |
| PUT | `/v1/customers/{id}/package` | troca pacote |
| PUT | `/v1/customers/{id}/connections` | troca conexoes |
| POST | `/v1/customers/{id}/courtesy-days` | dias de cortesia |
| POST | `/v1/customers/{id}/resync` | ressincroniza |
| PUT | `/v1/customers/{id}/server-migration` | migra de servidor |
| GET | `/v1/packages` | pacotes |
| GET | `/v1/servers` | servidores |
| GET | `/v1/reports/{kind}` | relatorios oficiais |

Erros seguem `{ "ok": false, "error": { "code", "message" } }` e preservam
`Retry-After` quando o painel responde `429`.

## Configuracao

| Variavel | Obrigatoria | Descricao |
|---|---|---|
| `SIGMA_API_BASE_URL` | nao | default `https://espaceoneplay.sigmab.pro/api/reseller-api/v1` |
| `SIGMA_API_TOKEN` | sim | token de revenda (Bearer) |
| `SIGMA_SERVER_TOKEN` | sim | token local exigido nas rotas `/v1/*` |
| `SIGMA_BIND` | nao | default `127.0.0.1:8787` |
| `SIGMA_TIMEOUT_SECS` | nao | default `30` |

## Seguranca

- Tokens apenas em variaveis de ambiente; `.env` e ignorado.
- O servidor nao expoe o token do painel em logs nem em respostas.
- Rate limit oficial: 120 req/min por usuario/tenant; `429` + `Retry-After`.
- Webhooks (`/me/webhooks*`) exigem sessao do painel e **nao** sao expostos
  por este servico; configure no painel.
- Escopos do token devem ser os minimos necessarios.

## Desenvolvimento

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
python scripts/openapi_to_json.py
```

O CI roda os quatro passos. O workflow de Docker publica no GHCR em tags `v*`.

## Docker

```bash
docker build -t sigma-api .
docker run --rm -p 8787:8787 \
  -e SIGMA_API_TOKEN -e SIGMA_SERVER_TOKEN \
  sigma-api
```

## Licenca

Proprietario. Nao redistribua tokens, dados de clientes ou o bundle do painel.
