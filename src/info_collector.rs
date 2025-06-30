use std::time::Duration;

use bollard::{
    Docker,
    query_parameters::{ListContainersOptionsBuilder, StatsOptions},
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

pub fn task(db_tx: DbCommandChannel) -> JoinHandle<()> {
    tokio::task::spawn(async move {
        let docker = Docker::connect_with_socket_defaults().unwrap();
        let mut host_info_collector = host::InfoCollector::new();
        tokio::time::sleep(MINIMUM_CPU_UPDATE_INTERVAL).await;

        loop {
            let timestamp = chrono::Local::now().naive_local();

            // host
            host_info_collector.refresh();
            let host_memory_usage = host_info_collector.get_memory_usage();
            let host_total_memory = host_memory_usage.total;

            db_tx
                .send(DbCommand::InsertResourceUsage {
                    timestamp,
                    cpu_usage: host_info_collector.get_cpu_usage(),
                    memory_usage: host_memory_usage,
                    container: None,
                })
                .expect("Failed to send DB command");

            // containers
            let containers = docker
                .list_containers(Some(ListContainersOptionsBuilder::new().all(false).build()))
                .await
                .unwrap();

            let handles = containers.into_iter().map(|container| {
                let docker = docker.clone();
                let db_tx = db_tx.clone();

                tokio::task::spawn(async move {
                    let stat_stream = &mut docker.stats(
                        &container.id.clone()?,
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
                    let used_memory = memory_stats.usage.unwrap();

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
                            container: Some(container.id?.clone()),
                        })
                        .expect("Failed to send DB command for container stats");

                    Some(())
                })
            });

            futures_util::future::join_all(handles).await;
            println!("Inserted CPU and memory usage data");

            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    })
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
