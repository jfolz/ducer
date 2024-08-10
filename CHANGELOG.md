# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).


## [1.0.3] - 2024-08-10
### Added
- Support CPython 3.13


## [1.0.2] - 2024-07-31
### Added
- Allow creation of subclasses of Automaton, Buffer, Map, and Set


## [1.0.1] - 2024-07-31
### Fixed
- Documentation link points to readthedocs


## [1.0.0] - 2024-07-31
### Changed
- Compatibility with builtins: `difference`, `intersection`,
  `symmetric_difference`, `union` are no longer classmethods
- `Map.build` and `Set.build` syntax now `build(path, interable)`
- `Str` and `Subsequene` automata accept `bytes` instead of `str`
- Remove superfluous `decode_int` and `encode_int` functions

### Fixed
- Memory corruption if given buffer is not u8

### Added
- `Map` methods: `copy`, `__eq__`
- `Set` methods: `copy`, `isdisjoint`, `issubset`, `issuperset`,
                 `__eq__` , `__ge__`, `__gt__`, `__le__`, `__lt__`


## [0.2.1] - 2024-07-24
### Fixed
- `Map.__getitem__` correctly raises KeyError

### Added
- `Map.get` with optional default
- `Buffer.__len__`


## [0.2.0] - 2024-07-23
### Added
-  CI builds on Linux, MacOS, and Windows.


## [0.1.0] - 2024-07-22
### Added
-  Initial public release.


[Unreleased]: https://github.com/jfolz/ducer/compare/1.0.3...main
[1.0.3]: https://github.com/jfolz/ducer/compare/1.0.2...1.0.3
[1.0.2]: https://github.com/jfolz/ducer/compare/1.0.1...1.0.2
[1.0.1]: https://github.com/jfolz/ducer/compare/1.0.0...1.0.1
[1.0.0]: https://github.com/jfolz/ducer/compare/0.2.1...1.0.0
[0.2.1]: https://github.com/jfolz/ducer/compare/0.2.0...0.2.1
[0.2.0]: https://github.com/jfolz/ducer/compare/0.1...0.2.0
[0.1.0]: https://github.com/jfolz/ducer/releases/tag/0.1
