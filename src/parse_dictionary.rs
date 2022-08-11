use itertools::Itertools;
use regex::Regex;

pub fn parse_dictionary(words: &[String]) -> anyhow::Result<()> {
    let extended_word_chars = r"[a-zA-Z7éøæåØÆÅ\-.,’()/＝]+";
    let extended_heading_words = format!(
        r"(?x)
            {extended_word_chars}
            ((?-x) (?x) {extended_word_chars})*
        "
    );
    let word_chars = r"[a-zA-ZøæåØÆÅ]+";
    let heading_words = format!(
        r"(?x)
            {word_chars}
            ((?-x) (?x) {word_chars})*
        "
    );
    let pos = r"(?x)
            (
                [\[［](
                    名(・[単複])?
                    | 固 | 代 | 数 | 形 | 動 | 副 | 前 | 接 | 間
                    | 不定詞マーカー | 冠 | 不定冠詞 | 形式主語
                )[\]］]
                | \[形\]\s*\[無変化\]
            )
        ";
    let pronunciation = r"(?x)
            ([
                a-z’åȧäæöøα:
                ˈˌəɑðŋɔgnɹ
                \u0329\u030A\u0308
                \u0227ᒑ;\u0283
                ()
                \uF0D9
            ]|(?-x) (?x))+
        ";
    let pronunciation_list = format!(
        r"(?x)
            {pronunciation}
            ([,，]\s* {pronunciation} )*
        "
    );
    let word_and_pronunciation = format!(
        r"(?x)
            (?P<wp_word> {heading_words} ) \s*
            \[ (?P<wp_pronunciation> {pronunciation_list} ) \] \s*
        "
    );
    let word_and_pronunciation_regex = Regex::new(&word_and_pronunciation)?;
    let other_forms = format!(
        r"(?x)
            ,\s*
            (?P<of_suffix_marker> \+ )?
            (?P<of_word> {extended_heading_words})
            (?P<of_imparative> ! )? \s*
            \[ (?P<of_pronunciation> {pronunciation_list} ) \] \s*
            (?P<of_slahsed> / {word_and_pronunciation} )*
        "
    );
    let other_forms_regex = Regex::new(&other_forms)?;
    let entry_pattern = format!(
        r"(?x)
            ^
            \+?
            (?P<word> {extended_heading_words})
            (\s*[1-4])?
            \s*

            (?P<pos> 
                {pos}
                ( [,，]\s* {pos} )*
            )?
            \s*

            \[ (?P<pronunciation> {pronunciation_list} ) \] \s*

            (?P<invariant_adjective> [\[［] 不変化 [\]］] \s* )?

            (?P<other_forms> ( {other_forms} )* )

            (?P<other_adjective_forms> (
                ,\s*
                {heading_words} (/ {heading_words} )* \s*
            )*)

            ( \(en\) )?

            [:：]
        "
    );
    let regex = Regex::new(&entry_pattern)?;

    for word in words {
        #[allow(clippy::if_same_then_else)]
        if let Some(res) = regex.captures(word) {
            let word = &res["word"];
            let pos = res.name("pos").map(|x| x.as_str());
            let pronunciation = &res["pronunciation"];
            let invariant_adjective = res.name("invariant_adjective").is_some();
            let other_forms = res.name("other_forms").map(|x| x.as_str());
            let other_adjective_forms = &res["other_adjective_forms"];

            let comma = &[',', '，'];
            let pos = pos.map_or_else(Vec::new, |pos| {
                use Pos::*;
                let v = pos.split(comma).map(|pos| {
                    match pos
                        .trim()
                        .strip_prefix(&['[', '［'])
                        .unwrap()
                        .strip_suffix(&[']', '］'])
                        .unwrap()
                    {
                        "名" => Noun(None),
                        "名・単" => Noun(Some(NounCount::Single)),
                        "名・複" => Noun(Some(NounCount::Multiple)),
                        "固" => ProperNoun,
                        "代" => Pronoun,
                        "数" => Numeral,
                        "形" => Adjective(invariant_adjective),
                        s if s.split_whitespace().collect_tuple() == Some(("形]", "[無変化")) => {
                            Adjective(true)
                        }
                        "動" => Verb,
                        "副" => Adverb,
                        "前" => Preposition,
                        "接" => Conjunction,
                        "間" => Interjection,
                        "不定詞マーカー" => InfinitiveMarker,
                        "冠" => Article,
                        "不定冠詞" => IndefiniteArticle,
                        "形式主語" => FormalSubject,
                        e => unreachable!("Unexpected pos {e:?}"),
                    }
                });
                v.collect()
            });

            let other_forms = other_forms.map_or_else(Vec::new, |other_forms| {
                let v = other_forms_regex.captures_iter(other_forms).map(|res| {
                    let word = res.name("of_word").unwrap().as_str();
                    let pronunciation = res.name("of_pronunciation").unwrap().as_str();
                    let slahsed = res.name("of_slashed").map_or_else(Vec::new, |pairs| {
                        let v = pairs.as_str().split('/').skip(1).map(|pair| {
                            let res = word_and_pronunciation_regex
                                .captures(pair.trim())
                                .expect("Already matched");
                            OtherForm {
                                word: res.name("wp_word").unwrap().as_str(),
                                pronunciations: parse_pronuncitation_list(
                                    res.name("wp_pronunciation").unwrap().as_str(),
                                ),
                                slahsed: vec![],
                            }
                        });
                        v.collect()
                    });
                    OtherForm {
                        word,
                        pronunciations: parse_pronuncitation_list(pronunciation),
                        slahsed,
                    }
                });
                v.collect()
            });

            let other_adjective_forms = match other_adjective_forms {
                "" => vec![],
                a => a
                    .split(',')
                    .map(|s| s.split('/').map(str::trim).collect_vec())
                    .collect(),
            };

            let _entry = Entry {
                word,
                pos,
                pronunciations: parse_pronuncitation_list(pronunciation),
                other_forms,
                other_adjective_forms,
            };
            // println!("{entry:#?}");
        } else if word.chars().filter(|&c| c == '→').count() == 1 {
            // TODO
        } else {
            println!("{word:?}");
        }
    }

    Ok(())
}

fn parse_pronuncitation_list(s: &str) -> Vec<&str> {
    s.split(',').map(str::trim).collect()
}

#[derive(Debug)]
pub struct Entry<'a> {
    pub word: &'a str,
    pub pos: Vec<Pos>,
    pub pronunciations: Vec<&'a str>,
    pub other_forms: Vec<OtherForm<'a>>,
    pub other_adjective_forms: Vec<Vec<&'a str>>,
}

#[derive(Clone, Copy, Debug)]
pub enum Pos {
    Noun(Option<NounCount>),
    ProperNoun,
    Pronoun,
    Numeral,
    Adjective(bool),
    Verb,
    Adverb,
    Preposition,
    Conjunction,
    Interjection,
    InfinitiveMarker,
    Article,
    IndefiniteArticle,
    FormalSubject,
}
#[derive(Clone, Copy, Debug)]
pub enum NounCount {
    Single,
    Multiple,
}

#[derive(Debug)]
pub struct OtherForm<'a> {
    pub word: &'a str,
    pub pronunciations: Vec<&'a str>,
    pub slahsed: Vec<OtherForm<'a>>,
}
