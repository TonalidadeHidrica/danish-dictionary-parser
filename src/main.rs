use std::{collections::HashMap, path::PathBuf};

use anyhow::Context;
use clap::Parser;
use itertools::Itertools;
use pdf::object::PageRc;

#[derive(Parser)]
struct Opts {
    file: PathBuf,
    page: u32,
}

fn main() -> anyhow::Result<()> {
    let opts = Opts::parse();
    let file = pdf::file::File::open(&opts.file)?;

    let mut count = HashMap::<_, u64>::new();
    for page in file.pages().flatten() {
        let contents = page
            .contents
            .as_ref()
            .context("The page does not have contents")?;
        for op in &contents.operations {
            *count.entry(op.operator.to_owned()).or_default() += 1;
        }
    }
    for (k, v) in count.iter().sorted_by_key(|x| x.1) {
        println!("{k}\t{v}");
    }

    // let page = file.get_page(opts.page)?;
    // process_page(page)?;

    Ok(())
}

fn process_page(page: PageRc) -> anyhow::Result<()> {
    let contents = page
        .contents
        .as_ref()
        .context("The page does not have contents")?;
    let mut count = HashMap::<_, u64>::new();
    for op in &contents.operations {
        *count.entry(&op.operator).or_default() += 1;
        println!("{:?}", op);
    }
    for (k, v) in count.iter().sorted_by_key(|x| x.1) {
        println!("{k}\t{v}");
    }
    Ok(())
}
