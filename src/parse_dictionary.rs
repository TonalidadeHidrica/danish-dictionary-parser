use anyhow::bail;
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
    let other_adjective_forms = format!(
        r"(?x)
            ,\s*
            (?P<oaf_word> {heading_words} )
            ( \s* \[ (?P<oaf_pronunciation> {pronunciation_list} ) \] \s* )?
            (?P<oaf_slashed> (/ {heading_words} )* ) \s*
        "
    );
    let other_adjective_forms_regex = Regex::new(&other_adjective_forms)?;
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
            (en\s*)?

            (?P<other_forms> ( {other_forms} )* )

            (?P<other_adjective_forms> ( {other_adjective_forms} )*)

            ( \(en\) )?

            [:：]
        "
    );
    let regex = Regex::new(&entry_pattern)?;

    for word in words {
        #[allow(clippy::if_same_then_else)]
        if let Some(res) = regex.captures(patch(word)) {
            let word = &res["word"];
            let pos = res.name("pos").map(|x| x.as_str());
            let pronunciation = &res["pronunciation"];
            let invariant_adjective = res.name("invariant_adjective").is_some();
            let other_forms = &res["other_forms"];
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

            let other_forms = other_forms_regex
                .captures_iter(other_forms)
                .map(|res| {
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
                })
                .collect();

            let other_adjective_forms = other_adjective_forms_regex
                .captures_iter(other_adjective_forms)
                .map(|res| {
                    let word = res.name("oaf_word").unwrap().as_str();
                    let pronunciations = res
                        .name("oaf_pronunciation")
                        .map_or_else(Vec::new, |s| parse_pronuncitation_list(s.as_str()));
                    let slahsed = res
                        .name("oaf_slashed")
                        .unwrap()
                        .as_str()
                        .split('/')
                        .skip(1)
                        .map(|s| OtherForm {
                            word: s.trim(),
                            pronunciations: vec![],
                            slahsed: vec![],
                        })
                        .collect();
                    OtherForm {
                        word,
                        pronunciations,
                        slahsed,
                    }
                })
                .collect();

            let _entry = Entry {
                word,
                pos,
                pronunciations: parse_pronuncitation_list(pronunciation),
                other_forms,
                other_adjective_forms,
            };
            println!("{_entry:#?}");
        } else if word.chars().filter(|&c| c == '→').count() == 1 {
            // TODO
        } else {
            bail!("Could not parse {word:?}")
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
    pub other_adjective_forms: Vec<OtherForm<'a>>,
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

pub fn patch(s: &str) -> &str {
    match s {
        // no colon
        "altså [副] [ˈal’sɔ, ˈαl’sɔ] すなわち，それゆえに，したがって，つまり，ようするに；〔話し手の心的態度を表す文副詞〕［意味を強めて］ほんとうに，まったく；［驚きを表して］なんと！；［遺憾，不満，苛立ち；非難・咎めを表して］ほんとうに，いいかげん（に）；［相手の言ったことに対して，相手の計画等の変更を促して］（…ということなのですが，それでもあなたは…するのですか？）；［間投詞的に用いられて，いらだち，非難，没頭など様々な感情を表す（特に話しことばにおいて）］あのねえ！，いいかい！，いやはや！，やれやれ！，これは驚いた！ " => "altså [副] [ˈal’sɔ, ˈαl’sɔ]: すなわち，それゆえに，したがって，つまり，ようするに；〔話し手の心的態度を表す文副詞〕［意味を強めて］ほんとうに，まったく；［驚きを表して］なんと！；［遺憾，不満，苛立ち；非難・咎めを表して］ほんとうに，いいかげん（に）；［相手の言ったことに対して，相手の計画等の変更を促して］（…ということなのですが，それでもあなたは…するのですか？）；［間投詞的に用いられて，いらだち，非難，没頭など様々な感情を表す（特に話しことばにおいて）］あのねえ！，いいかい！，いやはや！，やれやれ！，これは驚いた！ ",
        // no colon
        "Amager [固] [ˈαˌmα;] アマー［地名：コペンハーゲン南部の島．Kastrup空港がある］． " => "Amager [固] [ˈαˌmα;]: アマー［地名：コペンハーゲン南部の島．Kastrup空港がある］． ",
        // no colon
        "Amaliegade [固] [aˈmȧ;ljənˌgȧ:ðə] アメーリェゲーゼ［通り：コペンハーゲン］． " => "Amaliegade [固] [aˈmȧ;ljənˌgȧ:ðə]: アメーリェゲーゼ［通り：コペンハーゲン］． ",
        // no colon
        "Amalienborg [固] [aˈmȧ;ljənˌbɑ\u{30a}’] アメーリェンボー［王宮：コペンハーゲン］． " => "Amalienborg [固] [aˈmȧ;ljənˌbɑ\u{30a}’]: アメーリェンボー［王宮：コペンハーゲン］． ",
        // illegal pronuciation position
        "De [ˈdi, di] [代] [ˈdi, di], Dem [ˈdæm, dæm], Deres [ˈdȧɹɔs, ˈdȧ:ɔs, dȧɔs]:［人称代名詞２人称単数・複数］［フォーマルな関係の人に対して用いる］あなた，あなた方． " => "De [代] [ˈdi, di], Dem [ˈdæm, dæm], Deres [ˈdȧɹɔs, ˈdȧ:ɔs, dȧɔs]:［人称代名詞２人称単数・複数］［フォーマルな関係の人に対して用いる］あなた，あなた方． ",
        // illegal pronuciation position
        "de [ˈdi, di] [代] [ˈdi, di], dem [ˈdæm, dæm], deres [ˈdȧɹɔs, ˈdȧ:ɔs, dȧɔs]:［人称代名詞３人称複数］彼ら，彼女ら，それら；［不定代名詞］(一般の)人々，みんな；権威，当局；［指示代名詞］［人・動物・もの・ことを指して］あれら(の)，それら(の)；(…する・である)人たち・もの． " => "de [代] [ˈdi, di], dem [ˈdæm, dæm], deres [ˈdȧɹɔs, ˈdȧ:ɔs, dȧɔs]:［人称代名詞３人称複数］彼ら，彼女ら，それら；［不定代名詞］(一般の)人々，みんな；権威，当局；［指示代名詞］［人・動物・もの・ことを指して］あれら(の)，それら(の)；(…する・である)人たち・もの． ",
        // illegal pronuciation position
        "den1 [ˈdæn’, dæn] [代] [ˈdæn’, dæn], dens [ˈdæn(’)s, dæns], det [ˈde, de],  dets [ˈdæds, dæds], de [ˈdi, di], dem [ˈdæm, dæm], deres [ˈdȧɹɔs, ˈdȧ:ɔs, dȧɔs]:［人称代名詞３人称］［すでに述べた動物・もの・ことに参照して］それ；［指示代名詞］［人・動物・もの・ことを指して］あれ，それ；あの，その；前者の；前者． " => "den1 [代] [ˈdæn’, dæn], dens [ˈdæn(’)s, dæns], det [ˈde, de],  dets [ˈdæds, dæds], de [ˈdi, di], dem [ˈdæm, dæm], deres [ˈdȧɹɔs, ˈdȧ:ɔs, dȧɔs]:［人称代名詞３人称］［すでに述べた動物・もの・ことに参照して］それ；［指示代名詞］［人・動物・もの・ことを指して］あれ，それ；あの，その；前者の；前者． ",
        // no colon
        "en2 [数] [ˈe;n] et [ˈed]: 一，1つ，1人，1個 " => "en2 [数] [ˈe;n], et [ˈed]: 一，1つ，1人，1個 ",
        // no colon
        "firsindstyvende [数] [ˈfiɹ’sənsˈty:vənə] 第八十番目の． " => "firsindstyvende [数] [ˈfiɹ’sənsˈty:vənə]: 第八十番目の． ",
        // no colon
        "Frederiksborg Slot [固] [fræðrægsˈbɑ\u{30a};ˈslɔd] フレズレクスボー城［北シェランにある城］ " => "Frederiksborg Slot [固] [fræðrægsˈbɑ\u{30a};ˈslɔd]: フレズレクスボー城［北シェランにある城］ ",
        // no colon
        "græker [名] [ˈgræ;gɔ], grækeren [ˈgræ;gɔɔn], grækere [ˈgræ;gɔɔ],  grækerne [ˈgræ;gɔnə], ギリシア人 " => "græker [名] [ˈgræ;gɔ], grækeren [ˈgræ;gɔɔn], grækere [ˈgræ;gɔɔ],  grækerne [ˈgræ;gɔnə]: ギリシア人 ",
        // no colon
        "halvfemsindstyvende [数] [halˈfæm’sənsˈty:vənə, halˈfæm’sənsˈty:wənə] 第九十番目の． " => "halvfemsindstyvende [数] [halˈfæm’sənsˈty:vənə, halˈfæm’sənsˈty:wənə]: 第九十番目の． ",
        // no colon
        "halvfjerdsindstyvende [数] [halˈfjȧɹsənsˈty:vənə, halˈfjȧɹsənsˈty:wənə] 第七十番目の． " => "halvfjerdsindstyvende [数] [halˈfjȧɹsənsˈty:vənə, halˈfjȧɹsənsˈty:wənə]: 第七十番目の． ",
        // no colon
        "halvtredsindstyvende [数] [ˈtræsənsˈty:vənə, ˈtræsənsˈty:wənə] 第五十番目の． " => "halvtredsindstyvende [数] [ˈtræsənsˈty:vənə, ˈtræsənsˈty:wənə]: 第五十番目の． ",
        // illegal parenthesis
        "have1 [名] [ˈhȧ:və, ˈhȧ:wə], haven [[ˈhȧ:vən, ˈhȧ:wən], haver [ˈhȧ:vɔ, ˈhȧ:wɔ],  haverne [ˈhȧ:vɔnə, ˈhȧ:wɔnə]: 庭，庭園，公園． " => "have1 [名] [ˈhȧ:və, ˈhȧ:wə], haven [ˈhȧ:vən, ˈhȧ:wən], haver [ˈhȧ:vɔ, ˈhȧ:wɔ],  haverne [ˈhȧ:vɔnə, ˈhȧ:wɔnə]: 庭，庭園，公園． ",
        // TODO illegal insertion of "en" (not sure)
        "hof [名] [ˈhɔf] en, hoffer [ˈhɔfɔ]: デンマークのビール会社カールスベア社 (Carlsberg) のラガービールの愛称． " => "hof [名] [ˈhɔf] en, hoffer [ˈhɔfɔ]: デンマークのビール会社カールスベア社 (Carlsberg) のラガービールの愛称． ",
        // missing comma
        "hun [代] [ˈhun, hun] hende [ˈhenə, henə], hendes [ˈhenəs, henəs]:［人称代名詞３人称単数女性］彼女． " => "hun [代] [ˈhun, hun], hende [ˈhenə, henə], hendes [ˈhenəs, henəs]:［人称代名詞３人称単数女性］彼女． ",
        // Special word
        "hvem [代] [ˈvæm’],［所有格］hvis [ˈves]:［疑問代名詞］誰・どの人・どんな人(が・を・に)；［関係代名詞］［人を表す先行詞を受けて］…するところの(人)［現在では，この用法では，hvemが関係節中の主語になることはない］；［先行詞を含む不定関係代名詞］(…する)だれでも，どんな人でも． " => "hvem [代] [ˈvæm’], hvis [ˈves]:［疑問代名詞］誰・どの人・どんな人(が・を・に)；［関係代名詞］［人を表す先行詞を受けて］…するところの(人)［現在では，この用法では，hvemが関係節中の主語になることはない］；［先行詞を含む不定関係代名詞］(…する)だれでも，どんな人でも． ",
        // missing comma
        "jeg [代] [ˈjα(ᒑ), jα(ᒑ)] mig [ˈmαᒑ, mα(ᒑ)]:［人称代名詞１人称単数］私，僕． " => "jeg [代] [ˈjα(ᒑ), jα(ᒑ)], mig [ˈmαᒑ, mα(ᒑ)]:［人称代名詞１人称単数］私，僕． ",
        // illegal POS insertion
        "klejne [名] [ˈklαᒑnə], klejnen [名] [ˈklαᒑnən], klejner [ˈklαᒑnɔ], klejner [ˈklαᒑnɔnə]: クライネ（特にクリスマスの時期に食する，ねじりドーナッツ）． " => "klejne [名] [ˈklαᒑnə], klejnen [ˈklαᒑnən], klejner [ˈklαᒑnɔ], klejner [ˈklαᒑnɔnə]: クライネ（特にクリスマスの時期に食する，ねじりドーナッツ）． ",
        // typo, found \uf022 instead of !
        "knuse [動] [ˈknu:sə], knuser [ˈknu;sɔ], knuste [ˈknu:sdə], knust [ˈknu;sd],  knusende [ˈknu:sənə], knus\u{f022} [ˈknu;s]: 壊す，こなごなにする，砕く． " => "knuse [動] [ˈknu:sə], knuser [ˈknu;sɔ], knuste [ˈknu:sdə], knust [ˈknu;sd],  knusende [ˈknu:sənə], knus! [ˈknu;s]: 壊す，こなごなにする，砕く． ",
        // Special word
        "lille [形] [ˈlilə]［性，既知/未知を問わず，名詞の単数形とともに］, små [små;] [性，既知/未知を問わず，名詞の複数形とともに]，mindre [ˈmendrɔ], mindst [ˈmen’sd], mindste [ˈmen’sdə]: 小さな． " => "lille [形] [ˈlilə], små [små;], mindre [ˈmendrɔ], mindst [ˈmen’sd], mindste [ˈmen’sdə]: 小さな． ",
        // Missing colon
        "Louisiana [固] [luisiˈana] ルイスィアナ美術館［北シェランのホムレベク(Humlebæk) にある美術館］． " => "Louisiana [固] [luisiˈana]: ルイスィアナ美術館［北シェランのホムレベク(Humlebæk) にある美術館］． ",
        // insonsistent style for multiple pronunciations
        "O.k. [間] [ˈåwˈkæᒑ]/[ˈo;ˈkå;]: OK，了解，わかりました " => "O.k. [間] [ˈåwˈkæᒑ, ˈo;ˈkå;]: OK，了解，わかりました ",
        // missing colong
        "Samsø [固] [ˈsαmˌsø;] サムスー島． " => "Samsø [固] [ˈsαmˌsø;]: サムスー島． ",
        // illegal parenthesis
        "snitte [動] [ˈsnidə], snitter [ˈsnidɔ], snittede [ˈsnidəðə], snittet [ˈsnidəð],  snittende [ˈsnidənə], snit!] [ˈsnid]: (木などを)ナイフで少し削る；彫る，刻む；細かく・薄く切る． " => "snitte [動] [ˈsnidə], snitter [ˈsnidɔ], snittede [ˈsnidəðə], snittet [ˈsnidəð],  snittende [ˈsnidənə], snit! [ˈsnid]: (木などを)ナイフで少し削る；彫る，刻む；細かく・薄く切る． ",
        // too many commas
        "spændende [形] [ˈsbænənə] [不変化], , mere spændende, mest spændende: 面白い；わくわくする；スリリングな． " => "spændende [形] [ˈsbænənə] [不変化], mere spændende, mest spændende: 面白い；わくわくする；スリリングな． ",
        // TODO en
        "tuborg [名] [ˈtuˌbɑ;] en, tuborg [ˈtuˌbɑ;]: かつてはビール会社トゥボー社が生産し，現在はカールスベア社が生産しているラガービールgrøm tuborgの愛称． " => "tuborg [名] [ˈtuˌbɑ;] en, tuborg [ˈtuˌbɑ;]: かつてはビール会社トゥボー社が生産し，現在はカールスベア社が生産しているラガービールgrøm tuborgの愛称． ",
        // semicolon instead of colon
        "varm [形] [ˈvα;m], varmt [ˈvα;md], varme [ˈvα:mə], varmere [ˈvα:mɔɔ],  varmest [ˈvα:məsd], varmeste [ˈvα:məsdə];温かい，暖かい；やや暑い；熱い；思いやりのある，心のこもった． " => "varm [形] [ˈvα;m], varmt [ˈvα;md], varme [ˈvα:mə], varmere [ˈvα:mɔɔ],  varmest [ˈvα:məsd], varmeste [ˈvα:məsdə]: 温かい，暖かい；やや暑い；熱い；思いやりのある，心のこもった． ",
        // illegal parenthesis
        "vid [形] [ˈvi;ð], vidt [ˈvid], vide [形] [ˈvi:ðə], videre [ˈvi:ðɔɔ, videst [ˈvi:ðəsd],  videste [ˈvi:ðəsdə]: 広い，広大な，広々とした；遠い，遠く離れた． " => "vid [形] [ˈvi;ð], vidt [ˈvid], vide [ˈvi:ðə], videre [ˈvi:ðɔɔ], videst [ˈvi:ðəsd],  videste [ˈvi:ðəsdə]: 広い，広大な，広々とした；遠い，遠く離れた． ",
        // Exceptional pattern
        "øre1 [名] [ˈø:ɔ], øret [ˈø:ɔð], ører [ˈø:ɔ](/øren [ˈø:ɔn]), ørerne [ˈø:ɔnə]: 耳． " => "øre1 [名] [ˈø:ɔ], øret [ˈø:ɔð], ører [ˈø:ɔ]/øren [ˈø:ɔn], ørerne [ˈø:ɔnə]: 耳． ",
        _ => s,
    }
}
