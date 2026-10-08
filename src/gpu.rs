//! NVIDIA GPU usage through `nvidia-smi`. Anything unexpected means "no GPU".

use std::process::Command;

use crate::status::{GpuData, GpuMemory};

pub fn get_info() -> Option<GpuData> {
    let mut command = Command::new("nvidia-smi");
    command.args([
        "--format=csv,noheader,nounits",
        "--query-gpu=name,utilization.gpu,memory.free,memory.total",
    ]);

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }

    let output = command.output().ok()?;
    if !output.status.success() {
        return None;
    }
    parse(&String::from_utf8_lossy(&output.stdout))
}

/// One line per GPU, e.g. `NVIDIA GeForce RTX 3050 Ti Laptop GPU, 0, 3784, 4096`.
/// Only the first GPU is reported.
fn parse(output: &str) -> Option<GpuData> {
    let line = output.lines().find(|l| !l.trim().is_empty())?;
    // The name itself may contain commas, so read the numbers from the right.
    let mut fields = line.rsplitn(4, ',').map(str::trim);
    let total = fields.next()?.parse().ok()?;
    let free = fields.next()?.parse().ok()?;
    let usage = fields.next()?.parse().ok(); // "[N/A]" on some GPUs
    let name = fields.next()?.to_owned();
    Some(GpuData {
        name,
        usage,
        memory: GpuMemory { free, total },
    })
}

#[cfg(test)]
mod tests {
    use super::parse;
    use crate::status::{GpuData, GpuMemory};

    #[test]
    fn parses_a_gpu() {
        assert_eq!(
            parse("NVIDIA GeForce RTX 3050 Ti Laptop GPU, 7, 3784, 4096\n"),
            Some(GpuData {
                name: "NVIDIA GeForce RTX 3050 Ti Laptop GPU".into(),
                usage: Some(7),
                memory: GpuMemory {
                    free: 3784,
                    total: 4096
                },
            })
        );
    }

    #[test]
    fn usage_not_available() {
        let gpu = parse("Tesla K80, [N/A], 11000, 11441\r\n").unwrap();
        assert_eq!(gpu.usage, None);
        assert_eq!(gpu.memory.total, 11441);
    }

    #[test]
    fn first_of_several_gpus() {
        let gpu = parse("GPU A, 1, 2, 3\nGPU B, 4, 5, 6\n").unwrap();
        assert_eq!(gpu.name, "GPU A");
    }

    #[test]
    fn name_with_comma() {
        assert_eq!(
            parse("Vendor, Model X, 50, 100, 200").unwrap().name,
            "Vendor, Model X"
        );
    }

    #[test]
    fn errors_mean_no_gpu() {
        assert_eq!(parse(""), None);
        assert_eq!(
            parse("NVIDIA-SMI has failed because it couldn't communicate with the NVIDIA driver."),
            None
        );
        assert_eq!(parse("GPU, 1, [N/A], [N/A]"), None);
    }
}
