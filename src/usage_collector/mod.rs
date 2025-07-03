use anyhow::Result;
use bollard::{Docker, query_parameters::ListContainersOptions};
use chrono::Utc;
use std::time::Duration;
use sysinfo::MINIMUM_CPU_UPDATE_INTERVAL;

use crate::db::{DbChannelTx, DbCommand};

mod container;
mod host;

pub async fn start(db_tx: DbChannelTx) -> Result<()> {
    let docker = Docker::connect_with_socket_defaults()?;
    let mut host_usage_collector = host::UsageCollector::new();

    tokio::time::sleep(MINIMUM_CPU_UPDATE_INTERVAL).await;
    let mut ticker = tokio::time::interval(Duration::from_secs(5));

    loop {
        ticker.tick().await;

        collect_information(&mut host_usage_collector, &docker, db_tx.clone()).await?;
        println!("Inserted CPU and memory usage data");
    }
}

async fn collect_information(
    host_usage_collector: &mut host::UsageCollector,
    docker: &Docker,
    db_tx: DbChannelTx,
) -> Result<()> {
    let timestamp = Utc::now();

    // host
    host_usage_collector.refresh();

    db_tx.send(DbCommand::InsertResourceUsage {
        timestamp,
        cpu_usage: host_usage_collector.get_cpu_usage(),
        memory_usage: host_usage_collector.get_memory_usage(),
        container: None,
    })?;

    // containers
    let containers = docker
        .list_containers(Some(ListContainersOptions::default()))
        .await?;

    let container_futures = containers
        .into_iter()
        .filter_map(|container| container.id)
        .map(async |container_id| {
            let usage = container::get_resource_usage(&docker, &container_id).await;
            match usage {
                Some((cpu_usage, memory_usage)) => {
                    let result = db_tx.send(DbCommand::InsertResourceUsage {
                        timestamp,
                        cpu_usage,
                        memory_usage,
                        container: Some(container_id.clone()),
                    });

                    if let Err(e) = result {
                        log::error!("Failed to send data of container '{container_id}': {e}");
                    }
                }
                None => {
                    log::warn!("No resource usage data for container '{container_id}'");
                }
            }
        });

    futures_util::future::join_all(container_futures).await;

    Ok(())
}
