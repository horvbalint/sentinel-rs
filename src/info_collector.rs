use std::time::Duration;

use anyhow::Result;
use bollard::{
    Docker,
    query_parameters::{ListContainersOptions, StatsOptions},
    secret::ContainerCpuStats,
};
use futures_util::StreamExt;
use sysinfo::MINIMUM_CPU_UPDATE_INTERVAL;
use tokio::task::JoinHandle;

use crate::{
    db::{DbCommand, DbCommandChannel},
    host,
    types::{CpuUsage, MemoryUsage},
};

pub fn task(db_tx: DbCommandChannel) -> JoinHandle<Result<()>> {
    tokio::task::spawn(async move {
        let docker = Docker::connect_with_socket_defaults()?;
        let mut host_info_collector = host::InfoCollector::new();

        tokio::time::sleep(MINIMUM_CPU_UPDATE_INTERVAL).await;
        let mut ticker = tokio::time::interval(Duration::from_secs(5));

        loop {
            ticker.tick().await;

            collect_information(&mut host_info_collector, &docker, db_tx.clone()).await?;
            println!("Inserted CPU and memory usage data");
        }
    })
}

async fn collect_information(
    host_info_collector: &mut host::InfoCollector,
    docker: &Docker,
    db_tx: DbCommandChannel,
) -> Result<()> {
    let timestamp = chrono::Local::now().naive_local();

    // host
    host_info_collector.refresh();
    let host_memory_usage = host_info_collector.get_memory_usage();
    let host_total_memory = host_memory_usage.total;

    db_tx.send(DbCommand::InsertResourceUsage {
        timestamp,
        cpu_usage: host_info_collector.get_cpu_usage(),
        memory_usage: host_memory_usage,
        container: None,
    })?;

    // containers
    let containers = docker
        .list_containers(Some(ListContainersOptions::default()))
        .await?;

    let handles = containers.into_iter().map(|container| {
        let container_id = container.id.expect("Container ID should be present");
        let docker = docker.clone();
        let db_tx = db_tx.clone();

        tokio::task::spawn(async move {
            let stat_stream = &mut docker.stats(
                &container_id,
                Some(StatsOptions {
                    stream: false,
                    ..Default::default()
                }),
            );

            let Some(Ok(stats)) = stat_stream.next().await else {
                return None;
            };

            let memory_stats = stats.memory_stats?;
            let total_memory = memory_stats.limit.unwrap_or(host_total_memory);
            let used_memory = memory_stats.usage?;

            db_tx
                .send(DbCommand::InsertResourceUsage {
                    timestamp,
                    cpu_usage: CpuUsage {
                        percentage: calculate_container_cpu_usage(
                            stats.precpu_stats?,
                            stats.cpu_stats?,
                        )?,
                    },
                    memory_usage: MemoryUsage {
                        total: total_memory,
                        used: used_memory,
                        percentage: (used_memory as f64 / total_memory as f64) * 100.,
                    },
                    container: Some(container_id),
                })
                .expect("Failed to send DB command");

            Some(())
        })
    });

    futures_util::future::join_all(handles).await;

    Ok(())
}

fn calculate_container_cpu_usage(
    prev_cpu_stat: ContainerCpuStats,
    cpu_stat: ContainerCpuStats,
) -> Option<f64> {
    let delta_container_cpu_usage =
        cpu_stat.cpu_usage?.total_usage? - prev_cpu_stat.cpu_usage?.total_usage?;
    let delta_system_cpu_usage = cpu_stat.system_cpu_usage? - prev_cpu_stat.system_cpu_usage?;

    Some(
        delta_container_cpu_usage as f64 / delta_system_cpu_usage as f64
            * cpu_stat.online_cpus? as f64
            * 100.0,
    )
}
