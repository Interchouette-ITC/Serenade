//! Run the Trunk version pinned and managed by Serenade.

use anyhow::{Context, bail};

fn main() -> anyhow::Result<()> {
    let args = serenade_cli::trunk::args_from_env();
    if args.first().is_some_and(|arg| arg == "build") {
        let args = args[1..]
            .iter()
            .map(|arg| {
                arg.to_str()
                    .ok_or_else(|| anyhow::anyhow!("Trunk build arguments must be valid UTF-8"))
            })
            .collect::<anyhow::Result<Vec<_>>>()?;
        return serenade_cli::trunk::build(&std::env::current_dir()?, &args);
    }

    let mut command = serenade_cli::trunk::command()?;
    command.args(args);
    let status = command
        .status()
        .context("failed to start Serenade's managed Trunk executable")?;
    if !status.success() {
        bail!("managed Trunk exited with status {status}");
    }
    Ok(())
}
