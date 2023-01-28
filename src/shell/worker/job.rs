use crate::shell::message::ShellMsg;
use super::process::{ProcInfo, ProcState};
use super::{Worker, JobId};

use nix::{libc, unistd::{tcsetpgrp, Pid}};
use std::{
    collections::{HashMap, HashSet},
    sync::mpsc::SyncSender,
};

pub fn insert_job(worker: &mut Worker, job_id: JobId, pgid: Pid, pids: HashMap<Pid, ProcInfo>, line: &str) {
    assert!(!worker.jobs.contains_key(&job_id));
    worker.jobs.insert(job_id, (pgid, line.to_string()));

    let mut procs = HashSet::new();
    for (pid, info) in pids {
        procs.insert(pid);

        assert!(!worker.pid_to_info.contains_key(&pid));
        worker.pid_to_info.insert(pid, info);
    }

    assert!(!worker.pgid_to_pids.contains_key(&pgid));
    worker.pgid_to_pids.insert(pgid, (job_id, procs));
}

pub fn get_new_job_id(worker: &Worker) -> Option<JobId> {
    for i in 0..=usize::MAX{
        if worker.jobs.contains_key(&i) == false {
            return Some(i)
        }
    }
    None
}

pub fn manage_job(worker: &mut Worker, job_id: JobId, pgid: Pid, shell_tx: &SyncSender<ShellMsg>) {
    let is_fg = worker.fg.map_or(false, |x| pgid == x);
    if is_fg { 
        manage_job_when_is_fg(worker, job_id, pgid, shell_tx);
    } else {
        manage_job_when_is_not_fg(worker, job_id, pgid);
    }
}

fn manage_job_when_is_fg(worker: &mut Worker, job_id: JobId, pgid: Pid, shell_tx: &SyncSender<ShellMsg>) {
    let line = &(worker.jobs.get(&job_id).unwrap().1);
    if is_group_empty(&worker.pgid_to_pids, pgid) {
        eprintln!("[{job_id}] Terminated \t{line}");
        remove_job(worker, job_id);
        set_shell_fg(worker, shell_tx);
    } else if is_group_stop(worker, pgid).unwrap() {
        eprintln!("[{job_id}] Stopped \t{line}");
        set_shell_fg(worker, shell_tx);
    }
}

fn manage_job_when_is_not_fg(worker: &mut Worker, job_id: JobId, pgid: Pid) {
    let line = &(worker.jobs.get(&job_id).unwrap().1);
    if is_group_empty(&worker.pgid_to_pids, pgid) {
        eprintln!("[{job_id}] Terminated \t{line}");
        remove_job(worker, job_id);
    }
}

fn remove_job(worker: &mut Worker, job_id: JobId) {
    if let Some((pgid, _)) = worker.jobs.remove(&job_id) {
        if let Some((_, pids)) = worker.pgid_to_pids.remove(&pgid) {
            assert!(pids.is_empty());
        }
    }
}

fn is_group_empty(pgid_to_pids: &HashMap<Pid, (JobId, HashSet<Pid>)>, pgid: Pid) -> bool {
    pgid_to_pids.get(&pgid).unwrap().1.is_empty()
}

pub fn is_group_stop(worker: &Worker, pgid: Pid) -> Option<bool> {
    for pid in worker.pgid_to_pids.get(&pgid)?.1.iter() {
        if worker.pid_to_info.get(pid).unwrap().state == ProcState::Run {
            return Some(false);
        }
    }
    Some(true)
}

fn set_shell_fg(worker: &mut Worker, shell_tx: &SyncSender<ShellMsg>) {
    worker.fg = None;
    tcsetpgrp(libc::STDIN_FILENO, worker.shell_pgid).unwrap();
    shell_tx.send(ShellMsg::Continue(worker.exit_val)).unwrap();
}