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
| POST | `/v1/customers` | cria cliente (JSON: `server_id`, `package_id`, ...) |
| GET | `/v1/customers/by-username/{username}` | busca por username |
| GET | `/v1/customers/{id}` | detalhe do cliente |
| PUT | `/v1/customers/{id}` | edita campos graváveis (merge com o atual) |
| DELETE | `/v1/customers/{id}` | remove (soft delete) |
| POST | `/v1/customers/{id}/renew` | renova (plano/conexões/referência) |
| POST | `/v1/customers/{id}/change-package` | troca o plano |
| POST | `/v1/customers/{id}/change-connections` | troca conexões |
| PUT | `/v1/customers/{id}/toggle-status` | alterna Ativo/Inativo |
| POST | `/v1/customers/{id}/courtesy-days` | +3 dias de cortesia (vence em ≤3 dias) |
| PUT | `/v1/customers/{id}/server-migration` | migra servidor/plano |
| PUT | `/v1/customers/{id}/asn-lock` | alterna trava ASN |
| PUT | `/v1/customers/{id}/isp-lock` | alterna trava ISP |
| GET | `/v1/resellers` | revendas da conta |
| POST | `/v1/resellers` | cria revenda |
| GET/PUT/DELETE | `/v1/resellers/{id}` | detalhe/edita/exclui revenda |
| POST | `/v1/resellers/{id}/credits` | adiciona créditos |
| POST | `/v1/resellers/{id}/credits/remove` | remove créditos |
| PUT | `/v1/resellers/{id}/toggle-status` | alterna Ativo/Inativo da revenda |
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
| GET | `/v1/live-connections/{server_id}` | conexões ao vivo (tempo online, dispositivo, stream) |
| GET | `/v1/chatbot/link/{server_id}` | link público do BotBot |
| POST | `/v1/chatbot/tests` | cria teste via BotBot sem login |

Hoje o serviço cobre **leitura e escrita** dos fluxos mapeados: criar/editar/
renovar/trocar plano/conexões/status/excluir cliente, criar revenda e
adicionar créditos. O `PUT /v1/customers/{id}` aceita patch e faz o merge
com o objeto completo que o painel exige. Criar revenda preenche os defaults
observados (mínimo 10 créditos).

### Conexões ao vivo

`GET /v1/live-connections/{server_id}` devolve as linhas do painel com:
`user_username`, `active_connections`, `max_connections`, `reseller_username`,
`stream_display_name`, `user_agent` (dispositivo), `date_start_timestamp`
(tempo online = agora − timestamp), `country_name` e `quality`. O `meta` traz
`audience.countries` e `unknown_connections`.

### BotBot (teste sem login)

O painel publica um link que cria teste **sem sessão**:

```
POST https://<painel>/api/chatbot/<user_id>/<server_id>
```

`GET /v1/chatbot/link/{server_id}` devolve esse link; `POST /v1/chatbot/tests`
com `{server_id}` executa o POST e devolve `username`, `password`, `dns`,
`connections`, `expiresAt` e `payUrl`. Cada chamada cria um teste real — use
com moderação.

## Configuracao

| Variavel | Obrigatoria | Descricao |
|---|---|---|
| `SIGMA_PANEL_URL` | nao | default `https://espaceoneplay.sigmab.pro` |
| `SIGMA_USER` | sim | usuario/e-mail da revenda |
| `SIGMA_PASS` | sim | senha da revenda |
| `SIGMA_SERVER_TOKEN` | sim | token local exigido nas rotas `/v1/*` |
| `SIGMA_BIND` | nao | default `127.0.0.1:8787` |
| `SIGMA_TIMEOUT_SECS` | nao | default `120` |

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
docker run -d --name sigma-api -p 8787:8787 \
  -e SIGMA_USER='seu-usuario' \
  -e SIGMA_PASS='sua-senha' \
  -e SIGMA_SERVER_TOKEN='token-local-longo' \
  ghcr.io/ericlesrobsom/sigma-api:0.1.1
```

O Docker baixa a imagem sozinho se ela não existir. Depois:

```bash
curl -H "Authorization: Bearer token-local-longo" http://127.0.0.1:8787/v1/me
```

Se o pacote do GHCR estiver privado, rode `docker login ghcr.io` uma vez
(com um PAT com `read:packages`) antes.

## Licenca

MIT — veja [`LICENSE`](LICENSE). Você pode baixar, executar, modificar e
redistribuir, mantendo o aviso de copyright. Credenciais e tokens não fazem
parte do código: use variáveis de ambiente.
