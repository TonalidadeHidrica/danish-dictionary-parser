use std::{collections::HashMap, path::PathBuf};

use anyhow::{bail, Context};
use clap::Parser;
use itertools::Itertools;
use pdf::{
    encoding::{BaseEncoding, Encoding},
    font::{Font, FontType},
    object::{PageRc, RcRef, Resolve},
};

use danish_dictionary_parser::{count_ops::count_ops, walk_text::each_text};

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

    let fonts: HashMap<_, _> = page
        .resources()?
        .fonts()
        .map(|(k, &font)| {
            let font = file.get(font)?;
            let map = make_unicode_map(file, &font)?;
            anyhow::Ok((k, (font, map)))
        })
        .try_collect()?;

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

fn make_unicode_map(
    file: &pdf::file::File<Vec<u8>>,
    font: &RcRef<Font>,
) -> anyhow::Result<HashMap<u16, String>> {
    match font.name.as_ref().map(|x| x.as_str()) {
        // Embedded gaiji font.  No way to get these mapping from file so we hardcode them.
        Some("DXNKCI+GaijiL") => {
            return Ok(maplit::hashmap![
                65 => "\u{227}".into(),
                67 => "ᒑ".into(), // similar to [j]?  no such glyph in unicode
                68 => ";".into(), // long vowel with stød, no such glyph in unicode
                69 => "\u{283}".into(),
            ]);
        }
        // Patch IPA font that uses private use area to standard Unicode phonetic alphabet for
        // visualization purpose.
        Some("DXNKCI+Ipa-samdUclphon1SILDoulosL") => {
            return Ok(maplit::hashmap![
                // 【？】
                4 => "ˈ".into(),
                7 => "ˌ".into(),
                34 => "ə".into(),
                35 => "ɑ".into(),
                38 => "ð".into(),
                48 => "ŋ".into(),
                49 => "ɔ".into(),
                73 => "g".into(),
                80 => "n".into(),
                132 => "ɹ".into(),
                186 => "\u{0329}".into(),
                194 => "\u{030A}".into(),
                196 => "\u{0308}".into(),
                229 => "【？】".into(),
                254 => "【？】".into(),
                256 => "【？】".into(),
            ]);
        }
        _ => {}
    };
    if let Some(map) = font.to_unicode(file).transpose()? {
        Ok(map.iter().map(|(k, v)| (k, v.into())).collect())
    } else if let (
        FontType::TrueType,
        Some(Encoding {
            base: BaseEncoding::WinAnsiEncoding,
            differences,
        }),
    ) = (font.subtype, font.encoding())
    {
        // Based on https://github.com/kaj/rust-pdf/blob/master/src/encoding.rs
        let mut codes = HashMap::new();
        for code in 32..255u8 {
            codes.insert(code as u16, (code as char).to_string());
        }
        codes.insert(128, "€".into());
        codes.insert(130, "‚".into());
        codes.insert(131, "ƒ".into());
        codes.insert(132, "„".into());
        codes.insert(133, "…".into());
        codes.insert(134, "†".into());
        codes.insert(135, "‡".into());
        codes.insert(136, "ˆ".into());
        codes.insert(137, "‰".into());
        codes.insert(138, "Š".into());
        codes.insert(139, "‹".into());
        codes.insert(140, "Œ".into());
        codes.insert(142, "Ž".into());
        codes.insert(145, "‘".into());
        codes.insert(146, "’".into());
        codes.insert(147, "“".into());
        codes.insert(148, "”".into());
        codes.insert(149, "•".into());
        codes.insert(150, "–".into());
        codes.insert(151, "—".into());
        codes.insert(152, "˜".into());
        codes.insert(153, "™".into());
        codes.insert(154, "š".into());
        codes.insert(155, "›".into());
        codes.insert(158, "ž".into());
        codes.insert(159, "Ÿ".into());
        codes.extend(differences.iter().map(|(&k, v)| (k as u16, v.into())));
        Ok(codes)
    } else {
        bail!("Cannot generate ToUnicode map from {font:?}")
    }
}
