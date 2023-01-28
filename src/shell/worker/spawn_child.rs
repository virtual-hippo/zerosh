mod helper;

use crate::DynError;

use super::command::Command;
use super::job;
use super::process::{ProcState, ProcInfo};
use helper::*;
use super::syscall::syscall;
use super::Worker;

use nix::{
    libc, unistd::{
        self, dup2, execvp, fork, pipe, setpgid, tcsetpgrp, ForkResult, Pid
    }
};
use std::{
    collections::HashMap,
    process::exit,
    ffi::CString,
};
    

pub fn spawn_child(worker: &mut Worker, line: &str, cmds : &[Command]) -> bool{
    assert_ne!(cmds.len(), 0);

    if cmds.len() > 2 {
        return eprintln_and_return_false("ZeroSh: Unsupported pipe used more than 3 commands");
    }

    let job_id = if let Some(job_id) = job::get_new_job_id(worker) {
        job_id
    } else {
        return eprintln_and_return_false("ZeroSh: Jobs created up to the limit");
    };

    let mut input = None;
    let mut output = None;
    if cmds.len() > 1 {
        let p = pipe().unwrap();
        input = Some(p.0);
        output = Some(p.1);
    }

    let cleanup_pipe = CleanUp::new(
        || {
            if let Some(fd) = input {
                syscall(|| unistd::close(fd)).unwrap();
            }
            if let Some(fd) = output {
                syscall(|| unistd::close(fd)).unwrap();
            }
        }
    );

    let pgid;
    match fork_exec(Pid::from_raw(0), &cmds[0], None, output) {
        Ok(child) => pgid = child,
        Err(e) => {
            return eprintln_and_return_false(&format!("ZeroSh: Error on generate process {e}"));
        }
    }

    let info = ProcInfo {
        state: ProcState::Run,
        pgid,
    };
    let mut pids = HashMap::new();
    pids.insert(pgid, info.clone());

    if cmds.len() == 2 {
        match fork_exec(pgid, &cmds[1], input, None) {
            Ok(child) => {
                pids.insert(child, info);
            },
            Err(e) => {
                return eprintln_and_return_false(&format!("ZeroSh: Error on generate process {e}"));
            }
        }
    }
    std::mem::drop(cleanup_pipe);

    worker.fg = Some(pgid);
    job::insert_job(worker, job_id, pgid, pids, line);
    tcsetpgrp(libc::STDIN_FILENO, pgid).unwrap();

    true
}

fn fork_exec(
    pgid: Pid,
    cmd: &Command,
    input: Option<i32>,
    output: Option<i32>,
) -> Result<Pid, DynError>{
    let filename = CString::new(cmd.filename).unwrap();
    let args: Vec<CString> = cmd.args.iter().map(|s| CString::new(*s).unwrap()).collect();

    match syscall(|| unsafe { fork() })? {
        ForkResult::Parent { child, ..} => {
            setpgid(child, pgid).unwrap();
            Ok(child)
        }
        ForkResult::Child => {
            setpgid(Pid::from_raw(0), pgid).unwrap();

            if let Some(infd) = input {
                syscall(|| dup2(infd, libc::STDIN_FILENO)).unwrap();
            }
            if let Some(outfd) = output {
                syscall(|| dup2(outfd, libc::STDOUT_FILENO)).unwrap();
            }

            // close UNIX domain socket and pipe these used on signal_hook
            for i in 3..7 {
                let _ = syscall(|| unistd::close(i));
            }

            match execvp(&filename, &args) {
                Err(_) => {
                    unistd::write(libc::STDERR_FILENO, "executed unknown command\n".as_bytes()).ok();
                    exit(1);
                }
                Ok(_) => unreachable!(),
            }
        }
    }
}
