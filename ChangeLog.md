# Changelog

## 0.1.0
- First public release

## 0.1.3
- Fixed crates.io metadata
- Added Hasher as an option
- Added travis and benchmarks by criterion

## 0.1.4
- Added feature-gates for rocksdb and sled
- Moved criterion from [dependencies] to [dev-dependencies]

## 0.1.5
- Bumped up dependencies
- Changed license to MIT
- Changed the icon

## 0.2.0
- Update dependencies
- Fix outdated syntaxes and macros

## 0.3.0
- Update dependencies
- Fix bugs in memory-caching
- Add functions to track the latest state: `get_headroot` and `set_headroot`.

## 0.4.0
- Update dependencies
- Fixed bug where root hash depended on insertion order for small numbers of key-value pairs.
- Fix bugs in `Bits` serialization.
- Optimize `bit` a bit.
