use crate::shell::message::ShellMsg;

use super::command::Command;
use super::job::is_group_stop;
use super::Worker;

use nix::{
    libc,
    sys::signal::{killpg, Signal},
    unistd::tcsetpgrp,
};
use std::{
    env,
    path::PathBuf,
    sync::mpsc::SyncSender,
};

pub fn built_in_cmd(worker: &mut Worker, cmd: &[Command], shell_tx: &SyncSender<ShellMsg>) -> bool {
    if cmd.len() > 1 {
        return false;
    }

    match cmd[0].filename {
        "exit" => run_exit(worker, &cmd[0].args, shell_tx),
        "jobs" => run_jobs(worker, shell_tx),
        "fg" => run_fg(worker, &cmd[0].args, shell_tx),
        "cd" => run_cd(worker, &cmd[0].args, shell_tx),
        _ => false,
    }
}

fn run_exit(worker: &mut Worker, args: &[&str], shell_tx: &SyncSender<ShellMsg>) -> bool {
    if worker.jobs.is_empty() == false {
        eprintln!("Couludn't exit because some jobs are running");
        worker.exit_val = 1;
        shell_tx.send(ShellMsg::Continue(worker.exit_val)).unwrap();
        return true;
    }
    let exit_val = if let Some(s) = args.get(1) {
        if let Ok(n) = (*s).parse::<i32>() {
            n
        } else {
            eprintln!("\"{s}\" is an invalid argument");
            worker.exit_val = 1;
            shell_tx.send(ShellMsg::Continue(worker.exit_val)).unwrap();
            return true;
        }
    } else {
        worker.exit_val
    };
    shell_tx.send(ShellMsg::Quit(exit_val)).unwrap();
    true
}

fn run_jobs(worker: &mut Worker, shell_tx: &SyncSender<ShellMsg>) -> bool {
    for (job_id, (pg_id, cmd)) in worker.jobs.iter() {
        let state = if is_group_stop(worker, *pg_id).unwrap() {
            "Stopped"
        } else {
            "Running"
        };
        eprintln!("[{job_id}] {state}\t{cmd}");
    }
    worker.exit_val = 0;
    shell_tx.send(ShellMsg::Continue(worker.exit_val)).unwrap();
    true
}

fn run_fg(worker: &mut Worker, args: &[&str], shell_tx: &SyncSender<ShellMsg>) -> bool {
    worker.exit_val = 1;

    if args.len() < 2 {
        eprintln!("Usage: fg integer");
        shell_tx.send(ShellMsg::Continue(worker.exit_val)).unwrap();
        return true;
    }

    if let Ok(n) = args[1].parse::<usize>() {
        if let Some((pgid, cmd_string)) = worker.jobs.get(&n) {
            eprintln!("[{n}] Restart\t{cmd_string}");
            worker.fg = Some(*pgid);
            tcsetpgrp(libc::STDIN_FILENO, *pgid).unwrap();

            killpg(*pgid, Signal::SIGINT).unwrap();
            return true;
        }
    }

    eprintln!("Not found {}", args[1]);
    shell_tx.send(ShellMsg::Continue(worker.exit_val)).unwrap();
    true
}

fn run_cd(worker: &mut Worker, args: &[&str], shell_tx: &SyncSender<ShellMsg>) -> bool {
    let path = if args.len() == 1 {
        dirs::home_dir()
            .or_else(|| Some(PathBuf::from("/")))
            .unwrap()
    } else {
        PathBuf::from(args[1])
    };

    if let Err(e) = env::set_current_dir(&path) {
        worker.exit_val = 1;
        eprintln!("Faild to \"cd\"; {e}");
    } else {
        worker.exit_val = 0;
    }

    shell_tx.send(ShellMsg::Continue(worker.exit_val)).unwrap();
    true
}
