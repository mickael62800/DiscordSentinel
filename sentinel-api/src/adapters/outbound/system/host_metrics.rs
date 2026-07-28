//! Adapter sortant : métriques host (parsing du protocole `INFO` Redis,
//! collecte des disques via snapshot host ou sysinfo). Le handler HTTP
//! (`system/info.rs`) ne fait plus que l'assemblage et le mapping DTO.

use sysinfo::Disks;

/// Métriques Redis extraites de la sortie `INFO`.
#[derive(Debug, Default, Clone)]
pub struct RedisMetrics {
    pub used_memory_mb: u64,
    pub connected_clients: u64,
    pub total_keys: u64,
    pub uptime_seconds: u64,
}

/// Parse la sortie de `INFO` Redis (format "key:value" par ligne) et
/// extrait les champs qui nous interessent.
pub fn parse_redis_info(raw: &str) -> RedisMetrics {
    let mut m = RedisMetrics::default();
    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((k, v)) = line.split_once(':') else {
            continue;
        };
        match k {
            "used_memory" => {
                if let Ok(bytes) = v.parse::<u64>() {
                    m.used_memory_mb = bytes / 1024 / 1024;
                }
            }
            "connected_clients" => {
                m.connected_clients = v.parse().unwrap_or(0);
            }
            "uptime_in_seconds" => {
                m.uptime_seconds = v.parse().unwrap_or(0);
            }
            k if k.starts_with("db") => {
                // Ex: "db0:keys=1234,expires=56,avg_ttl=789"
                if let Some(keys_part) = v.split(',').find(|p| p.starts_with("keys=")) {
                    if let Some(n) = keys_part.strip_prefix("keys=") {
                        m.total_keys += n.parse::<u64>().unwrap_or(0);
                    }
                }
            }
            _ => {}
        }
    }
    m
}

/// Etat d'un disque / point de montage.
#[derive(Debug, Clone)]
pub struct DiskInfo {
    pub name: String,
    pub mount_point: String,
    pub fs_type: String,
    pub total_gb: f64,
    pub used_gb: f64,
    pub available_gb: f64,
    pub usage_percent: f32,
    pub is_removable: bool,
}

fn bytes_to_gb(bytes: u64) -> f64 {
    (bytes as f64) / (1024.0 * 1024.0 * 1024.0)
}

/// Collecte les disques : en priorite le snapshot host
/// `/var/lib/sentinel/disks-current.json` (genere par le cron
/// `sentinel-disk-trend.sh`, expose TOUS les disques physiques meme depuis le
/// container), sinon fallback sysinfo (vue container, filtre des fs virtuels).
pub fn collect_disks() -> Vec<DiskInfo> {
    read_host_disks_snapshot().unwrap_or_else(|| {
        let disks_info = Disks::new_with_refreshed_list();
        disks_info
            .iter()
            .filter(|d| {
                let fs = d.file_system().to_string_lossy();
                !matches!(
                    fs.as_ref(),
                    "overlay" | "shm" | "tmpfs" | "devtmpfs" | "proc" | "sysfs"
                ) || d.total_space() > 100 * 1024 * 1024
            })
            .map(|d| {
                let total = d.total_space();
                let avail = d.available_space();
                let used = total.saturating_sub(avail);
                let usage = if total > 0 {
                    ((used as f64 / total as f64) * 100.0) as f32
                } else {
                    0.0
                };
                DiskInfo {
                    name: d.name().to_string_lossy().into_owned(),
                    mount_point: d.mount_point().to_string_lossy().into_owned(),
                    fs_type: d.file_system().to_string_lossy().into_owned(),
                    total_gb: bytes_to_gb(total),
                    used_gb: bytes_to_gb(used),
                    available_gb: bytes_to_gb(avail),
                    usage_percent: usage,
                    is_removable: d.is_removable(),
                }
            })
            .collect()
    })
}

