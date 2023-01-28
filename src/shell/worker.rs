mod built_in_cmd;
mod command;
mod job;
mod process;
mod spawn_child;
mod syscall;
mod wait_child;

use built_in_cmd::built_in_cmd;
use command::{parse_cmd, Command};
use process::ProcInfo;
use spawn_child::spawn_child;
use wait_child::wait_child;
use crate::shell::message::{WorkerMsg, ShellMsg};

use nix::{
    libc,
    unistd::{tcgetpgrp, Pid},
};
use signal_hook::consts::SIGCHLD;
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    sync::mpsc::{Receiver, SyncSender},
    thread,
};

pub type JobId = usize;
#[derive(Debug)]
pub struct Worker{
    exit_val: i32,
    fg: Option<Pid>,
    jobs: BTreeMap<JobId, (Pid, String)>,
    pgid_to_pids: HashMap<Pid, (JobId, HashSet<Pid>)>,
    pid_to_info: HashMap<Pid, ProcInfo>,
    shell_pgid: Pid,
}

impl Worker {
    pub fn new() -> Self {
        Worker{
            exit_val: 0,
            fg: None,
            jobs: BTreeMap::new(),
            pgid_to_pids: HashMap::new(),
            pid_to_info: HashMap::new(),
            shell_pgid: tcgetpgrp(libc::STDIN_FILENO).unwrap(),
        }
    }
    pub fn spawn(mut self, worker_rx: Receiver<WorkerMsg>, shell_tx: SyncSender<ShellMsg>) {
        thread::spawn(move || {
            for msg in worker_rx.iter() {
                self.process_msg(msg, &shell_tx);
            }
        });
    }

    fn process_msg(&mut self, msg: WorkerMsg, shell_tx: &SyncSender<ShellMsg>) {
        match msg {
            WorkerMsg::Cmd(line) => self.process_line(line, shell_tx),
            WorkerMsg::Signal(SIGCHLD) => wait_child(self, &shell_tx),
            _ => unreachable!(),
        }
    }

    fn process_line(&mut self, line: String, shell_tx: &SyncSender<ShellMsg>) {
        match parse_cmd(&line) {
            Ok(cmds) => {
                self.run_cmd(&line, cmds, shell_tx);
            }
            Err(e) => {
                eprintln!("ZeroSh: {e}");
                shell_tx.send(ShellMsg::Continue(self.exit_val)).unwrap();
                return;
            }
        }
    }

    fn run_cmd(&mut self, line: &String, cmds: Vec<Command>, shell_tx: &SyncSender<ShellMsg>) {
        if built_in_cmd(self, &cmds, &shell_tx) {
            return;
        }

        if spawn_child(self, line, &cmds) == false {
            shell_tx.send(ShellMsg::Continue(self.exit_val)).unwrap();
            return;
        }
    }
}