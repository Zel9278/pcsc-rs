use regex::Regex;

use crate::status::{GpuData, GpuMemory};
use std::process::Command;

pub fn get_info() -> Option<GpuData> {
    let output = Command::new("nvidia-smi")
        .args([
            "--format=csv",
            "--query-gpu=name,utilization.gpu,memory.free,memory.total",
        ])
        .output();
    if output.is_err() {
        return None;
    } else {
        let res = output.expect("process error");

        let split_seperator = Regex::new(r"\r\n|\n").expect("Invalid regex");
        let split_binding = String::from_utf8(res.stdout).unwrap();
        let splited: Vec<_> = split_seperator.split(&split_binding).into_iter().collect();

        let replace_seperator = Regex::new(r" %| MiB| GiB|\r").expect("Invalid regex");
        let split2_seperator = Regex::new(r", ").expect("Invalid regex");
        let replaced =
            replace_seperator.replace_all(splited.get(1).expect("not found at index 1"), "");
        let splited2: Vec<_> = split2_seperator.split(&replaced).into_iter().collect();

        let usage: Option<u64> = match splited2[1] {
            "[N/A]" => None,
            _ => Some(splited2[1].to_string().parse::<u64>().unwrap()),
        };

        let result = Some(GpuData {
            name: splited2[0].into(),
            usage,
            memory: GpuMemory {
                free: splited2[2].to_string().parse::<u64>().unwrap(),
                total: splited2[3].to_string().parse::<u64>().unwrap(),
            },
        });

        return result;
    };
}
