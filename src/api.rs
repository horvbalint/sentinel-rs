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

use crate::{
    db::{DbChannelTx, DbCommand},
    types::Interval,
};

pub async fn start(db_tx: DbChannelTx) -> Result<()> {
    let app = Router::new()
        .route("/host/cpu/last", get(cpu_last))
        .route("/host/cpu/last/{interval}", get(cpu_interval))
        .route("/host/cpu/history", get(cpu_history))
        .route("/host/memory/last", get(memory_last))
        .route("/host/memory/last/{interval}", get(memory_interval))
        .route("/host/memory/history", get(memory_history))
        .route("/{container}/cpu/last", get(cpu_last))
        .route("/{container}/cpu/last/{interval}", get(cpu_interval))
        .route("/{container}/cpu/history", get(cpu_history))
        .route("/{container}/memory/last", get(memory_last))
        .route("/{container}/memory/last/{interval}", get(memory_interval))
        .route("/{container}/memory/history", get(memory_history))
        .with_state(db_tx);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn memory_last(
    State(tx): State<DbChannelTx>,
    container: Option<Path<String>>,
) -> impl IntoResponse {
    query_one(tx, |respond_to| DbCommand::GetLastMemoryUsage {
        container: container.map(|p| p.0),
        respond_to,
    })
    .await
}

async fn cpu_last(
    State(tx): State<DbChannelTx>,
    container: Option<Path<String>>,
) -> impl IntoResponse {
    query_one(tx, |respond_to| DbCommand::GetLastCpuUsage {
        container: container.map(|p| p.0),
        respond_to,
    })
    .await
}

#[derive(Debug, Deserialize)]
struct IntervalRouteParams {
    container: Option<String>,
    interval: Interval,
}

async fn cpu_interval(
    State(tx): State<DbChannelTx>,
    Path(params): Path<IntervalRouteParams>,
) -> impl IntoResponse {
    query_multiple(tx, |respond_to| DbCommand::GetIntervalCpuUsage {
        container: params.container,
        interval: params.interval,
        respond_to,
    })
    .await
}

async fn memory_interval(
    State(tx): State<DbChannelTx>,
    Path(params): Path<IntervalRouteParams>,
) -> impl IntoResponse {
    query_multiple(tx, |respond_to| DbCommand::GetIntervalMemoryUsage {
        container: params.container,
        interval: params.interval,
        respond_to,
    })
    .await
}

#[derive(Debug, Deserialize)]
struct BetweenQueryParams {
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
}

async fn cpu_history(
    State(tx): State<DbChannelTx>,
    container: Option<Path<String>>,
    Query(BetweenQueryParams { from, to }): Query<BetweenQueryParams>,
) -> impl IntoResponse {
    query_multiple(tx, move |respond_to| DbCommand::GetCpuUsageHistory {
        from: from,
        to,
        container: container.map(|p| p.0),
        respond_to,
    })
    .await
}

async fn memory_history(
    State(tx): State<DbChannelTx>,
    container: Option<Path<String>>,
    Query(BetweenQueryParams { from, to }): Query<BetweenQueryParams>,
) -> impl IntoResponse {
    query_multiple(tx, move |respond_to| DbCommand::GetMemoryUsageHistory {
        from,
        to,
        container: container.map(|p| p.0),
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
