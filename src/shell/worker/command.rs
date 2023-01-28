use crate::helper::DynError;

pub struct Command<'a>{
    pub filename: &'a str,
    pub args: Vec<&'a str>,
}
type CmdResult<'a> = Result<Vec<Command<'a>>, DynError>;

fn parse_cmd_one(line: &str) -> Result<Command, DynError> {
    let line_splited: Vec<&str> = line.split(' ').collect();
    let mut filename = "";
    let mut args = Vec::new();
    for (n, s) in line_splited.iter().filter(|s| !s.is_empty()).enumerate() {
        if n == 0 {
            filename= *s;
        }
        args.push(*s);
    }
    if filename.is_empty() {
        Err("Blank Command".into())
    } else {
        Ok(Command{filename, args})
    }
}

fn parse_pipe(line: &str) ->  Vec<&str> {
    let cmds: Vec<&str> = line.split('|').collect();
    cmds
}

pub fn parse_cmd(line: &str) -> CmdResult {
    let cmds = parse_pipe(line);
    if cmds.is_empty() {
        return Err("Blank Command".into());
    }
    let mut result = Vec::new();
    for cmd in cmds {
        let command = parse_cmd_one(cmd)?;
        result.push(command);
    }
    Ok(result)
}