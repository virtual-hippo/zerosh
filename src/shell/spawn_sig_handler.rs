use crate::helper::DynError;
use crate::shell::message::WorkerMsg;

use signal_hook::{consts::*, iterator::Signals};
use std::{
    sync::mpsc::{Sender},
    thread,
};

// signal_handler thread
pub fn spawn_sig_handler(tx: Sender<WorkerMsg>) -> Result<(), DynError> {
    let mut signals = Signals::new(&[SIGINT, SIGTSTP, SIGCHLD])?;
    thread::spawn(move || {
        for sig in signals.forever() {
            tx.send(WorkerMsg::Signal(sig)).unwrap();
        }
    });
    Ok(())
}