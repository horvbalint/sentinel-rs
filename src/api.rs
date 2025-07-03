use anyhow::Result;
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    response::IntoResponse,
    routing::get,
};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use tokio::sync::oneshot;

use crate::db::{DbChannelTx, DbCommand};

pub async fn start(db_tx: DbChannelTx) -> Result<()> {
    let app = Router::new()
        .route("/host/cpu/last", get(host_cpu_last))
        .route("/host/cpu/history", get(host_cpu_history))
        .route("/host/memory/last", get(host_memory_last))
        .route("/host/memory/history", get(host_memory_history))
        .route("/{container}/cpu/last", get(container_cpu_last))
        .route("/{container}/cpu/history", get(container_cpu_history))
        .route("/{container}/memory/last", get(container_memory_last))
        .route("/{container}/memory/history", get(container_memory_history))
        .with_state(db_tx);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn host_cpu_last(State(tx): State<DbChannelTx>) -> impl IntoResponse {
    query_one(tx, |respond_to| DbCommand::GetLastCpuUsage {
        container: None,
        respond_to,
    })
    .await
}

async fn host_memory_last(State(tx): State<DbChannelTx>) -> impl IntoResponse {
    query_one(tx, |respond_to| DbCommand::GetLastMemoryUsage {
        container: None,
        respond_to,
    })
    .await
}

async fn container_cpu_last(
    State(tx): State<DbChannelTx>,
    Path(container): Path<String>,
) -> impl IntoResponse {
    query_one(tx, |respond_to| DbCommand::GetLastCpuUsage {
        container: Some(container),
        respond_to,
    })
    .await
}

async fn container_memory_last(
    State(tx): State<DbChannelTx>,
    Path(container): Path<String>,
) -> impl IntoResponse {
    query_one(tx, |respond_to| DbCommand::GetLastMemoryUsage {
        container: Some(container),
        respond_to,
    })
    .await
}

#[derive(Debug, Deserialize)]
struct BetweenParams {
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
}

async fn host_cpu_history(
    State(tx): State<DbChannelTx>,
    Query(BetweenParams { from, to }): Query<BetweenParams>,
) -> impl IntoResponse {
    query_multiple(tx, move |respond_to| DbCommand::GetCpuUsageHistory {
        from: from,
        to,
        container: None,
        respond_to,
    })
    .await
}

async fn host_memory_history(
    State(tx): State<DbChannelTx>,
    Query(BetweenParams { from, to }): Query<BetweenParams>,
) -> impl IntoResponse {
    query_multiple(tx, move |respond_to| DbCommand::GetMemoryUsageHistory {
        from,
        to,
        container: None,
        respond_to,
    })
    .await
}

async fn container_cpu_history(
    State(tx): State<DbChannelTx>,
    Path(container): Path<String>,
    Query(BetweenParams { from, to }): Query<BetweenParams>,
) -> impl IntoResponse {
    query_multiple(tx, move |respond_to| DbCommand::GetCpuUsageHistory {
        from,
        to,
        container: Some(container),
        respond_to,
    })
    .await
}

async fn container_memory_history(
    State(tx): State<DbChannelTx>,
    Path(container): Path<String>,
    Query(BetweenParams { from, to }): Query<BetweenParams>,
) -> impl IntoResponse {
    query_multiple(tx, move |respond_to| DbCommand::GetMemoryUsageHistory {
        from,
        to,
        container: Some(container),
        respond_to,
    })
    .await
}

async fn query_one<T, F>(db_tx: DbChannelTx, fun: F) -> Json<Option<T>>
where
    F: FnOnce(oneshot::Sender<Option<T>>) -> DbCommand,
{
    let (tx, rx) = oneshot::channel();
    let _ = db_tx.send(fun(tx));

    match rx.await {
        Ok(result) => Json(result),
        Err(_) => Json(None),
    }
}

async fn query_multiple<T, F>(db_tx: DbChannelTx, fun: F) -> Json<Vec<T>>
where
    F: FnOnce(oneshot::Sender<Vec<T>>) -> DbCommand,
{
    let (tx, rx) = oneshot::channel();
    let _ = db_tx.send(fun(tx));

    match rx.await {
        Ok(result) => Json(result),
        Err(_) => Json(vec![]),
    }
}
