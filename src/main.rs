use std::io::{self, IsTerminal};
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;

use anyhow::{Result, bail};
use clap::Parser;

use fdiff::diff::DiffEngine;

#[derive(Debug, Parser)]
#[command(
    version,
    about = "Explore folder-level differences in a live terminal UI"
)]
struct Args {
    /// 비교할 왼쪽 디렉터리
    left: PathBuf,

    /// 비교할 오른쪽 디렉터리
    right: PathBuf,

    /// TUI 대신 한 번의 plain text snapshot을 출력
    #[arg(long)]
    plain: bool,

    /// 차이가 있으면 exit code 1을 반환하는 automation mode
    #[arg(long)]
    check: bool,

    /// 동일한 항목도 함께 표시
    #[arg(short, long)]
    all: bool,

    /// live scan 간격
    #[arg(long, default_value_t = 750)]
    interval_ms: u64,
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("fdiff: {error:#}");
            ExitCode::from(2)
        }
    }
}

fn run() -> Result<ExitCode> {
    let args = Args::parse();
    if args.interval_ms < 100 {
        bail!("--interval-ms must be at least 100");
    }

    let mut engine = DiffEngine::new(&args.left, &args.right)?;
    let report = engine.scan()?;
    let interactive =
        !args.plain && !args.check && io::stdin().is_terminal() && io::stdout().is_terminal();

    if interactive {
        fdiff::tui::run(
            &mut engine,
            report,
            Duration::from_millis(args.interval_ms),
            args.all,
        )?;
        return Ok(ExitCode::SUCCESS);
    }

    print!("{}", fdiff::output::render_plain(&report, args.all));
    if args.check && report.has_differences() {
        Ok(ExitCode::from(1))
    } else {
        Ok(ExitCode::SUCCESS)
    }
}
