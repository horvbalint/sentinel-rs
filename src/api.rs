use anyhow::Result;
use axum::{
    Json, Router,
    extract::{Path, State},
    response::IntoResponse,
    routing::get,
};
use tokio::sync::oneshot;

use crate::db::{DbCommand, DbCommandChannel};

pub fn task(db_tx: DbCommandChannel) -> tokio::task::JoinHandle<Result<()>> {
    tokio::task::spawn(async move {
        let app = Router::new()
            .route("/host/cpu/current", get(host_cpu_current))
            .route("/host/memory/current", get(host_memory_current))
            .route("/{container}/cpu/current", get(container_cpu_current))
            .route("/{container}/memory/current", get(container_memory_current))
            .with_state(db_tx);

        let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
        axum::serve(listener, app).await?;

        Ok(())
    })
}

async fn host_cpu_current(State(tx): State<DbCommandChannel>) -> impl IntoResponse {
    query_db_controller(tx, |respond_to| DbCommand::GetLastCpuUsage {
        container: None,
        respond_to,
    })
    .await
}

async fn host_memory_current(State(tx): State<DbCommandChannel>) -> impl IntoResponse {
    query_db_controller(tx, |respond_to| DbCommand::GetLastMemoryUsage {
        container: None,
        respond_to,
    })
    .await
}

async fn container_cpu_current(
    State(tx): State<DbCommandChannel>,
    Path(container): Path<String>,
) -> impl IntoResponse {
    query_db_controller(tx, |respond_to| DbCommand::GetLastCpuUsage {
        container: Some(container),
        respond_to,
    })
    .await
}

async fn container_memory_current(
    State(tx): State<DbCommandChannel>,
    Path(container): Path<String>,
) -> impl IntoResponse {
    query_db_controller(tx, |respond_to| DbCommand::GetLastMemoryUsage {
        container: Some(container),
        respond_to,
    })
    .await
}

async fn query_db_controller<T, F>(db_tx: DbCommandChannel, fun: F) -> Json<Option<T>>
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
