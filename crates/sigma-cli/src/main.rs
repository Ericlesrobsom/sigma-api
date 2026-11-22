use anyhow::Context;
use clap::{Parser, Subcommand};
use serde::Serialize;
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
    Resellers,
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
        },
        Cmd::Packages => print(&client.packages().await?)?,
        Cmd::Servers => print(&client.servers().await?)?,
        Cmd::CreditOrders => print(&client.credit_orders().await?)?,
        Cmd::CreditTransactions { page } => print(&client.credit_transactions(page).await?)?,
        Cmd::CreditSummary => print(&client.credit_summary().await?)?,
        Cmd::Resellers => print(&client.resellers().await?)?,
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
