pub enum WorkerMsg {
    Signal(i32),
    Cmd(String),
}

pub enum ShellMsg {
    Continue(i32),
    Quit(i32),
}