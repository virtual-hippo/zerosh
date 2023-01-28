use nix::unistd::Pid;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum ProcState {
    Run,
    Stop,
}

#[derive(Debug, Clone)]
pub struct ProcInfo {
    pub state: ProcState,
    pub pgid: Pid,
}
