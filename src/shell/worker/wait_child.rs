use crate::shell::message::ShellMsg;
use super::job;
use super::process::ProcState;
use super::Worker;
use super::syscall::syscall;

use nix::{
    sys::wait::{waitpid, WaitPidFlag, WaitStatus},
    unistd::Pid,
};
use std::{
    mem::replace,
    process::exit,
    sync::mpsc::SyncSender,
};

pub fn wait_child(worker: &mut Worker, shell_tx: &SyncSender<ShellMsg>) {
    // WUNTRACED: stop child
    // WNOHANG: not block
    // WCONTINUED: re run
    let flag = Some(
        WaitPidFlag::WUNTRACED |
        WaitPidFlag::WNOHANG |
        WaitPidFlag::WCONTINUED
    );

    loop {
        match syscall(|| waitpid(Pid::from_raw(-1), flag)) {
            Ok(WaitStatus::Exited(pid, status)) => {
                worker.exit_val = status;
                process_term(worker, pid, shell_tx);
            }
            Ok(WaitStatus::Signaled(pid, sig, core)) => {
                eprintln!(
                    "\nZeroSh: Child process terminated by signal{}: pid = {pid}, signal = {sig}",
                    if core { "(core dump)" } else { "" }
                );
                worker.exit_val = sig as i32 + 128;
                process_term(worker, pid, shell_tx);
            }
            Ok(WaitStatus::Stopped(pid, _sig)) => process_stop(worker, pid, shell_tx),
            Ok(WaitStatus::Continued(pid)) => process_continue(worker, pid),
            Ok(WaitStatus::StillAlive) => return,
            Err(nix::Error::ECHILD) => return,
            Err(e) => {
                eprintln!("\nZeroSh: Faild to wait: {e}");
                exit(1);
            }
            #[cfg(any(target_os = "linux", target_os = "android"))]
            Ok(WaitStatus::PtraceEvent(pid, _, _) | WaitStatus::PtraceSyscall(pid)) => {
                process_stop(worker, pid, shell_tx)
            }
        }
    }
}

fn process_continue(worker: &mut Worker, pid: Pid) {
    set_pid_state(worker, pid, ProcState::Run);
}

fn process_stop(worker: &mut Worker, pid: Pid, shell_tx: &SyncSender<ShellMsg>) {
    set_pid_state(worker, pid, ProcState::Stop);
    let pgid = worker.pid_to_info.get(&pid).unwrap().pgid;
    let job_id = worker.pgid_to_pids.get(&pgid).unwrap().0;

    job::manage_job(worker, job_id, pgid, shell_tx);
}

fn process_term(worker: &mut Worker, pid: Pid, shell_tx: &SyncSender<ShellMsg>) {
    if let Some((job_id, pgid)) = remove_pid(worker, pid) {
        job::manage_job(worker, job_id, pgid, shell_tx);
    }
}

fn set_pid_state(worker: &mut Worker, pid: Pid, state: ProcState) -> Option<ProcState> {
    let info = worker.pid_to_info.get_mut(&pid)?;
    Some(replace(&mut info.state, state))
}

fn remove_pid(worker: &mut Worker, pid: Pid) -> Option<(usize, Pid)> {
    let pgid = worker.pid_to_info.get(&pid)?.pgid;
    let it = worker.pgid_to_pids.get_mut(&pgid)?;
    it.1.remove(&pid);
    let job_id = it.0;
    Some((job_id, pgid))
}