use std::{collections::HashMap, path::PathBuf};

use anyhow::{bail, Context};
use clap::Parser;
use fs_err::File;
use itertools::Itertools;
use pdf::{
    font::Font,
    object::{PageRc, RcRef},
};

use danish_dictionary_parser::{
    count_ops::count_ops,
    decode_pdf_string::{decode_pdf_string, make_font_map, FontMap},
    parse_dictionary::parse_dictionary,
    walk_text::{each_text, TextEntry},
};
#[derive(Parser)]
struct Opts {
    file: PathBuf,
    output_file: Option<PathBuf>,
    #[clap(long)]
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
        let pages = file.pages().skip(all_but);
        let words = get_words(&opts, &file, pages)?;
        let entries = parse_dictionary(&words)?;
        if let Some(path) = &opts.output_file {
            serde_json::to_writer(File::create(path)?, &entries)?;
        }
    }

    Ok(())
}

fn get_words(
    opts: &Opts,
    file: &pdf::file::File<Vec<u8>>,
    pages: impl Iterator<Item = pdf::error::Result<PageRc>>,
) -> anyhow::Result<Vec<String>> {
    let mut words = vec![];
    for page in pages {
        for line in process_page(opts, file, page?)? {
            // line is guaranteed to be non-empty
            // heading
            if line[0].0.positions.glyph_size() > 11.0 {
                continue;
            }
            // empty line
            if line.len() == 1 && line[0].1 == [" "] {
                continue;
            }
            if not_indented(&line[0].0)? {
                words.push(String::new());
            }
            let word = words
                .last_mut()
                .context("Found indented line before the first line")?;
            for (_, chars) in line {
                for c in chars {
                    word.push_str(&c);
                }
            }
        }
    }
    Ok(words)
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
                let a = if not_indented(entry)? { "    " } else { "" };
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

fn not_indented(first_entry: &TextEntry) -> anyhow::Result<bool> {
    Ok(match first_entry.positions.coordinates().x {
        // Hack: manual indentation
        _ if first_entry.text.as_bytes() == b" " => false,
        x if (70.5..71.5).contains(&x) => true,
        x if (80.0..82.5).contains(&x) => false,
        x if (91.5..92.5).contains(&x) => false,
        x => bail!("Unexpected x coordinates: {x}"),
    })
}
