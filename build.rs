use git2::{DescribeFormatOptions, DescribeOptions, Error, Repository};
use std::io;
#[cfg(windows)]
use winres::WindowsResource;

#[cfg_attr(not(windows), allow(clippy::unnecessary_wraps))]
fn main() -> io::Result<()> {
    match get_git_describe_result() {
        Ok(result) => println!("cargo::rustc-env=GIT_DESCRIBE={result}"),
        Err(e) => println!("cargo::warning={e}"),
    }

    // Only for the client's own executable: as a library (pcsc_rs::Monitor) the icon would
    // end up in the program that uses it
    #[cfg(windows)]
    if std::env::var_os("CARGO_FEATURE_CLIENT").is_some() {
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
