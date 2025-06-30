use std::time::Duration;

use axum::{
    Router,
    extract::{Path, State},
    response::Json,
    routing::get,
};
use bollard::{
    Docker,
    query_parameters::{ListContainersOptions, StatsOptions},
    secret::ContainerCpuStats,
};
use futures_util::{StreamExt, future};
use rusqlite::Connection;
use sysinfo::MINIMUM_CPU_UPDATE_INTERVAL;
use tokio::sync::{
    mpsc::{self, UnboundedSender},
    oneshot,
};

use crate::types::{CpuUsage, MemoryUsage};

mod api;
mod db;
mod sys;
mod types;

enum DbCommand {
    InsertResourceUsage {
        timestamp: chrono::NaiveDateTime,
        cpu_usage: CpuUsage,
        memory_usage: MemoryUsage,
        container: Option<String>,
    },
    GetLastCpuUsage {
        container: Option<String>,
        respond_to: oneshot::Sender<Option<CpuUsage>>,
    },
    GetLastMemoryUsage {
        container: Option<String>,
        respond_to: oneshot::Sender<Option<MemoryUsage>>,
    },
}

#[tokio::main]
async fn main() {
    let (db_tx, mut db_rx) = mpsc::unbounded_channel::<DbCommand>();

    let db_future = tokio::task::spawn_blocking(move || {
        let connection = Connection::open("./test.db").expect("Failed to connect to database");
        let mut db = db::Db::new(&connection);

        while let Some(command) = db_rx.blocking_recv() {
            match command {
                DbCommand::InsertResourceUsage {
                    timestamp,
                    memory_usage,
                    cpu_usage,
                    container,
                } => {
                    db.insert_resource_usage(timestamp, memory_usage, cpu_usage, container);
                }
                DbCommand::GetLastCpuUsage {
                    container,
                    respond_to,
                } => {
                    respond_to
                        .send(db.get_last_cpu_usage(container))
                        .expect("failed to send response to GetLastCpuUsage");
                }
                DbCommand::GetLastMemoryUsage {
                    container,
                    respond_to,
                } => {
                    respond_to
                        .send(db.get_last_memory_usage(container))
                        .expect("failed to send response to GetLastMemoryUsage");
                }
            };
        }
    });

    let collector_db_tx = db_tx.clone();
    let info_collector_future = tokio::task::spawn(async move {
        let docker = Docker::connect_with_socket_defaults().unwrap();
        let mut sys_info_collector = sys::SysInfoCollector::new();
        tokio::time::sleep(MINIMUM_CPU_UPDATE_INTERVAL).await;

        loop {
            let timestamp = chrono::Local::now().naive_local();

            // host
            sys_info_collector.refresh();
            let host_memory_usage = sys_info_collector.get_memory_usage();
            let host_total_memory = host_memory_usage.total;

            collector_db_tx
                .send(DbCommand::InsertResourceUsage {
                    timestamp,
                    cpu_usage: sys_info_collector.get_cpu_usage(),
                    memory_usage: host_memory_usage,
                    container: None,
                })
                .expect("Failed to send DB command");

            // containers
            let containers = docker
                .list_containers(Some(ListContainersOptions {
                    all: false,
                    ..Default::default()
                }))
                .await
                .unwrap();

            let handles = containers.into_iter().map(|container| {
                let docker = docker.clone();
                let collector_db_tx = collector_db_tx.clone();
                tokio::task::spawn(async move {
                    let container_id = &container.id.unwrap();

                    let stat_stream = &mut docker.stats(
                        container_id,
                        Some(StatsOptions {
                            stream: false,
                            ..Default::default()
                        }),
                    );

                    if let Some(Ok(stats)) = stat_stream.next().await
                        && let Some(cpu_stat) = stats.cpu_stats
                        && let Some(prev_cpu_stat) = stats.precpu_stats
                        && let Some(memory_stat) = stats.memory_stats
                        && let Some(cpu_percent) =
                            calculate_container_cpu_usage(prev_cpu_stat, cpu_stat)
                    {
                        collector_db_tx
                            .send(DbCommand::InsertResourceUsage {
                                timestamp,
                                cpu_usage: CpuUsage { usage: cpu_percent },
                                memory_usage: MemoryUsage {
                                    total: memory_stat.limit.unwrap_or(host_total_memory),
                                    used: memory_stat.usage.unwrap(),
                                },
                                container: Some(container_id.clone()),
                            })
                            .expect("Failed to send DB command for container stats");
                    }
                })
            });

            future::join_all(handles).await;
            println!("Inserted CPU and memory usage data");

            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    });

    let server_future = tokio::task::spawn(async move {
        let app = Router::new()
            .route(
                "/host/cpu/current",
                get(async |State(db_tx): State<UnboundedSender<DbCommand>>| {
                    let (tx, rx) = tokio::sync::oneshot::channel();
                    let _ = db_tx.send(DbCommand::GetLastCpuUsage {
                        container: None,
                        respond_to: tx,
                    });
                    match rx.await {
                        Ok(result) => Json(result),
                        Err(_) => Json(None),
                    }
                }),
            )
            .route(
                "/host/memory/current",
                get(async |State(db_tx): State<UnboundedSender<DbCommand>>| {
                    let (tx, rx) = tokio::sync::oneshot::channel();
                    let _ = db_tx.send(DbCommand::GetLastMemoryUsage {
                        container: None,
                        respond_to: tx,
                    });
                    match rx.await {
                        Ok(result) => Json(result),
                        Err(_) => Json(None),
                    }
                }),
            )
            .route(
                "/{container}/cpu/current",
                get(
                    async |State(db_tx): State<UnboundedSender<DbCommand>>,
                           Path(container): Path<String>| {
                        let (tx, rx) = tokio::sync::oneshot::channel();
                        let _ = db_tx.send(DbCommand::GetLastCpuUsage {
                            container: Some(container),
                            respond_to: tx,
                        });
                        match rx.await {
                            Ok(result) => Json(result),
                            Err(_) => Json(None),
                        }
                    },
                ),
            )
            .route(
                "/{container}/memory/current",
                get(
                    async |State(db_tx): State<UnboundedSender<DbCommand>>,
                           Path(container): Path<String>| {
                        let (tx, rx) = tokio::sync::oneshot::channel();
                        let _ = db_tx.send(DbCommand::GetLastMemoryUsage {
                            container: Some(container),
                            respond_to: tx,
                        });
                        match rx.await {
                            Ok(result) => Json(result),
                            Err(_) => Json(None),
                        }
                    },
                ),
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

fn calculate_container_cpu_usage(
    prev_cpu_stat: ContainerCpuStats,
    cpu_stat: ContainerCpuStats,
) -> Option<f32> {
    let delta_container_cpu_usage =
        cpu_stat.cpu_usage?.total_usage? - prev_cpu_stat.cpu_usage?.total_usage?;
    let delta_system_cpu_usage = cpu_stat.system_cpu_usage? - prev_cpu_stat.system_cpu_usage?;

    Some(
        (delta_container_cpu_usage as f64 / delta_system_cpu_usage as f64
            * cpu_stat.online_cpus? as f64
            * 100.0) as f32,
    )
}
