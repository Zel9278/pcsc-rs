use git2::{DescribeFormatOptions, DescribeOptions, Error, Repository};
use std::io;
#[cfg(windows)]
use winres::WindowsResource;

#[cfg_attr(not(windows), allow(clippy::unnecessary_wraps))]
fn main() -> io::Result<()> {
    let target = std::env::var("TARGET").unwrap_or_default();
    let label = arch_label(&target);
    let suffix = if label.is_empty() {
        String::new()
    } else {
        format!(" {label}")
    };
    println!("cargo::rustc-env=PCSC_ARCH_SUFFIX={suffix}");

    match get_git_describe_result() {
        Ok(result) => println!("cargo::rustc-env=GIT_DESCRIBE={result}"),
        Err(e) => println!("cargo::warning={e}"),
    }

    #[cfg(windows)]
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        WindowsResource::new()
            .set_icon("assets/icon.ico")
            .compile()?;
    }
    #[cfg(feature = "xp")]
    thunk::thunk();
    Ok(())
}

fn get_git_describe_result() -> Result<String, Error> {
    let repo = Repository::open(".")?;
    let describe = repo.describe(
        DescribeOptions::new()
            .describe_tags()
            .show_commit_oid_as_fallback(true),
    )?;
    let formatted = describe.format(Some(
        DescribeFormatOptions::new()
            .always_use_long_format(true)
            .dirty_suffix("-dirty"),
    ))?;
    Ok(formatted)
}
fn arch_label(target: &str) -> &'static str {
    match target {
        "aarch64-linux-android" => "ARM64 bionic",
        "armv7-linux-androideabi" => "ARMv7 bionic",
        "armv7-unknown-linux-musleabi" => "ARMv7 musl",
        "arm-unknown-linux-musleabi" => "ARMv6 musl",
        "aarch64-unknown-linux-musl" => "ARM64 musl",
        "x86_64-unknown-linux-musl" => "x86_64 musl",
        "aarch64-apple-darwin" => "macOS ARM64",
        "x86_64-apple-darwin" => "macOS x86_64",
        "x86_64-pc-windows-msvc" => "Windows x64",
        "i686-pc-windows-msvc" => "Windows x86",
        _ => "",
    }
}
