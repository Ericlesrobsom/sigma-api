use anyhow::Context;
use clap::{Parser, Subcommand};
use serde::Serialize;
use serde_json::Value;
use sigma_core::{
    ConnectionsChange, CourtesyDays, CustomerCreate, CustomerQuery, CustomerUpdate, PackageChange,
    RenewRequest, SigmaClient, SigmaConfig,
};

#[derive(Parser)]
#[command(name = "sigma", version, about = "CLI da Sigma Reseller API v1")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Perfil da conta dona do token.
    Me,
    /// Operacoes de clientes.
    Customers {
        #[command(subcommand)]
        cmd: CustomersCmd,
    },
    /// Cupons de desconto.
    Coupons {
        #[command(subcommand)]
        cmd: CouponsCmd,
    },
    /// Revendas e creditos.
    Resellers {
        #[command(subcommand)]
        cmd: ResellersCmd,
    },
    /// DNS proprio e de revendas.
    Dns {
        #[command(subcommand)]
        cmd: DnsCmd,
    },
    /// Pacotes/planos do catalogo.
    Packages,
    /// Servidores de provisionamento.
    Servers,
    /// Relatorios: customers-count, resellers-count, membership, low-credits, new-resellers, expiring.
    Reports { kind: String },
}

#[derive(Subcommand)]
enum CustomersCmd {
    /// Lista clientes com paginacao/filtros.
    List {
        #[arg(long)]
        page: Option<u32>,
        #[arg(long = "per-page")]
        per_page: Option<u32>,
        #[arg(long)]
        username: Option<String>,
        #[arg(long)]
        status: Option<String>,
    },
    /// Mostra um cliente pelo id.
    Get { id: String },
    /// Cria um cliente.
    Create {
        #[arg(long = "server-id")]
        server_id: String,
        #[arg(long = "package-id")]
        package_id: String,
        #[arg(long)]
        username: Option<String>,
        #[arg(long)]
        password: Option<String>,
        #[arg(long, default_value_t = 1)]
        connections: i64,
    },
    /// Renova um cliente.
    Renew {
        id: String,
        #[arg(long = "package-id")]
        package_id: Option<String>,
        #[arg(long)]
        connections: Option<i64>,
    },
    /// Troca o pacote.
    Package {
        id: String,
        #[arg(long = "package-id")]
        package_id: String,
    },
    /// Troca as conexoes.
    Connections { id: String, connections: i64 },
    /// Adiciona dias de cortesia (1..365).
    Courtesy { id: String, days: i64 },
    /// Atualiza campos do cliente via JSON.
    Update { id: String, json: String },
}

#[derive(Subcommand)]
enum CouponsCmd {
    List,
    Get { id: String },
    Create { json: String },
    Delete { id: String },
}

#[derive(Subcommand)]
enum ResellersCmd {
    List,
    Get { id: String },
    Create { json: String },
    Update { id: String, json: String },
    CreditsAdd { id: String, json: String },
    CreditsRemove { id: String, json: String },
    CreditsRecharge { id: String, json: String },
}

