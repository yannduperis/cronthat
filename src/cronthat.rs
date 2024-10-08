use anyhow::{bail, Context, Result};
use chrono::{DateTime, Duration, Local, NaiveDateTime};
use clap::Parser;
use cron::Schedule;
use run_script::types::IoOptions;
use run_script::ScriptOptions;
use std::str::FromStr;
use std::thread::sleep;

static DATETIME_FORMAT: &'static str = "%Y-%m-%d %H:%M:%S";

/// Schedule commands for execution in an interactive shell with cron expressions. It will keep
/// executing the provided command until interrupted or until specified conditions are met.
#[derive(Parser)]
#[command(version, about, author)]
pub struct CronThat {
    /// Cron expression to schedule your command, you can use tools like https://crontab.cronhub.io/ to help you.
    /// Precision up to the second.
    cron_expression: String,

    /// Command to run
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    command: Vec<String>,

    /// Stop when the command returns a non-zero exit code.
    #[arg(short = 'e', long)]
    stop_on_error: bool,

    /// Number of times the command should be executed (mutually exclusive with --until)
    #[clap(short('n'), long)]
    repetitions: Option<usize>,

    /// When to stop (mutually exclusive with --repetitions)
    #[clap(short, long, value_parser = parse_date_time)]
    until: Option<DateTime<Local>>,

    /// Schedule a first execution immediately
    #[clap(short('w'), long)]
    now: bool,
}

fn parse_date_time(value: &str) -> Result<DateTime<Local>> {
    let dt = NaiveDateTime::parse_from_str(value, DATETIME_FORMAT)?
        .and_local_timezone(Local::now().timezone())
        .single()
        .context("cannot parse with timezone")?;
    Ok(dt)
}

impl CronThat {
    pub fn execute(&self) -> Result<()> {
        self.check_args()?;
        let schedule =
            Schedule::from_str(&self.cron_expression).context("invalid cron expression")?;

        if self.now {
            self.spawn_command()?;
        }

        for (i, datetime) in schedule
            .upcoming(Local::now().timezone())
            .into_iter()
            .enumerate()
        {
            if self.must_stop(i) {
                break;
            }

            let now: DateTime<Local> = Local::now();
            let wait = datetime.signed_duration_since(now);
            let succeeded = if wait > Duration::zero() {
                sleep(wait.to_std()?);
                self.spawn_command()?
            } else {
                self.spawn_command()?
            };

            if !succeeded {
                if self.stop_on_error {
                    bail!("command exited with non-zero status code");
                } else {
                    println!("warning: command exited with non-zero status code");
                    println!();
                }
            }
        }

        Ok(())
    }

    fn spawn_command(&self) -> Result<bool> {
        println!("{} -- Spawning command", Local::now());
        let mut options = ScriptOptions::new();
        options.output_redirection = IoOptions::Inherit;
        let (status, _, _) = run_script::run(self.command.join(" ").as_str(), &vec![], &options)?;
        Ok(status == 0)
    }

    fn check_args(&self) -> Result<()> {
        if self.repetitions.is_some() && self.until.is_some() {
            bail!("--repetitions and --until are mutually exclusive");
        }

        if self.command.is_empty() {
            bail!("no command to execute");
        }

        Ok(())
    }

