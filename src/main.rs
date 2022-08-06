use std::path::PathBuf;

use anyhow::Context;
use clap::Parser;
// use danish_dictionary_parser::operator::Operator;

use pdf::object::PageRc;

#[derive(Parser)]
struct Opts {
    file: PathBuf,
    page: Option<u32>,
}

fn main() -> anyhow::Result<()> {
    let opts = Opts::parse();
    let file = pdf::file::File::open(&opts.file)?;

    if let Some(page) = opts.page {
        process_page(&file, file.get_page(page)?)?;
    } else {
        for page in file.pages().flatten() {
            process_page(&file, page)?;
        }
    }

    Ok(())
}

fn process_page(file: &pdf::file::File<Vec<u8>>, page: PageRc) -> anyhow::Result<()> {
    let contents = page
        .contents
        .as_ref()
        .context("The page does not have contents")?;
    for op in &contents.operations(file) {
        println!("{:?}", op);
    }
    Ok(())
}