#[derive(Subcommand)]
enum DnsCmd {
    /// DNS proprio.
    Me,
    /// Define o DNS proprio via JSON.
    Set { json: String },
    /// Lista DNS das revendas.
    Resellers,
    /// DNS de uma revenda.
    Reseller { id: String },
    /// Define o DNS de uma revenda via JSON.
    SetReseller { id: String, json: String },
    /// Substituicao em lote via JSON.
    Replace { json: String },
    /// Aplica o seu DNS nas revendas via JSON.
    Apply { json: String },
    /// Historico de mudancas.
    History,
    /// Reverte um lote.
    Revert { batch: String },
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
            } => {
                let query = CustomerQuery {
                    page,
                    per_page,
                    username,
                    status,
                    is_trial: None,
                };
                print(&client.customers(&query).await?)?;
            }
            CustomersCmd::Get { id } => print(&client.customer(&id).await?)?,
            CustomersCmd::Create {
                server_id,
                package_id,
                username,
                password,
                connections,
            } => {
                let body = CustomerCreate {
                    server_id,
                    package_id,
                    username,
                    password,
                    mac_address: None,
                    plan_price: None,
                    expires_at: None,
                    trial_hours: None,
                    connections: Some(connections),
                    name: None,
                    email: None,
                    telegram: None,
                    whatsapp: None,
                    note: None,
                    playlist: None,
                    bouquets: None,
                    status: None,
                    is_mag: None,
                    is_asnlock: None,
                    is_isplock: None,
                };
                print(&client.create_customer(&body).await?)?;
            }
            CustomersCmd::Renew {
                id,
                package_id,
                connections,
            } => {
                print(
                    &client
                        .renew_customer(
                            &id,
                            &RenewRequest {
                                package_id,
                                connections,
                            },
                        )
                        .await?,
                )?;
            }
            CustomersCmd::Package { id, package_id } => {
                print(
                    &client
                        .change_package(&id, &PackageChange { package_id })
                        .await?,
                )?;
            }
            CustomersCmd::Connections { id, connections } => {
                print(
                    &client
                        .change_connections(&id, &ConnectionsChange { connections })
                        .await?,
                )?;
            }
            CustomersCmd::Courtesy { id, days } => {
                print(&client.courtesy_days(&id, &CourtesyDays { days }).await?)?;
            }
            CustomersCmd::Update { id, json } => {
                let body: CustomerUpdate = serde_json::from_str(&json)?;
                print(&client.update_customer(&id, &body).await?)?;
            }
        },
        Cmd::Coupons { cmd } => match cmd {
            CouponsCmd::List => print(&client.coupons().await?)?,
            CouponsCmd::Get { id } => print(&client.coupon(&id).await?)?,
            CouponsCmd::Create { json } => {
                let body: Value = serde_json::from_str(&json)?;
                print(&client.create_coupon(&body).await?)?;
            }
            CouponsCmd::Delete { id } => print(&client.delete_coupon(&id).await?)?,
        },
        Cmd::Resellers { cmd } => match cmd {
            ResellersCmd::List => print(&client.resellers().await?)?,
            ResellersCmd::Get { id } => print(&client.reseller(&id).await?)?,
            ResellersCmd::Create { json } => {
                let body: Value = serde_json::from_str(&json)?;
                print(&client.create_reseller(&body).await?)?;
            }
            ResellersCmd::Update { id, json } => {
                let body: Value = serde_json::from_str(&json)?;
                print(&client.update_reseller(&id, &body).await?)?;
            }
            ResellersCmd::CreditsAdd { id, json } => {
                let body: Value = serde_json::from_str(&json)?;
                print(&client.reseller_credits(&id, "add", &body).await?)?;
            }
            ResellersCmd::CreditsRemove { id, json } => {
                let body: Value = serde_json::from_str(&json)?;
                print(&client.reseller_credits(&id, "remove", &body).await?)?;
            }
            ResellersCmd::CreditsRecharge { id, json } => {
                let body: Value = serde_json::from_str(&json)?;
                print(&client.reseller_credits(&id, "recharge", &body).await?)?;
            }
        },
        Cmd::Dns { cmd } => match cmd {
            DnsCmd::Me => print(&client.own_dns().await?)?,
            DnsCmd::Set { json } => {
                let body: Value = serde_json::from_str(&json)?;
                print(&client.set_own_dns(&body).await?)?;
            }
            DnsCmd::Resellers => print(&client.resellers_dns().await?)?,
            DnsCmd::Reseller { id } => print(&client.reseller_dns(&id).await?)?,
            DnsCmd::SetReseller { id, json } => {
                let body: Value = serde_json::from_str(&json)?;
                print(&client.set_reseller_dns(&id, &body).await?)?;
            }
            DnsCmd::Replace { json } => {
                let body: Value = serde_json::from_str(&json)?;
                print(&client.replace_dns(&body).await?)?;
            }
            DnsCmd::Apply { json } => {
                let body: Value = serde_json::from_str(&json)?;
                print(&client.apply_dns(&body).await?)?;
            }
            DnsCmd::History => print(&client.dns_history().await?)?,
            DnsCmd::Revert { batch } => print(&client.revert_dns(&batch).await?)?,
        },
        Cmd::Packages => print(&client.packages().await?)?,
        Cmd::Servers => print(&client.servers().await?)?,
        Cmd::Reports { kind } => print(&client.report(&kind).await?)?,
    }
    Ok(())
}

fn print<T: Serialize>(value: &T) -> anyhow::Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}
