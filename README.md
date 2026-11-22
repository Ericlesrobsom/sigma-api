# sigma-api

API em **Rust** que **loga no painel Sigma como revenda** (usuário/senha +
CAP) e expõe os dados disponíveis para a conta. Não precisa de token de API
administrativo — o token de revenda é criado pela própria sessão.

## Por que login e não API oficial

A Sigma Reseller API oficial (Bearer token) é liberada pelo administrador.
Uma revenda só tem usuário e senha do painel. Este serviço faz o mesmo
caminho do navegador:

1. `POST /api/cap/challenge` → desafio proof-of-work;
2. resolve o CAP (FNV-1a + PRNG + SHA-256, porte do `capjs-core`);
3. `POST /api/cap/redeem` → token do CAP;
4. `POST /api/auth/login` com `X-Cap-Capable: 1` → token de sessão;
5. usa `Authorization: Bearer <sessão>` em `/api/*`;
6. em `401`, refaz o login sozinho.

## Pacotes

| Pacote | Binario | O que e |
|---|---|---|
| `sigma-core` | — | Login, CAP, sessão e cliente HTTP do painel |
| `sigma-server` | `sigma-serve` | API REST local em JSON, documentada em OpenAPI |
| `sigma-cli` | `sigma` | CLI para operacao manual e diagnostico |

## Inicio rapido

```bash
cp .env.example .env
# edite SIGMA_USER, SIGMA_PASS e SIGMA_SERVER_TOKEN
cargo run -p sigma-server --bin sigma-serve
```

```bash
curl -s http://127.0.0.1:8787/health

curl -s -H "Authorization: Bearer $SIGMA_SERVER_TOKEN" \
  http://127.0.0.1:8787/v1/me
```

CLI (fala direto com o painel, fazendo login proprio):

```bash
cargo run -p sigma-cli -- me
cargo run -p sigma-cli -- customers list --per-page 20
cargo run -p sigma-cli -- packages
cargo run -p sigma-cli -- credit-summary
```

## Endpoints do servidor

Todas as rotas `/v1/*` exigem `Authorization: Bearer $SIGMA_SERVER_TOKEN`.
A senha do painel **nunca** sai do servidor.

| Metodo | Rota | Funcao |
|---|---|---|
| GET | `/health` | health check publico |
| GET | `/openapi.json` | contrato OpenAPI |
| GET | `/v1/me` | perfil, creditos e permissoes |
| GET | `/v1/customers` | clientes (`page`, `perPage`, `username`, `status`, `isTrial`) |
| GET | `/v1/customers/by-username/{username}` | busca por username |
| GET | `/v1/customers/{id}` | detalhe do cliente |
| GET | `/v1/packages` | planos/pacotes |
| GET | `/v1/servers` | servidores |
| GET | `/v1/credit-orders` | pedidos de credito |
| GET | `/v1/credit-transactions` | extrato (`page`) |
| GET | `/v1/credit-summary` | resumo do extrato |
| GET | `/v1/resellers` | revendas da conta |
| GET | `/v1/customers-count` | contagem de clientes |
| GET | `/v1/customers-count-by-server` | contagem por servidor |
| GET | `/v1/expiring` | clientes a vencer |
| GET | `/v1/notices` | avisos do painel |

Hoje o servico e **somente leitura**. Criar/editar/renovar cliente exige
descobrir os endpoints de escrita do painel (o fluxo da UI ainda nao foi
capturado); nao inventamos rota nem body.

## Configuracao

| Variavel | Obrigatoria | Descricao |
|---|---|---|
| `SIGMA_PANEL_URL` | nao | default `https://espaceoneplay.sigmab.pro` |
| `SIGMA_USER` | sim | usuario/e-mail da revenda |
| `SIGMA_PASS` | sim | senha da revenda |
| `SIGMA_SERVER_TOKEN` | sim | token local exigido nas rotas `/v1/*` |
| `SIGMA_BIND` | nao | default `127.0.0.1:8787` |
| `SIGMA_TIMEOUT_SECS` | nao | default `30` |

## Limites conhecidos

- O painel limita tentativas de login (429). O servidor loga **uma vez** e
  reusa a sessao; nao reinicie em loop.
- A conta `revenda-pos` so enxerga os proprios clientes/revendas; 403/404
  do painel sao repassados.
- CAP e obrigatorio: sem proof-of-work o login retorna
  `422 Please complete the human verification`.
- Webhooks e configuracoes globais nao existem para revenda.

## Desenvolvimento

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
python scripts/openapi_to_json.py
```

Testes: CAP com vetor real (80/80), login com mock, servidor com sessao
mockada. Em producao, valide primeiro `GET /v1/me`.

## Docker

```bash
docker build -t sigma-api .
docker run --rm -p 8787:8787 \
  -e SIGMA_USER -e SIGMA_PASS -e SIGMA_SERVER_TOKEN \
  sigma-api
```
