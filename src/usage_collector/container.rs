use bollard::{Docker, query_parameters::StatsOptions, secret::ContainerCpuStats};
use futures_util::StreamExt;

use crate::types::{CpuUsage, MemoryUsage};

pub async fn get_resource_usage(
    docker: &Docker,
    container_id: &str,
) -> Option<(CpuUsage, MemoryUsage)> {
    let stat_stream = &mut docker.stats(
        container_id,
        Some(StatsOptions {
            stream: false,
            ..Default::default()
        }),
    );

    let Some(Ok(stats)) = stat_stream.next().await else {
        return None;
    };

    let memory_stats = stats.memory_stats?;
    let total_memory = memory_stats.limit?;
    let used_memory = memory_stats.usage?;

    Some((
        CpuUsage {
            percentage: calculate_container_cpu_usage(stats.precpu_stats?, stats.cpu_stats?)?,
        },
        MemoryUsage {
            total: total_memory,
            used: used_memory,
            percentage: (used_memory as f64 / total_memory as f64) * 100.,
        },
    ))
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
