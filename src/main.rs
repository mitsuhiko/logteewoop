//! logteewoop is a work in progress thing that lets you tee stdout/stderr
//! to a remote logteewoop service.
mod actors;
mod cli;
mod server;

fn main() {
    if let Err(err) = cli::run() {
        println!("error: {}", err);
        std::process::exit(1);
    }
}