/// Lit /var/lib/sentinel/disks-current.json (genere par le cron host
/// `sentinel-disk-trend.sh`). Format :
/// `{"updated_at":"...","disks":[{"timestamp":...,"mount":"/","used_gb":N,"total_gb":N,"usage_pct":N},...]}`
///
/// Retourne None si le fichier n'existe pas / est illisible / mal forme.
/// L'appelant fallback alors sur sysinfo (vue container).
fn read_host_disks_snapshot() -> Option<Vec<DiskInfo>> {
    use serde::Deserialize;

    #[derive(Deserialize)]
    struct HostDisk {
        mount: String,
        #[serde(default)]
        used_gb: f64,
        #[serde(default)]
        total_gb: f64,
        #[serde(default)]
        usage_pct: f32,
    }
    #[derive(Deserialize)]
    struct HostDisks {
        disks: Vec<HostDisk>,
    }

    let raw = std::fs::read_to_string("/var/lib/sentinel/disks-current.json").ok()?;
    let parsed: HostDisks = serde_json::from_str(&raw).ok()?;
    if parsed.disks.is_empty() {
        return None;
    }
    Some(
        parsed
            .disks
            .into_iter()
            .map(|d| DiskInfo {
                name: d.mount.clone(),
                mount_point: d.mount,
                fs_type: "host".to_string(),
                total_gb: d.total_gb,
                used_gb: d.used_gb,
                available_gb: (d.total_gb - d.used_gb).max(0.0),
                usage_percent: d.usage_pct,
                is_removable: false,
            })
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_redis_info_handles_basic_fields() {
        let raw = "# Memory\nused_memory:1048576\nconnected_clients:42\nuptime_in_seconds:300\n";
        let m = parse_redis_info(raw);
        assert_eq!(m.used_memory_mb, 1);
        assert_eq!(m.connected_clients, 42);
        assert_eq!(m.uptime_seconds, 300);
    }

    #[test]
    fn parse_redis_info_sums_keys_across_dbs() {
        let raw = "db0:keys=100,expires=10,avg_ttl=0\ndb1:keys=50,expires=5,avg_ttl=0\n";
        let m = parse_redis_info(raw);
        assert_eq!(m.total_keys, 150);
    }

    #[test]
    fn parse_redis_info_ignores_comments_and_blank_lines() {
        let raw = "\n# Server\n\nused_memory:2097152\n# more comments\n";
        let m = parse_redis_info(raw);
        assert_eq!(m.used_memory_mb, 2);
    }

    #[test]
    fn parse_redis_info_ignores_unknown_fields() {
        let raw = "some_other_field:xyz\nused_memory:1048576\n";
        let m = parse_redis_info(raw);
        assert_eq!(m.used_memory_mb, 1);
        assert_eq!(m.connected_clients, 0);
    }

    #[test]
    fn parse_redis_info_handles_malformed_values_gracefully() {
        let raw = "connected_clients:not_a_number\nused_memory:also_bad\n";
        let m = parse_redis_info(raw);
        assert_eq!(m.connected_clients, 0);
        assert_eq!(m.used_memory_mb, 0);
    }

    #[test]
    fn parse_redis_info_empty_input() {
        let m = parse_redis_info("");
        assert_eq!(m.used_memory_mb, 0);
        assert_eq!(m.connected_clients, 0);
        assert_eq!(m.uptime_seconds, 0);
        assert_eq!(m.total_keys, 0);
    }

    #[test]
    fn parse_redis_info_db_without_keys_prefix_ignored() {
        let raw = "db0:expires=10,avg_ttl=0\n";
        let m = parse_redis_info(raw);
        assert_eq!(m.total_keys, 0);
    }

    #[test]
    fn parse_redis_info_used_memory_rounds_down() {
        // 1.5 Mo = 1 572 864 bytes → 1 Mo (division entiere)
        let raw = "used_memory:1572864\n";
        let m = parse_redis_info(raw);
        assert_eq!(m.used_memory_mb, 1);
    }
}
