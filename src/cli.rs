use std::ffi::OsString;

use failure::Error;
use structopt::clap::AppSettings;
use structopt::StructOpt;
use uuid::Uuid;

#[derive(StructOpt, Debug)]
#[structopt(
    bin_name = "logteewoop",
    raw(
        setting = "AppSettings::ArgRequiredElseHelp",
        global_setting = "AppSettings::UnifiedHelpMessage",
        global_setting = "AppSettings::DeriveDisplayOrder",
        global_setting = "AppSettings::DontCollapseArgsInUsage"
    )
)]
pub struct Opts {
    #[structopt(subcommand)]
    pub command: Command,
}

#[derive(StructOpt, Debug)]
#[structopt(bin_name = "cargo-insta", rename_all = "kebab-case")]
pub enum Command {
    /// Run a logteewoop server
    Serve(ServeCommand),
    /// Run something and stream stdout/stderro to a logteewop server.
    Exec(ExecCommand),
}

#[derive(StructOpt, Debug)]
#[structopt(rename_all = "kebab-case")]
pub struct ServeCommand {
    /// The host to bind to
    #[structopt(long, default_value = "0.0.0.0")]
    host: String,
    /// The port to bind to
    #[structopt(long, default_value = "9002")]
    port: u16,
}

#[derive(StructOpt, Debug)]
#[structopt(
    rename_all = "kebab-case",
    raw(setting = "AppSettings::TrailingVarArg")
)]
pub struct ExecCommand {
    /// The URL of the logteewoop server to report to.
    #[structopt(long, value_name = "SERVER_URL")]
    server: String,
    /// The stream ID to use.
    #[structopt(long, value_name = "STREAM_ID")]
    stream_id: Option<Uuid>,
    /// The command to execute
    #[structopt(parse(from_os_str))]
    args: Vec<OsString>,
}

fn serve_cmd(cmd: &ServeCommand) -> Result<(), Error> {
    use crate::server::serve;
    serve(&cmd.host, cmd.port)
}

fn exec_cmd(cmd: &ExecCommand) -> Result<(), Error> {
    println!("{:#?}", &cmd);
    panic!("not implemented yet");
}

pub fn run() -> Result<(), Error> {
    let opts = Opts::from_args();
    match opts.command {
        Command::Serve(cmd) => serve_cmd(&cmd),
        Command::Exec(cmd) => exec_cmd(&cmd),
    }
}
