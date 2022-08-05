use std::path::PathBuf;

use anyhow::Context;
use clap::Parser;

#[derive(Parser)]
struct Opts {
    file: PathBuf,
    page: u32,
}

fn main() -> anyhow::Result<()> {
    let opts = Opts::parse();
    let file = pdf::file::File::open(&opts.file)?;
    let page = file.get_page(opts.page)?;
    let contents = page
        .contents
        .as_ref()
        .context("The page does not have contents")?;
    for op in &contents.operations {
        println!("{:?}", op);
    }

    Ok(())
}
