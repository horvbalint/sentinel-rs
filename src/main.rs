use std::time::Duration;

use axum::{Router, extract::State, response::Json, routing::get};
use rusqlite::Connection;
use tokio::sync::mpsc::UnboundedSender;

use crate::types::{CpuUsage, MemoryUsage};

mod api;
mod db;
mod sys;
mod types;

enum DbCommand {
    InsertUsage {
        timestamp: chrono::NaiveDateTime,
        cpu_usage: CpuUsage,
        memory_usage: MemoryUsage,
    },
    GetLastCpuUsage {
        respond_to: tokio::sync::oneshot::Sender<Option<CpuUsage>>,
    },
    GetLastMemoryUsage {
        respond_to: tokio::sync::oneshot::Sender<Option<MemoryUsage>>,
    },
}

#[tokio::main]
async fn main() {
    let (db_tx, mut db_rx) = tokio::sync::mpsc::unbounded_channel::<DbCommand>();

    let db_future = tokio::task::spawn_blocking(move || {
        let connection = Connection::open_in_memory().expect("Failed to connect to database");
        let mut db = db::Db::new(&connection);

        while let Some(command) = db_rx.blocking_recv() {
            match command {
                DbCommand::InsertUsage {
                    timestamp,
                    memory_usage,
                    cpu_usage,
                } => {
                    db.insert_usage(timestamp, memory_usage, cpu_usage);
                }
                DbCommand::GetLastCpuUsage { respond_to } => {
                    respond_to
                        .send(db.get_last_cpu_usage())
                        .expect("failed to send response to GetLastCpuUsage");
                }
                DbCommand::GetLastMemoryUsage { respond_to } => {
                    respond_to
                        .send(db.get_last_memory_usage())
                        .expect("failed to send response to GetLastMemoryUsage");
                }
            };
        }
    });

    let collector_db_tx = db_tx.clone();
    let info_collector_future = tokio::task::spawn_blocking(move || {
        let mut sys_info_collector = sys::SysInfoCollector::new();

        loop {
            std::thread::sleep(Duration::from_secs(1));
            sys_info_collector.refresh();

            collector_db_tx
                .send(DbCommand::InsertUsage {
                    timestamp: chrono::Local::now().naive_local(),
                    cpu_usage: sys_info_collector.get_cpu_usage(),
                    memory_usage: sys_info_collector.get_memory_usage(),
                })
                .expect("Failed to send DB command");

            println!("Inserted CPU and memory usage data");
        }
    });

    let server_future = tokio::task::spawn(async move {
        let app = Router::new()
            .route("/", get(|| async { "Hello, World!" }))
            .route(
                "/cpu/current",
                get(async |State(db_tx): State<UnboundedSender<DbCommand>>| {
                    let (tx, rx) = tokio::sync::oneshot::channel();
                    let _ = db_tx.send(DbCommand::GetLastCpuUsage { respond_to: tx });
                    match rx.await {
                        Ok(result) => Json(result),
                        Err(_) => Json(None),
                    }
                }),
            )
            .route(
                "/memory/current",
                get(async |State(db_tx): State<UnboundedSender<DbCommand>>| {
                    let (tx, rx) = tokio::sync::oneshot::channel();
                    let _ = db_tx.send(DbCommand::GetLastMemoryUsage { respond_to: tx });
                    match rx.await {
                        Ok(result) => Json(result),
                        Err(_) => Json(None),
                    }
                }),
            )
            .with_state(db_tx);

        let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
        axum::serve(listener, app).await.unwrap();
    });

    let (db_result, server_result, _) =
        tokio::join!(db_future, server_future, info_collector_future);

    if let Err(e) = db_result {
        eprintln!("Error in database thread: {}", e);
    }

    if let Err(e) = server_result {
        eprintln!("Error in server thread: {}", e);
    }
}
