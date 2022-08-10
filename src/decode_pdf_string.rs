use std::collections::HashMap;

use anyhow::bail;
use itertools::Itertools;
use pdf::{
    encoding::{BaseEncoding, Encoding},
    font::{Font, FontType},
    object::{PageRc, RcRef, Resolve},
    primitive::PdfString,
};

pub type FontMap = HashMap<u16, String>;
pub fn make_font_map<'p>(
    file: &pdf::file::File<Vec<u8>>,
    page: &'p PageRc,
) -> anyhow::Result<HashMap<&'p str, (RcRef<Font>, FontMap)>> {
    page.resources()?
        .fonts()
        .map(|(k, &font)| {
            let font = file.get(font)?;
            let map = make_unicode_map(file, &font)?;
            anyhow::Ok((k, (font, map)))
        })
        .try_collect()
}

pub fn decode_pdf_string<'b, 'a: 'b>(
    map: &'a FontMap,
    subtype: FontType,
    text: &'b PdfString,
) -> anyhow::Result<impl Iterator<Item = &'a String> + 'b> {
    Ok(match subtype {
        FontType::TrueType => {
            itertools::Either::Left(text.as_bytes().iter().map(|&b| &map[&(b as u16)]))
        }
        FontType::Type0 => itertools::Either::Right(
            text.as_bytes()
                .chunks(2)
                .map(|c| &map[&u16::from_be_bytes(c.try_into().unwrap())]),
        ),
        _ => bail!("Unsupported font type {subtype:?}"),
    })
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
        Some("NZLSMO+GaijiL2") => {
            return Ok(maplit::hashmap![
                76 => "\u{329}".into(),
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
