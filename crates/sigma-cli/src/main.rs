use anyhow::Context;
use clap::{Parser, Subcommand};
use serde::Serialize;
use serde_json::{json, Value};
use sigma_core::{CustomerQuery, SigmaClient, SigmaConfig};

#[derive(Parser)]
#[command(
    name = "sigma",
    version,
    about = "CLI da API do painel Sigma (login de revenda)"
)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Perfil da conta logada.
    Me,
    /// Clientes.
    Customers {
        #[command(subcommand)]
        cmd: CustomersCmd,
    },
    /// Pacotes/planos.
    Packages,
    /// Servidores.
    Servers,
    /// Pedidos de credito.
    CreditOrders,
    /// Extrato de creditos.
    CreditTransactions {
        #[arg(long, default_value_t = 1)]
        page: u32,
    },
    /// Resumo do extrato.
    CreditSummary,
    /// Revendas da conta.
    Resellers {
        #[command(subcommand)]
        cmd: ResellersCmd,
    },
    /// Contagem de clientes.
    CustomersCount,
    /// Contagem por servidor.
    CustomersCountByServer,
    /// Clientes a vencer.
    Expiring,
    /// Avisos do painel.
    Notices,
}

#[derive(Subcommand)]
enum CustomersCmd {
    List {
        #[arg(long)]
        page: Option<u32>,
        #[arg(long = "per-page")]
        per_page: Option<u32>,
        #[arg(long)]
        username: Option<String>,
        #[arg(long)]
        status: Option<String>,
        #[arg(long = "is-trial")]
        is_trial: Option<String>,
    },
    Get {
        id: String,
    },
    ByUsername {
        username: String,
    },
    /// Cria cliente com um JSON (server_id, package_id, username, password...).
    Create {
        json: String,
    },
    /// Atualiza campos graváveis com um JSON.
    Update {
        id: String,
        json: String,
    },
    /// Renova o cliente.
    Renew {
        id: String,
        #[arg(long = "package-id")]
        package_id: Option<String>,
        #[arg(long)]
        connections: Option<i64>,
    },
    /// Alterna Ativo/Inativo.
    Toggle {
        id: String,
    },
    /// Remove (soft delete) o cliente.
    Delete {
        id: String,
    },
}

#[derive(Subcommand)]
enum ResellersCmd {
    List,
    /// Cria revenda com um JSON (username, password, credits, name...).
    Create {
        json: String,
    },
    /// Adiciona créditos à revenda.
    AddCredits {
        id: String,
        credits: f64,
        #[arg(long, default_value = "")]
        reference: String,
        #[arg(long = "private-note", default_value = "")]
        private_note: String,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    let cli = Cli::parse();
    let client = SigmaClient::new(SigmaConfig::from_env()?).context("config")?;

    match cli.cmd {
        Cmd::Me => print(&client.me().await?)?,
        Cmd::Customers { cmd } => match cmd {
            CustomersCmd::List {
                page,
                per_page,
                username,
                status,
                is_trial,
            } => {
                let query = CustomerQuery {
                    page,
                    per_page,
                    username,
                    status,
                    is_trial,
                };
                print(&client.customers(&query).await?)?;
            }
            CustomersCmd::Get { id } => print(&client.customer(&id).await?)?,
            CustomersCmd::ByUsername { username } => {
                print(&client.customer_by_username(&username).await?)?
            }
            CustomersCmd::Create { json } => {
                let body: Value = serde_json::from_str(&json)?;
                print(&client.create_customer(&body).await?)?;
            }
            CustomersCmd::Update { id, json } => {
                let patch: Value = serde_json::from_str(&json)?;
                print(&client.update_customer(&id, &patch).await?)?;
            }
            CustomersCmd::Renew {
                id,
                package_id,
                connections,
            } => {
                let mut body = json!({
                    "reference": "",
                    "create_manual_customer_order": false,
                    "manual_payment_total": null
                });
                if let Some(p) = package_id {
                    body["package_id"] = json!(p);
                }
                if let Some(c) = connections {
                    body["connections"] = json!(c);
                }
                print(&client.renew_customer(&id, &body).await?)?;
            }
            CustomersCmd::Toggle { id } => print(&client.toggle_customer_status(&id).await?)?,
            CustomersCmd::Delete { id } => print(&client.delete_customer(&id).await?)?,
        },
        Cmd::Packages => print(&client.packages().await?)?,
        Cmd::Servers => print(&client.servers().await?)?,
        Cmd::CreditOrders => print(&client.credit_orders().await?)?,
        Cmd::CreditTransactions { page } => print(&client.credit_transactions(page).await?)?,
        Cmd::CreditSummary => print(&client.credit_summary().await?)?,
        Cmd::Resellers { cmd } => match cmd {
            ResellersCmd::List => print(&client.resellers().await?)?,
            ResellersCmd::Create { json } => {
                let body: Value = serde_json::from_str(&json)?;
                print(&client.create_reseller(&body).await?)?;
            }
            ResellersCmd::AddCredits {
                id,
                credits,
                reference,
                private_note,
            } => {
                print(
                    &client
                        .add_reseller_credits(&id, credits, &reference, &private_note, false)
                        .await?,
                )?;
            }
        },
        Cmd::CustomersCount => print(&client.customers_count().await?)?,
        Cmd::CustomersCountByServer => print(&client.customers_count_by_server().await?)?,
        Cmd::Expiring => print(&client.expiring().await?)?,
        Cmd::Notices => print(&client.notices().await?)?,
    }
    Ok(())
}

fn print<T: Serialize>(value: &T) -> anyhow::Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}
