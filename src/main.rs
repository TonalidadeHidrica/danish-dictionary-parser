use std::{collections::HashMap, path::PathBuf};

use anyhow::{bail, Context};
use clap::Parser;
use itertools::Itertools;
use pdf::{
    encoding::{BaseEncoding, Encoding},
    font::{Font, FontType},
    object::{PageRc, RcRef, Resolve},
};

use danish_dictionary_parser::{
    count_ops::count_ops, decode_pdf_string::make_font_map, walk_text::each_text,
};

#[derive(Parser)]
struct Opts {
    file: PathBuf,
    page: Option<u32>,
    #[clap(long)]
    count: bool,
    #[clap(long)]
    verbose: bool,
}

fn main() -> anyhow::Result<()> {
    let opts = Opts::parse();
    let file = pdf::file::File::open(&opts.file)?;

    if opts.count {
        count_ops(&file)?;
    } else if let Some(page) = opts.page {
        process_page(&opts, &file, file.get_page(page)?)?;
    } else {
        for page in file.pages().flatten() {
            process_page(&opts, &file, page)?;
        }
    }

    Ok(())
}

fn process_page(opts: &Opts, file: &pdf::file::File<Vec<u8>>, page: PageRc) -> anyhow::Result<()> {
    let entries: Vec<_> = each_text(file, &page)?.try_collect()?;
    let lines = {
        let mut last_y = f32::INFINITY;
        let mut last_i = None;
        let mut lines = vec![];
        for (i, e) in entries
            .iter()
            .enumerate()
            .skip_while(|e| e.1.positions.coordinates().y <= 50.0)
        {
            let p = e.positions.coordinates();
            if p.y < last_y - 8.0 {
                // println!("{:?}", e);
                last_y = p.y;
                if let Some(last_i) = last_i.replace(i) {
                    lines.push(&entries[last_i..i]);
                }
            }
        }
        if let Some(last_i) = last_i {
            lines.push(&entries[last_i..]);
        }
        lines
    };

    let fonts = make_font_map(file, &page)?;

    for line in lines {
        if opts.verbose {
            println!("=============");
        }
        for entry in line {
            let (font, map) = fonts
                .get(entry.font.as_str())
                .with_context(|| format!("Font {:?} not found", entry.font))?;
            if opts.verbose {
                print!("{:?}\t{:?}\t", entry.font, entry.text);
            }
            match font.subtype {
                FontType::TrueType => entry
                    .text
                    .as_bytes()
                    .iter()
                    .map(|&b| &map[&(b as u16)])
                    .for_each(|s| print!("{s}")),
                FontType::Type0 => entry
                    .text
                    .as_bytes()
                    .chunks(2)
                    .map(|c| &map[&u16::from_be_bytes(c.try_into().unwrap())])
                    .for_each(|s| print!("{s}")),
                _ => bail!("Unsupported font {font:?}"),
            };
            if opts.verbose {
                println!();
            }
        }
        println!();
    }

    Ok(())
}
