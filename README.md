![Banner](./visual/banner.png)

This command line program schedules commands for execution in an interactive shell with cron expressions. It will keep
executing the provided command until interrupted or until specified conditions are met.

As simple as

`cronthat "* * * * * *" -- echo are we there yet "?"`
> Prints "are we there yet ?" every second.

`cronthat "0 */5 12 * * *" --until "2030-01-13 00:00:00" -- count-sheep`
> Count sheep every five minutes between 12:00 PM and 12:59 PM and stop in 2030 (you can still `CTRL-C` if you find sleep).

`cronthat "0 0 2 * * *" --now -- ./scripts/etl.sh`
> Execute an ETL script every day at 2 PM and force a first execution now.

There are a few other tricks but `cronthat` is really not that complicated and the `--help` flag should be enough.

```text
$ cronthat --help
Schedule a command with a CRON expression until interruption.

Usage: cronthat [OPTIONS] <CRON_EXPRESSION> [COMMAND]...

Arguments:
  <CRON_EXPRESSION>  Cron expression to schedule your command, you can use tools like https://crontab.cronhub.io/ to help you. Precision up to the second
  [COMMAND]...       Command to run

Options:
  -e, --stop-on-error              Stop when the command returns a non-zero exit code
  -n, --repetitions <REPETITIONS>  Number of times the command should be executed (mutually exclusive with --until)
  -u, --until <UNTIL>              When to stop (mutually exclusive with --repetitions)
  -w, --now                        Schedule a first execution immediately
  -h, --help                       Print help
  -V, --version                    Print version
```

# Installation

## From source

`cronthat` is written in rust, so you can just run `cargo install --git https://github.com/yannduperis/cronthat.git cronthat` 
while I manage to get releases and packages (crates.io, AUR and Brew).

## Release

🚧 Not there yet

## Package

### AUR

🚧 Not there yet

### Brew

🚧 Not there yet


# Why ?

I needed a command to be reliably executed every day (while away from my computer). For such a single-use case, I didn't 
want the hassle of a cron task, but I wanted that sweet syntax for scheduled execution. Believe it or not, I couldn't
find a simple tool to start "single-use" cron tasks that I can see live in my terminal and terminate with 
`CTRL-C. Now there is one.