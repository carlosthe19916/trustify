#![allow(unused)]

mod openapi;

use actix_web::{
    body::MessageBody,
    dev::{ConnectionInfo, Url},
    error::UrlGenerationError,
    get, web,
    web::Json,
    HttpRequest, HttpResponse, Responder,
};
use anyhow::Context;
use bytesize::ByteSize;
use futures::FutureExt;
use std::fmt::Display;
use std::fs::create_dir_all;
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;
use std::time::Duration;
use trustify_auth::{
    auth::AuthConfigArguments,
    authenticator::Authenticator,
    authorizer::Authorizer,
    swagger_ui::{swagger_ui_with_auth, SwaggerUiOidc, SwaggerUiOidcConfig},
};
use trustify_common::{
    config::{Database, StorageConfig},
    db,
};
use trustify_infrastructure::{
    app::http::{HttpServerBuilder, HttpServerConfig},
    endpoint::Trustify,
    health::checks::{Local, Probe},
    tracing::Tracing,
    Infrastructure, InfrastructureConfig, InitContext, Metrics,
};
use trustify_module_importer::server::importer;
use trustify_module_ingestor::graph::Graph;
use trustify_module_storage::service::dispatch::DispatchBackend;
use trustify_module_storage::service::fs::FileSystemBackend;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

#[cfg(feature = "ui")]
use trustify_module_ui::UI;

/// Run the API server
#[derive(clap::Args, Debug)]
pub struct Run {
    #[command(flatten)]
    pub database: Database,

    #[arg(long, env)]
    pub devmode: bool,

    /// Location of the storage
    #[command(flatten)]
    pub storage: StorageConfig,

    #[command(flatten)]
    pub infra: InfrastructureConfig,

    #[command(flatten)]
    pub auth: AuthConfigArguments,

    #[command(flatten)]
    pub http: HttpServerConfig<Trustify>,

    #[command(flatten)]
    pub swagger_ui_oidc: SwaggerUiOidcConfig,
}

const SERVICE_ID: &str = "trustify";

struct InitData {
    authenticator: Option<Arc<Authenticator>>,
    authorizer: Authorizer,
    graph: Arc<Graph>,
    db: db::Database,
    storage: DispatchBackend,
    http: HttpServerConfig<Trustify>,
    tracing: Tracing,
    swagger_oidc: Option<Arc<SwaggerUiOidc>>,
    #[cfg(feature = "ui")]
    ui: UI,
}

impl Run {
    pub async fn run(self) -> anyhow::Result<ExitCode> {
        // logging is only active once the infrastructure run method has been called
        Infrastructure::from(self.infra.clone())
            .run(
                SERVICE_ID,
                { |context| async move { InitData::new(context, self).await } },
                |context| async move { context.init_data.run(&context.metrics).await },
            )
            .await?;

        Ok(ExitCode::SUCCESS)
    }
}

impl InitData {
    async fn new(context: InitContext, run: Run) -> anyhow::Result<Self> {
        let (authn, authz) = run.auth.split(run.devmode)?.unzip();
        let authenticator: Option<Arc<Authenticator>> =
            Authenticator::from_config(authn).await?.map(Arc::new);
        let authorizer = Authorizer::new(authz);

        if authenticator.is_none() {
            log::warn!("Authentication is disabled");
        }

        let swagger_oidc = match authenticator.is_some() {
            true => SwaggerUiOidc::from_devmode_or_config(run.devmode, run.swagger_ui_oidc)
                .await?
                .map(Arc::new),
            false => None,
        };

        let db = db::Database::new(&run.database).await?;

        if run.devmode {
            db.migrate().await?;
        }

        let graph = Graph::new(db.clone());

        let check = Local::spawn_periodic("no database connection", Duration::from_secs(1), {
            let db = db.clone();
            move || {
                let db = db.clone();
                async move { db.ping().await.is_ok() }
            }
        })?;

        context.health.readiness.register("database", check).await;

        let storage = run
            .storage
            .fs_path
            .as_ref()
            .cloned()
            .unwrap_or_else(|| PathBuf::from("./.trustify/storage"));
        if run.devmode {
            create_dir_all(&storage).context(format!(
                "Failed to create filesystem storage directory: {:?}",
                run.storage.fs_path
            ))?;
        }

        let storage = DispatchBackend::Filesystem(FileSystemBackend::new(storage).await?);

        #[cfg(feature = "ui")]
        let ui = UI {
            // TODO: where/how should we configure these details?
            version: env!("CARGO_PKG_VERSION").to_string(),
            auth_required: String::from("true"),
            oidc_server_url: String::from("http://localhost:8090/realms/chicken"),
            oidc_client_id: String::from("frontend"),
            oidc_scope: String::from("openid"),
            analytics_enabled: String::from("false"),
            analytics_write_key: String::from(""),
        };

        Ok(InitData {
            authenticator,
            authorizer,
            graph: Arc::new(graph),
            db,
            http: run.http,
            tracing: run.infra.tracing,
            swagger_oidc,
            storage,
            #[cfg(feature = "ui")]
            ui,
        })
    }

    async fn run(self, metrics: &Metrics) -> anyhow::Result<()> {
        let swagger_oidc = self.swagger_oidc;

        let limit = ByteSize::gb(1).as_u64() as usize;

        let http = {
            let graph = self.graph.clone();
            let db = self.db.clone();
            let storage = self.storage.clone();

            HttpServerBuilder::try_from(self.http)?
                .tracing(self.tracing)
                .metrics(metrics.registry().clone(), SERVICE_ID)
                .default_authenticator(self.authenticator)
                .authorizer(self.authorizer.clone())
                .configure(move |svc| {
                    svc.app_data(web::PayloadConfig::default().limit(limit))
                        .service(swagger_ui_with_auth(
                            openapi::openapi(),
                            swagger_oidc.clone(),
                        ));
                    svc.app_data(web::Data::from(self.graph.clone()))
                        .configure(|svc| {
                            trustify_module_importer::endpoints::configure(svc, db.clone());

                            trustify_module_fundamental::endpoints::configure(
                                svc,
                                db.clone(),
                                storage.clone(),
                            );

                            #[cfg(feature = "ui")]
                            trustify_module_ui::endpoints::configure(svc, &self.ui);
                        });
                })
        };

        let http = async { http.run().await }.boxed_local();
        let importer = async { importer(self.db, self.storage).await }.boxed_local();

        let (result, _, _) = futures::future::select_all([http, importer]).await;

        log::info!("one of the server tasks returned, exiting");

        result
    }
}

fn build_url(ci: &ConnectionInfo, path: impl Display) -> Option<url::Url> {
    url::Url::parse(&format!(
        "{scheme}://{host}{path}",
        scheme = ci.scheme(),
        host = ci.host()
    ))
    .ok()
}

#[get("/")]
async fn index(ci: ConnectionInfo) -> Result<Json<Vec<url::Url>>, UrlGenerationError> {
    let mut result = vec![];

    result.extend(build_url(&ci, "/"));
    result.extend(build_url(&ci, "/openapi.json"));
    result.extend(build_url(&ci, "/swagger-ui/"));

    Ok(Json(result))
}