    fn must_stop(&self, i: usize) -> bool {
        if self.repetitions.is_none() && self.until.is_none() {
            false
        } else {
            if self.repetitions.is_some() {
                i >= self.repetitions.unwrap()
            } else {
                Local::now() > self.until.unwrap()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::cronthat::{CronThat, DATETIME_FORMAT};
    use chrono::{Local, TimeDelta};
    use clap::Parser;
    use std::fs::File;
    use std::io;
    use std::ops::Add;
    use tokio::task::spawn_blocking;
    use tokio::time::timeout;

    static CRON_EVERY_S: &'static str = "* * * * * *";

    #[test]
    fn cronthat_parse_command() {
        let cli =
            CronThat::try_parse_from(vec!["cronthat", CRON_EVERY_S, "--", "echo", "hello-world"])
                .unwrap();
        assert_eq!(cli.command, vec!["echo", "hello-world"]);
    }

    #[tokio::test]
    async fn cronthat_execute_limited_repetitions() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let tmp_path = tmp.path().to_path_buf();

        let timeout_duration = tokio::time::Duration::from_secs(2);
        timeout(timeout_duration, async {
            let tmp_path = tmp_path.clone();
            spawn_blocking(move || {
                let tmp_path = tmp_path.clone();
                let cli = CronThat::try_parse_from(vec![
                    "cronthat",
                    CRON_EVERY_S,
                    "--repetitions",
                    "2",
                    "--",
                    &format!("echo helloworld >> {:?}", tmp_path),
                ])
                .unwrap();
                cli.execute().unwrap();
            })
            .await
            .unwrap();
        })
        .await
        .expect("timed out");

        let content = io::read_to_string(File::open(tmp_path).unwrap()).unwrap();
        assert_eq!(content, "helloworld\nhelloworld\n");
    }

    #[tokio::test]
    async fn cronthat_execute_until() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let tmp_path = tmp.path().to_path_buf();

        let now = Local::now();
        let in_three_seconds = now.add(TimeDelta::seconds(3));
        let until = in_three_seconds.format(DATETIME_FORMAT).to_string();

        let timeout_duration = tokio::time::Duration::from_secs(3);
        timeout(timeout_duration, async {
            let tmp_path = tmp_path.clone();
            spawn_blocking(move || {
                let tmp_path = tmp_path.clone();
                let cli = CronThat::try_parse_from(vec![
                    "cronthat",
                    CRON_EVERY_S,
                    "--until",
                    until.as_str(),
                    "--",
                    &format!("echo helloworld >> {:?}", tmp_path),
                ])
                .unwrap();
                cli.execute().unwrap();
            })
            .await
            .unwrap();
        })
        .await
        .expect("timed out");

        let content = io::read_to_string(File::open(tmp_path).unwrap()).unwrap();
        assert_eq!(content, "helloworld\nhelloworld\nhelloworld\n");
    }

    #[tokio::test]
    async fn cronthat_execute_error() {
        let timeout_duration = tokio::time::Duration::from_secs(2);

        // Default to ignore errors
        timeout(timeout_duration.clone(), async {
            spawn_blocking(move || {
                let cli = CronThat::try_parse_from(vec![
                    "cronthat",
                    CRON_EVERY_S,
                    "--repetitions",
                    "2",
                    "--",
                    "exit",
                    "1",
                ])
                .unwrap();
                cli.execute().unwrap();
            })
            .await
            .unwrap();
        })
        .await
        .expect("timed out");

        // Stop on errors
        timeout(timeout_duration.clone(), async {
            spawn_blocking(move || {
                let cli = CronThat::try_parse_from(vec![
                    "cronthat",
                    CRON_EVERY_S,
                    "--stop-on-error",
                    "--",
                    "exit",
                    "1",
                ])
                .unwrap();
                cli.execute().expect_err("must stop on error");
            })
            .await
            .unwrap();
        })
        .await
        .expect("timed out");
    }

    #[tokio::test]
    async fn cronthat_execute_now() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let tmp_path = tmp.path().to_path_buf();

        let timeout_duration = tokio::time::Duration::from_millis(1500);
        let in_one_second = Local::now().add(TimeDelta::seconds(1));

        // Default to ignore errors
        timeout(timeout_duration.clone(), async {
            let tmp_path = tmp_path.clone();
            spawn_blocking(move || {
                let cli = CronThat::try_parse_from(vec![
                    "cronthat",
                    CRON_EVERY_S,
                    "--now",
                    "--until",
                    &in_one_second.format(DATETIME_FORMAT).to_string(),
                    "--",
                    &format!("echo helloworld >> {:?}", tmp_path),
                ])
                .unwrap();
                cli.execute().unwrap();
            })
            .await
            .unwrap();
        })
        .await
        .expect("timed out");

        let content = io::read_to_string(File::open(tmp_path).unwrap()).unwrap();
        assert_eq!(content, "helloworld\nhelloworld\n");
    }
}
