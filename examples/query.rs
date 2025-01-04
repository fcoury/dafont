use std::collections::HashMap;

use dafont::{get_font_name, FcFontCache, FcPattern, PatternMatch};

fn main() {
    let cache = FcFontCache::build();
    let fonts = cache.query_all(&FcPattern {
        monospace: PatternMatch::True,
        ..Default::default()
    });

    println!("total fonts: {}", fonts.len());

    let mut font_by_family = HashMap::new();
    for font in fonts {
        let Some((family, name)) = get_font_name(font) else {
            eprintln!("failed to get font name for {}", font.path);
            continue;
        };

        font_by_family
            .entry(family)
            .or_insert_with(Vec::new)
            .push(name);
    }

    let mut families: Vec<_> = font_by_family.keys().collect();
    families.sort();

    for family in families {
        println!("{family}");

        let names = &font_by_family[family];
        for name in names {
            println!("  {name}");
        }

        println!();
    }
}
