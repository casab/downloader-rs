use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    #[arg(short, long, num_args=1..)]
    pub links: Vec<String>,

    #[arg(short, long)]
    pub api: bool,
}

pub fn get_args() -> Args {
    Args::parse()
}
