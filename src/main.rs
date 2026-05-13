use clap::Parser;

/// CLI for working with the Algolia APIs
#[derive(Parser, Debug)]
#[command(version, about)]
struct Args {
    /// Print more information
    #[arg(short, long)]
    about: bool,
}

fn main() {
    let args = Args::parse();

    if args.about {
        println!("This is the Algolia CLI.");
    }

    println!("It works!");
}
