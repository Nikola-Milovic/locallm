use tokio::process::Command;
use std::process::Stdio;

#[derive(Debug, Clone, Default)]
pub struct GpuStats {
    pub vram_used_mb: u64,
    pub vram_total_mb: u64,
    pub gpu_usage_percent: u8,
    pub temperature_c: Option<u8>,
    pub gpu_name: Option<String>,
}

impl GpuStats {
    pub fn vram_usage_percent(&self) -> f32 {
        if self.vram_total_mb == 0 {
            return 0.0;
        }
        (self.vram_used_mb as f32 / self.vram_total_mb as f32) * 100.0
    }
}

/// Read AMD GPU stats - finds the discrete GPU (highest VRAM)
pub async fn read_amd_gpu_stats() -> Option<GpuStats> {
    // Try all card devices and pick the one with most VRAM (likely discrete GPU)
    let mut best_stats: Option<GpuStats> = None;
    let mut best_vram: u64 = 0;

    for card_num in 0..4 {
        if let Some(stats) = read_card_stats(card_num).await {
            if stats.vram_total_mb > best_vram {
                best_vram = stats.vram_total_mb;
                best_stats = Some(stats);
            }
        }
    }

    // If sysfs didn't work, try rocm-smi
    if best_stats.is_none() {
        best_stats = read_from_rocm_smi().await;
    }

    best_stats
}

async fn read_card_stats(card_num: u32) -> Option<GpuStats> {
    let hwmon_base = format!("/sys/class/drm/card{}/device", card_num);

    // Check if this is an AMD GPU by looking for mem_info_vram_total
    let vram_total_path = format!("{}/mem_info_vram_total", hwmon_base);
    let vram_total = tokio::fs::read_to_string(&vram_total_path)
        .await
        .ok()
        .and_then(|s| s.trim().parse::<u64>().ok())
        .map(|b| b / 1024 / 1024)?; // Convert to MB

    // If VRAM is very small (< 512MB), skip - likely not a discrete GPU
    if vram_total < 512 {
        return None;
    }

    let vram_used = tokio::fs::read_to_string(format!("{}/mem_info_vram_used", hwmon_base))
        .await
        .ok()
        .and_then(|s| s.trim().parse::<u64>().ok())
        .map(|b| b / 1024 / 1024)
        .unwrap_or(0);

    let gpu_usage = tokio::fs::read_to_string(format!("{}/gpu_busy_percent", hwmon_base))
        .await
        .ok()
        .and_then(|s| s.trim().parse::<u8>().ok())
        .unwrap_or(0);

    let temperature = find_gpu_temp(card_num).await;

    Some(GpuStats {
        vram_used_mb: vram_used,
        vram_total_mb: vram_total,
        gpu_usage_percent: gpu_usage,
        temperature_c: temperature,
        gpu_name: None,
    })
}

async fn find_gpu_temp(card_num: u32) -> Option<u8> {
    // Try to find hwmon for this card
    let hwmon_dir = format!("/sys/class/drm/card{}/device/hwmon", card_num);

    if let Ok(mut entries) = tokio::fs::read_dir(&hwmon_dir).await {
        while let Ok(Some(entry)) = entries.next_entry().await {
            let temp_path = entry.path().join("temp1_input");
            if let Ok(temp_str) = tokio::fs::read_to_string(&temp_path).await {
                if let Ok(temp_mc) = temp_str.trim().parse::<u32>() {
                    return Some((temp_mc / 1000) as u8);
                }
            }
        }
    }

    // Fallback: search all hwmon devices for amdgpu
    let hwmon_dir = "/sys/class/hwmon";
    if let Ok(mut entries) = tokio::fs::read_dir(hwmon_dir).await {
        while let Ok(Some(entry)) = entries.next_entry().await {
            let path = entry.path();
            let name_path = path.join("name");
            if let Ok(name) = tokio::fs::read_to_string(&name_path).await {
                if name.trim() == "amdgpu" {
                    let temp_path = path.join("temp1_input");
                    if let Ok(temp_str) = tokio::fs::read_to_string(&temp_path).await {
                        if let Ok(temp_mc) = temp_str.trim().parse::<u32>() {
                            return Some((temp_mc / 1000) as u8);
                        }
                    }
                }
            }
        }
    }

    None
}

async fn read_from_rocm_smi() -> Option<GpuStats> {
    let output = Command::new("rocm-smi")
        .args(["--showmeminfo", "vram", "--json"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .await
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let json_str = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&json_str).ok()?;

    // rocm-smi output format varies, try to extract VRAM info
    let card = json.get("card0")?;
    let vram = card.get("VRAM Total Memory (B)")?.as_u64()? / 1024 / 1024;
    let vram_used = card.get("VRAM Total Used Memory (B)")?.as_u64()? / 1024 / 1024;

    Some(GpuStats {
        vram_used_mb: vram_used,
        vram_total_mb: vram,
        gpu_usage_percent: 0,
        temperature_c: None,
        gpu_name: None,
    })
}
