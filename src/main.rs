use std::path::PathBuf;

use anyhow::Context;
use clap::Parser;
use danish_dictionary_parser::operator::Operator;

use pdf::object::PageRc;

#[derive(Parser)]
struct Opts {
    file: PathBuf,
    page: u32,
}

fn main() -> anyhow::Result<()> {
    let opts = Opts::parse();
    let file = pdf::file::File::open(&opts.file)?;

    for page in file.pages().flatten() {
        process_page(page)?;
    }

    Ok(())
}

fn process_page(page: PageRc) -> anyhow::Result<()> {
    let contents = page
        .contents
        .as_ref()
        .context("The page does not have contents")?;
    for op in &contents.operations {
        let _op: Operator = op
            .clone()
            .try_into()
            .with_context(|| format!("While parsing {op:?}"))?;
    }
    Ok(())
}
