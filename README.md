# dafont

Fork of [**rust-fontconfig**](https://github.com/fschutt/rust-fontconfig) with monospace detection.

## Original README

Pure-Rust rewrite of the Linux fontconfig library (no system
dependencies) - using allsorts as a font parser in order to
parse `.woff`, `.woff2`, `.ttc`, `.otf` and `.ttf`

**NOTE**: Also works on Windows and macOS - without external dependencies!

## Motivation

There are a number of reasons why I want to have
a pure-Rust version of fontconfig:

- fontconfig with all dependencies (expat and freetype) is ~190.000
  lines of C (extremely bloated for what it does)
- fontconfig, freetype, expat and basically any kind of parsing in C
  is a common attack vector (via maliciously crafted fonts). The Rust
  version (allsorts) checks the boundaries before accessing
  memory, so attacks via font files should be less common.
- it gets rid of the cmake / cc dependencies necessary to build
  [azul](https://azul.rs) on Linux
- fontconfig isn't really a "hard" library to rewrite, it just
  parses fonts and selects fonts by name
- Rust has existing xml parsers and font parsers, just use those
- It allows fontconfig libraries to be purely statically linked
- Font parsing / loading can be easily multithreaded (parsing font files in parallel)
- It reduces the number of necessary non-Rust dependencies on Linux for azul to 0
- fontconfig (or at least the Rust bindings) do not allow you
  to store an in-memory cache, only an on-disk cache, requiring
  disk access on every query (= slow)
- Potential `no_std` support for minimal binaries?

Now for the more practical reasons:

- libfontconfig 0.12.x sometimes hangs and crashes
  ([see issue](https://github.com/maps4print/azul/issues/110))
- libfontconfig introduces build issues with cmake / cc
  ([see issue](https://github.com/maps4print/azul/issues/206))
- To support font fallback in CSS selectors and text runs based
  on Unicode ranges, you have to do several calls into C, since
  fontconfig doesn't handle that
- The rust rewrite uses multithreading and memory mapping, since
  that is faster than reading each file individually
- The rust rewrite only parses the font tables necessary to select
  the name, not the entire font
- The rust rewrite uses very few allocations (some are necessary
  because of UTF-16 / UTF-8 conversions and multithreading lifetime
  issues)

## Usage

```rust
use rust_fontconfig::{FcFontCache, FcPattern};

fn main() {
    let cache = FcFontCache::build(); let result =

    cache.query(&FcPattern {
      name: Some(String::from("Arial")),
      .. Default::default()
    });

    println!("font path: {:?}", result);
}
```

## Performance

- cache building: ~90ms for ~530 fonts
- cache query: ~4µs

## License

MIT

