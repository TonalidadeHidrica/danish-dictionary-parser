use std::{collections::HashMap, path::PathBuf};

use anyhow::{bail, Context};
use clap::Parser;
use itertools::Itertools;
use pdf::{
    font::Font,
    object::{PageRc, RcRef},
};

use danish_dictionary_parser::{
    count_ops::count_ops,
    decode_pdf_string::{decode_pdf_string, make_font_map, FontMap},
    walk_text::{each_text, TextEntry},
};

#[derive(Parser)]
struct Opts {
    file: PathBuf,
    page: Option<u32>,
    #[clap(long)]
    all_but: Option<usize>,
    #[clap(long)]
    count: bool,
    #[clap(long)]
    verbose: bool,
    #[clap(long)]
    dump_lines: bool,
}

fn main() -> anyhow::Result<()> {
    let opts = Opts::parse();
    let file = pdf::file::File::open(&opts.file)?;

    if opts.count {
        count_ops(&file)?;
    } else if let Some(page) = opts.page {
        process_page(&opts, &file, file.get_page(page)?)?;
    } else {
        let all_but = opts.all_but.unwrap_or(0);
        for page in file.pages().flatten().skip(all_but) {
            process_page(&opts, &file, page)?;
        }
    }

    Ok(())
}

type ParsedTextEntry = (TextEntry, Vec<String>);
fn process_page(
    opts: &Opts,
    file: &pdf::file::File<Vec<u8>>,
    page: PageRc,
) -> anyhow::Result<Vec<Vec<ParsedTextEntry>>> {
    let lines = {
        let mut last_y = f32::INFINITY;
        let mut lines = vec![];
        for entry in each_text(file, &page)?.skip_while(|e| {
            e.as_ref()
                .map_or(false, |e| e.positions.coordinates().y <= 50.0)
        }) {
            let entry = entry?;
            let p = entry.positions.coordinates();
            if p.y < last_y - 8.0 {
                last_y = p.y;
                lines.push(vec![entry]);
            } else {
                lines
                    .last_mut()
                    .expect("The branch above should run in the first iteration")
                    .push(entry);
            }
        }
        lines
    };

    let fonts = make_font_map(file, &page)?;
    if opts.dump_lines {
        dump_lines(opts, &fonts, &lines)?;
    }

    let mut parsed_lines = vec![];
    for line in lines {
        let mut parsed_line = vec![];
        for entry in line {
            let (font, map) = fonts
                .get(entry.font.as_str())
                .with_context(|| format!("Font {:?} not found", entry.font))?;
            let strings = decode_pdf_string(map, font.subtype, &entry.text)?
                .map(|e| e.to_owned())
                .collect_vec();
            parsed_line.push((entry, strings));
        }
        parsed_lines.push(parsed_line);
    }
    Ok(parsed_lines)
}

fn dump_lines(
    opts: &Opts,
    fonts: &HashMap<&str, (RcRef<Font>, FontMap)>,
    lines: &[Vec<TextEntry>],
) -> Result<(), anyhow::Error> {
    for line in lines {
        if opts.verbose {
            println!("=============");
        }
        if let Some(entry) = line.get(0) {
            if !opts.verbose {
                let a = if indented(entry)? { "    " } else { "" };
                print!("{a:}");
            }
        }
        if !opts.verbose {
            print!("[");
        }
        for entry in line {
            let (font, map) = fonts
                .get(entry.font.as_str())
                .with_context(|| format!("Font {:?} not found", entry.font))?;
            if opts.verbose {
                print!(
                    "{:?}\t{:?}\t{:.3?}",
                    entry.font,
                    entry.text,
                    entry.positions.coordinates()
                );
            }
            for s in decode_pdf_string(map, font.subtype, &entry.text)? {
                print!("{s}");
            }
            if opts.verbose {
                println!();
            }
        }
        if !opts.verbose {
            print!("]");
        }
        println!();
    }
    Ok(())
}

fn indented(entry: &TextEntry) -> anyhow::Result<bool> {
    Ok(match entry.positions.coordinates().x {
        x if (70.5..71.5).contains(&x) => true,
        x if (80.0..82.5).contains(&x) => false,
        x if (91.5..92.5).contains(&x) => false,
        x => bail!("Unexpected x coordinates: {x}"),
    })
}
