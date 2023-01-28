mod spawn_sig_handler;
mod message;
mod worker;

use message::{WorkerMsg, ShellMsg};
use worker::Worker;
use spawn_sig_handler::spawn_sig_handler;

use crate::helper::DynError;

use nix::sys::signal::{signal, SigHandler, Signal};
use rustyline::{error::ReadlineError, Editor};
use std::{
    process::exit,
    sync::mpsc::{channel, sync_channel},
};

#[derive(Debug)]
pub struct Shell {
    logfile: String,
}

impl Shell {
    pub fn new(logfile: &str) -> Self {
        Shell {
            logfile: logfile.to_string(),
        }
    }

    // main thread
    pub fn run(&self) -> Result<(), DynError>{
        unsafe {
            signal(Signal::SIGTTOU, SigHandler::SigIgn).unwrap();
        }

        let mut rl = Editor::<()>::new()?;
        if let Err(e) = rl.load_history(&self.logfile) {
            eprintln!("ZeroSh: faild to read the history file: {e}");
        }

        // generate chanel, signal_handler, and woerker thread
        let (worker_tx, worker_rx) = channel();
        let (shell_tx, shell_rx) = sync_channel(0);
        spawn_sig_handler(worker_tx.clone())?;
        Worker::new().spawn(worker_rx, shell_tx);

        let exit_val;
        let mut prev = 0;
        loop {
            let face = if prev == 0 {
                '\u{1F642}'
            } else {
                '\u{1F480}'
            };
            match rl.readline(&format!("ZeroSh {face} %> ")) {
                Ok(line) => {
                    let line_trimed = line.trim();
                    if line_trimed.is_empty() {
                        continue;
                    } else {
                        rl.add_history_entry(line_trimed);
                    }

                    worker_tx.send(WorkerMsg::Cmd(line)).unwrap();
                    match shell_rx.recv().unwrap() {
                        ShellMsg::Continue(n) => prev = n,
                        ShellMsg::Quit(n) => {
                            exit_val = n;
                            break;
                        }
                    }
                }
                Err(ReadlineError::Interrupted) => eprintln!("ZeroSh: Ctrl+D"),
                Err(ReadlineError::Eof) => {
                    worker_tx.send(WorkerMsg::Cmd("exit".to_string())).unwrap();
                    match shell_rx.recv().unwrap() {
                        ShellMsg::Quit(n) => {
                            exit_val = n;
                            break;
                        }
                        _ => panic!("faild to exit"),
                    }
                }
                Err(e) => {
                    eprintln!("ZeroSh: ReadlineError\n{e}");
                    exit_val = 1;
                    break;
                }
            }
        }

        if let Err(e) = rl.save_history(&self.logfile) {
            eprintln!("ZeroSh: faild to write the history file: {e}");
        }
        exit(exit_val);
    }
}
