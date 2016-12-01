# Cask

A fast key-value store written in Rust. The underlying storage system is a log-structured hash table
which is inspired by [bitcask](https://github.com/basho/bitcask/).

[![Build Status](https://travis-ci.org/andrebeat/cask.svg?branch=master)](https://travis-ci.org/andrebeat/cask)
[![Crates.io](https://img.shields.io/crates/v/cask.svg?maxage=2592000)](https://crates.io/crates/cask)
[![License](https://img.shields.io/dub/l/vibe-d.svg)](https://raw.githubusercontent.com/andrebeat/cask/master/LICENSE)

[API Documentation](http://andrebeat.github.io/cask/cask/struct.Cask.html)

* * *

**WARNING**: ⚠️ Please do not trust any valuable data to this yet. ⚠️

## Installation

Use the [crates.io](http://crates.io/) repository, add this to your Cargo.toml along with the rest
of your dependencies:

```toml
[dependencies]
cask = "0.4.0"
```

Then, use `Cask` in your crate:

```rust
extern crate cask;
use cask::Cask;
```

## Usage

The basic usage of the library is shown below:

```rust
let cask = Cask::open("test.db", false);

let key = "hello";
let value = "world";

cask.put(key, value);
cask.get(key);
cask.delete(key);
```

## TODO

- [ ] Merge files during compaction
- [ ] Configurable compaction triggers and thresholds
- [ ] Proper error handling
- [ ] Data corruption recovery
- [ ] Add tests
- [ ] Benchmark

## License

cask is licensed under the [MIT](http://opensource.org/licenses/MIT) license. See `LICENSE` for
details.
