mod core;

use std::{
    borrow::Cow, fmt::Display, fs::File, hash::RandomState, net::SocketAddr, num::NonZeroUsize,
    path::PathBuf, pin::pin, str::FromStr, sync::Arc,
};

use crate::jsonrpc_types;
use anyhow::Context as _;
use clap::Parser;
use core::CheckAllMethods;
use futures::{
    future::{self, Either},
    stream, StreamExt as _,
};
use http::Uri;
use http_body_util::{BodyExt as _, Full};
use hyper::body::{Bytes, Incoming};
use hyper_util::client::legacy::{connect::HttpConnector, Client};
use jsonschema::CompilationOptions;
use serde::Deserialize;
use tokio::{net::TcpListener, signal};
use tokio_stream::wrappers::TcpListenerStream;
use tracing::{debug, error, info, info_span, Instrument};
use tracing_subscriber::EnvFilter;

struct Config {
    remote: Uri,
    check: CheckAllMethods,
}

#[derive(Parser)]
pub struct Args {
    local: SocketAddr,
    remote: Uri,
    spec: PathBuf,
    #[arg(short, long)]
    concurrency: Option<NonZeroUsize>,
    #[arg(long)]
    log: Option<PathBuf>,
}

#[derive(Deserialize)]
struct Log {
    #[serde(flatten)]
    subscriber: tracing_configuration::Subscriber,
    #[serde(deserialize_with = "from_str")]
    filter: EnvFilter,
}

async fn proxy(
    client_request: http::Request<Incoming>,
    origin: &Client<HttpConnector, Full<Bytes>>,
    config: &Config,
) -> anyhow::Result<http::Response<Full<Bytes>>> {
    let (mut req_parts, req_body) = client_request.into_parts();
    let req_body = req_body
        .collect()
        .await
        .context("couldn't collect client request")?
        .to_bytes();

    req_parts.uri.clone_from(&config.remote);

    let response = origin
        .request(http::Request::from_parts(
            req_parts,
            Full::new(req_body.clone()),
        ))
        .await
        .context("couldn't forward to origin")?;

    let (resp_parts, resp_body) = response.into_parts();
    let resp_body = resp_body
        .collect()
        .await
        .context("couldn't collect origin response")?
        .to_bytes();

    if let (Ok(request), Ok(response)) = (
        serde_json::from_slice::<jsonrpc_types::Request>(&req_body),
        serde_json::from_slice::<jsonrpc_types::Response>(&resp_body),
    ) {
        match config.check.get(&request.method) {
            Some(check) => {
                let annotations = check.check(&request, Some(&response));
                match annotations.is_empty() {
                    true => info!(target: "app::validate", method = %request.method, "passed"),
                    false => {
                        info!(target: "app::validate", method = %request.method, ?annotations, "failed")
                    }
                }
            }
            None => debug!(target: "app::skip", "not a specified method"),
        }
    } else {
        debug!(target: "app::skip", "not a JSON-RPC exchange")
    }

    Ok(http::Response::from_parts(resp_parts, Full::new(resp_body)))
}

/// ```text
/// ┌────────┬─request──►┌────────────┬─request──►┌────────┐
/// │ client │           │ us (proxy) │           │ origin │
/// └────────┘◄─response─└────────────┘◄─response─└────────┘
/// ```
pub async fn main(
    Args {
        local,
        remote,
        spec,
        concurrency,
        log,
    }: Args,
) -> anyhow::Result<()> {
    let check = CheckAllMethods::new_with_hasher_and_compilation_options(
        serde_json::from_reader(File::open(spec).context("couldn't open file")?)
            .context("invalid spec file")?,
        RandomState::new(),
        &CompilationOptions::default(),
    )
    .context("invalid spec file")?;

    if let Some(log) = log {
        let Log { subscriber, filter } =
            serde_json::from_reader(File::open(log).context("couldn't open logging config file")?)
                .context("invalid logging config file")?;
        let (builder, _guard) = subscriber
            .try_builder()
            .context("couldn't set up logging")?;
        builder.with_env_filter(filter).init();
    }

    let config = Arc::new(Config { remote, check });

    let us = &hyper_util::server::conn::auto::Builder::new(hyper_util::rt::TokioExecutor::new());
    let origin = Client::builder(hyper_util::rt::TokioExecutor::new())
        .build::<_, Full<Bytes>>(HttpConnector::new());

    let (client_connections, stop_accepting_connections) =
        stream::abortable(TcpListenerStream::new(
            TcpListener::bind(local)
                .await
                .context("couldn't bind address")?,
        ));

    info!(target: "app::serve", addr = %local, "listening");

    let serve_clients = client_connections
        .map(|client_connection| {
            let origin = &origin;
            let config = &config;
            async move {
                match client_connection.and_then(|conn| conn.peer_addr().map(|addr| (conn, addr))) {
                    Ok((conn, addr)) => {
                        let handle = us
                            .serve_connection_with_upgrades(
                                // ^~~ requires 'static
                                hyper_util::rt::TokioIo::new(conn),
                                hyper::service::service_fn({
                                    |incoming_request: http::Request<Incoming>| {
                                        let them = origin.clone();
                                        let config = config.clone();
                                        async move {
                                            proxy(incoming_request, &them, &config)
                                                .instrument(info_span!(
                                                    target: "app",
                                                    "serving client",
                                                    %addr
                                                ))
                                                .await
                                        }
                                    }
                                }),
                            )
                            .await;
                        match handle {
                            Ok(()) => debug!(
                                target: "app::serve",
                                %addr,
                                "finished serving client"
                            ),
                            Err(e) => {
                                error!(
                                    target: "app::serve",
                                    error = ?anyhow::Chain::new(&*e).map(|it|it.to_string()).collect::<Vec<_>>(),
                                    %addr,
                                    "error serving client"
                                );
                            }
                        };
                    }
                    Err(error) => {
                        error!(
                            target: "app::accept",
                            ?error,
                            "error accepting connection from client",
                        )
                    }
                }
            }
        })
        .buffered(match concurrency {
            Some(it) => it.get(),
            None => num_cpus::get(),
        })
        .collect::<()>();

    match future::select(serve_clients, pin!(signal::ctrl_c())).await {
        Either::Left(((), _)) => {} // unreachable
        Either::Right((_signal_or_err, cont)) => {
            stop_accepting_connections.abort();
            info!(
                target: "app::shutdown",
                "graceful shutdown on Ctrl-C, finishing outstanding requests (repeat to force)"
            );
            match future::select(cont, pin!(signal::ctrl_c())).await {
                Either::Left(((), _)) => info!(
                    target: "app::shutdown",
                    "finished graceful shutdown"
                ),
                Either::Right((_signal_or_err, _)) => info!(
                    target: "app::shutdown",
                    "forced shutdown"
                ),
            }
        }
    };

    Ok(())
}

fn from_str<'de, D: serde::Deserializer<'de>, T: FromStr<Err = E>, E: Display>(
    d: D,
) -> Result<T, D::Error> {
    Cow::<str>::deserialize(d)?
        .parse()
        .map_err(serde::de::Error::custom)
}
