use std::path::PathBuf;

use clap::Parser;
use itertools::Itertools;
use ordered_float::NotNan;
use pdf::object::PageRc;

use danish_dictionary_parser::{count_ops::count_ops, walk_text::each_text};

#[derive(Parser)]
struct Opts {
    file: PathBuf,
    page: Option<u32>,
    #[clap(long)]
    count: bool,
}

fn main() -> anyhow::Result<()> {
    let opts = Opts::parse();
    let file = pdf::file::File::open(&opts.file)?;

    if opts.count {
        count_ops(&file)?;
    } else if let Some(page) = opts.page {
        process_page(&file, file.get_page(page)?)?;
    } else {
        for page in file.pages().flatten() {
            process_page(&file, page)?;
        }
    }

    Ok(())
}

fn process_page(file: &pdf::file::File<Vec<u8>>, page: PageRc) -> anyhow::Result<()> {
    for e in each_text(file, &page)? {
        let e = e?;
        println!("{:?}\t{:?}", e.positions.coordinates(), e.text);
    }

    let v: Vec<_> = each_text(file, &page)?
        .filter_map_ok(|e| {
            ((e.positions.glyph_size() - 10.5).abs() <= 0.1).then(|| e.positions.coordinates().1)
        })
        .sorted_by_key(|x| x.as_ref().ok().map(|&x| NotNan::new(x).unwrap()))
        .try_collect()?;
    for e in v {
        // println!("{}", e);
    }

    Ok(())
}
