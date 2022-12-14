# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [v0.8.0] - 2022-11-30
### :boom: BREAKING CHANGES
- due to [`df77020`](https://github.com/JakeStanger/ironbar/commit/df77020c5277ae9e379bb4fd67c221be5cb20426) - use snake_case for module tokens for consistency *(commit by [@JakeStanger](https://github.com/JakeStanger))*:

  This renames the module from `sys-info` to `sys_info`, and almost every formatting token from `kebab-case` to `snake_case`. Any use of this module will need to be updated.

- due to [`8c75bc4`](https://github.com/JakeStanger/ironbar/commit/8c75bc46ac2885a748d31df9261d988cc797e916) - rename `path` to `cmd` for consistency *(commit by [@JakeStanger](https://github.com/JakeStanger))*:

  This changes the option in the `script` module. Any uses of the module must be updated to use the new option name.

- due to [`e274ba3`](https://github.com/JakeStanger/ironbar/commit/e274ba39cd6d8f1c73033ac1e60e5bce89205ce2) - rename `exec` to `on_click` for consistency *(commit by [@JakeStanger](https://github.com/JakeStanger))*:

  This changes the option on buttons in the `custom` module. Any uses of the module must be updated to use the new custom widget attribute name.


### :sparkles: New Features
- [`73158c2`](https://github.com/JakeStanger/ironbar/commit/73158c2fce2880347b88d58541dea000534996c8) - **script**: new watch mode *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`a3f90ad`](https://github.com/JakeStanger/ironbar/commit/a3f90adaf19aebed7020eeb44b91250af080d313) - add nix flake support *(commit by [@yavko](https://github.com/yavko))*
- [`c9e66d4`](https://github.com/JakeStanger/ironbar/commit/c9e66d4664137c50aba4aecdc3a3ba43d3da11fe) - common module options (`show_if`, `on_click`, `tooltip`) *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`5d153a0`](https://github.com/JakeStanger/ironbar/commit/5d153a02fc9b113bb77a04596b806edd182fc5d3) - **custom**: ability to embed scripts in labels for dynamic content *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`d20972c`](https://github.com/JakeStanger/ironbar/commit/d20972cb32714627d0cca947021453979c76dd03) - dynamic tooltips *(commit by [@JakeStanger](https://github.com/JakeStanger))*

### :recycle: Refactors
- [`ff17ec1`](https://github.com/JakeStanger/ironbar/commit/ff17ec1996cf344663e84e79d11b08dc84b97635) - various changes based on rust 1.65 clippy *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`4662f60`](https://github.com/JakeStanger/ironbar/commit/4662f60ac54165be6fb7aea12c245309db0fe5d6) - move various clients to own folder *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`0fb5fa8`](https://github.com/JakeStanger/ironbar/commit/0fb5fa8c2a166c3d46b006ceb0d53af076824ff4) - use latest `libcorn` with serde support *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`df77020`](https://github.com/JakeStanger/ironbar/commit/df77020c5277ae9e379bb4fd67c221be5cb20426) - **sys_info**: use snake_case for module tokens for consistency *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`8c75bc4`](https://github.com/JakeStanger/ironbar/commit/8c75bc46ac2885a748d31df9261d988cc797e916) - **script**: rename `path` to `cmd` for consistency *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`e274ba3`](https://github.com/JakeStanger/ironbar/commit/e274ba39cd6d8f1c73033ac1e60e5bce89205ce2) - **custom**: rename `exec` to `on_click` for consistency *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`64f5404`](https://github.com/JakeStanger/ironbar/commit/64f54040ef626157af6b6a9ce5258507a10a23fb) - move dynamic_label.rs to dynamic_string.rs and fix failing test *(commit by [@JakeStanger](https://github.com/JakeStanger))*

### :white_check_mark: Tests
- [`907a565`](https://github.com/JakeStanger/ironbar/commit/907a565f3d418a276dfb454e1189ddede1814291) - **dynamic label**: do not run if cannot initialise gtk *(commit by [@JakeStanger](https://github.com/JakeStanger))*

### :memo: Documentation Changes
- [`1c032ae`](https://github.com/JakeStanger/ironbar/commit/1c032ae8e3a38b82c286bab7d102842f14b708e1) - update CHANGELOG.md for v0.7.0 [skip ci] *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`58d55db`](https://github.com/JakeStanger/ironbar/commit/58d55db6600fe2f9b23ae8ec6a50a686d2acaf65) - migrate wiki into main repo *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`c480296`](https://github.com/JakeStanger/ironbar/commit/c48029664d5f58bf73faa2931f34b38b8b184d25) - **script**: improve doc comment *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`8c77410`](https://github.com/JakeStanger/ironbar/commit/8c774100f1c8ea051284c6950339a2c8ed59a52a) - **script**: add information on new mode options *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`c4cdf4b`](https://github.com/JakeStanger/ironbar/commit/c4cdf4be8ba83f3669158a1552eab4a840085204) - update example configs *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`ec69649`](https://github.com/JakeStanger/ironbar/commit/ec69649a04f6199953836e51c2efe1fe2a19e320) - update example configs *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`1320639`](https://github.com/JakeStanger/ironbar/commit/1320639d4e6b7c8cd8f861b26b2b854504775ef0) - add custom power menu example *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`afedf02`](https://github.com/JakeStanger/ironbar/commit/afedf0214d3a71f6283c70bd3a110d24f68d2fdf) - add link to new custom power menu example *(commit by [@JakeStanger](https://github.com/JakeStanger))*


## [v0.7.0] - 2022-11-05
### :sparkles: New Features
- [`fad90fd`](https://github.com/JakeStanger/ironbar/commit/fad90fdad683a612497ac7822a66a90f43fce0a2) - **sys-info**: add loads more formatting tokens *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`dec402e`](https://github.com/JakeStanger/ironbar/commit/dec402edd9d6c5b8677ff337699ad99ebc69b776) - **sys-info**: config options for refresh intervals *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`91c57ed`](https://github.com/JakeStanger/ironbar/commit/91c57edc73f15397ea0de70c4a6a6532c35caf2a) - **sys-info**: pango markup support *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`ec1d596`](https://github.com/JakeStanger/ironbar/commit/ec1d59677b13c9654a98d78f909ba2d0fcfbb72d) - **logging**: `IRONBAR_LOG` and `IRONBAR_FILE_LOG` env vars *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`493df6b`](https://github.com/JakeStanger/ironbar/commit/493df6bb49fec8c465706d3f9b395728ba73a621) - **mpd**: add volume slider to popup *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`3750124`](https://github.com/JakeStanger/ironbar/commit/3750124d8cfb4783932a6b3359384f245fcd2394) - new custom module *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`b7792a4`](https://github.com/JakeStanger/ironbar/commit/b7792a415e09fc535750ea5af530f91aa791c4bc) - env var to set custom css location *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`ad77dc4`](https://github.com/JakeStanger/ironbar/commit/ad77dc4e4c2f80fcb4c9604c796be0f981e895ee) - improved logging & error handling *(commit by [@JakeStanger](https://github.com/JakeStanger))*

### :bug: Bug Fixes
- [`9e6dbbd`](https://github.com/JakeStanger/ironbar/commit/9e6dbbd131a09f101b0d490265fe7d4ec564e38c) - **sys-info**: tokens not replaced if more than one in string *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`f17ae7a`](https://github.com/JakeStanger/ironbar/commit/f17ae7a415b931c64942de085e8889f37b3f9b11) - **script**: not parsing pango markup *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`b66bd78`](https://github.com/JakeStanger/ironbar/commit/b66bd788b23256a2127a1352693fdd3f929d9c4b) - logging for creating bar incorrect still *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`3c43c20`](https://github.com/JakeStanger/ironbar/commit/3c43c20c6ae53a9aa6b67770b0c489806784f4ac) - weird behaviour when config does not exist *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`70e1b52`](https://github.com/JakeStanger/ironbar/commit/70e1b526a9681b16545d7f05d77470d76bd8819e) - **logging**: file log not capturing panics *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`cbd0c49`](https://github.com/JakeStanger/ironbar/commit/cbd0c49e251b5c8e0289ca6200a393d89994992d) - css watcher not working *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`e693c1c`](https://github.com/JakeStanger/ironbar/commit/e693c1c166eef0b5edcdcd033bb12d572e4e5f04) - **mpd**: volume slider causing mpd server errors *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`3a83bd3`](https://github.com/JakeStanger/ironbar/commit/3a83bd31ab165869f7f274b054b2f16485261fd1) - able to insert duplicate keys into collection *(commit by [@JakeStanger](https://github.com/JakeStanger))*

### :recycle: Refactors
- [`5ebc84c`](https://github.com/JakeStanger/ironbar/commit/5ebc84c7b98cc648a659ca37fdc0f041057f0ea4) - **logging**: consts for default log levels *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`bc625b9`](https://github.com/JakeStanger/ironbar/commit/bc625b929b8644ce92f275b5d98cdf74b93fe067) - clippy & fmt *(commit by [@JakeStanger](https://github.com/JakeStanger))*

### :memo: Documentation Changes
- [`9d9c275`](https://github.com/JakeStanger/ironbar/commit/9d9c2753137331ae85ac8ab7d75a6de9a9c82042) - update CHANGELOG.md for v0.6.0 [skip ci] *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`27d0479`](https://github.com/JakeStanger/ironbar/commit/27d04795af1c25fe5f765c7480d5dd5d096a8ab7) - **readme**: add warning about crate being outdated *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`a06c4bc`](https://github.com/JakeStanger/ironbar/commit/a06c4bccca6cb51935605ac9239e63024fb7c663) - **examples**: add full system info config *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`0a331f3`](https://github.com/JakeStanger/ironbar/commit/0a331f31381f0d967793c0d8b7a14e2a43bf666f) - **readme**: remove warning about outdated cargo package *(commit by [@JakeStanger](https://github.com/JakeStanger))*


## [v0.6.0] - 2022-10-15
### :sparkles: New Features
- [`b188bc7`](https://github.com/JakeStanger/ironbar/commit/b188bc714614406935d8bb88a719adab2dfce32f) - initial support for running outside sway *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`324f00c`](https://github.com/JakeStanger/ironbar/commit/324f00cdf9200e3e3ecedfa68ab4c99b170242e2) - wlroots-agnostic support for `focused` module *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`b1c66b9`](https://github.com/JakeStanger/ironbar/commit/b1c66b9117cf8a10350cdb857a5267a1a72ad914) - wlroots-agnostic support for `launcher` module *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`1dd0a9e`](https://github.com/JakeStanger/ironbar/commit/1dd0a9e52f69e672d9ac313c1da0e201c911e6c2) - **launcher**: add popup css selectors *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`06cfad6`](https://github.com/JakeStanger/ironbar/commit/06cfad62e228f7fc63938f2280206450005cb064) - more positioning options *(PR [#23](https://github.com/JakeStanger/ironbar/pull/23) by [@JakeStanger](https://github.com/JakeStanger))*

### :bug: Bug Fixes
- [`5523e9a`](https://github.com/JakeStanger/ironbar/commit/5523e9af46e457f9d45902debaaacf26b586e457) - **popup**: often opening in wrong place *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`8536ad7`](https://github.com/JakeStanger/ironbar/commit/8536ad719a92aec4166e35b75cb029075ad3ae34) - **mpd**: incorrectly checking for unix sockets *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`bd5bdf5`](https://github.com/JakeStanger/ironbar/commit/bd5bdf5af548304958663d593fccb454afa6c8ff) - logging for creating bar incorrect *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`75339f0`](https://github.com/JakeStanger/ironbar/commit/75339f07ed164fa94838036a604a1dcb6d53564c) - vertical bars ignoring height config option *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`b7b6488`](https://github.com/JakeStanger/ironbar/commit/b7b64886e3c48ace3faffbb1e277275aeeac3adf) - sometimes panicking on startup *(commit by [@JakeStanger](https://github.com/JakeStanger))*

### :recycle: Refactors
- [`5ce50b0`](https://github.com/JakeStanger/ironbar/commit/5ce50b0987812a1ade2d1262e8d7df6916cfc39a) - tidy and format *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`1b853bc`](https://github.com/JakeStanger/ironbar/commit/1b853bcb71197a4bf3ca75725cc010b1d404c2b3) - fix clippy warning *(commit by [@JakeStanger](https://github.com/JakeStanger))*

### :memo: Documentation Changes
- [`daafa09`](https://github.com/JakeStanger/ironbar/commit/daafa0943e5b9886b09fd18d6fff04558fb02335) - update CHANGELOG.md for v0.5.2 [skip ci] *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`b352181`](https://github.com/JakeStanger/ironbar/commit/b352181b3d232ccc79ffc1d9e22a633729d01a47) - update json example *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`bb4fe7f`](https://github.com/JakeStanger/ironbar/commit/bb4fe7f7f58fa2a6d0a2259bd9442700d2c884f7) - **readme**: credit smithay client toolkit *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`994d0f5`](https://github.com/JakeStanger/ironbar/commit/994d0f580b4d1b6ff750839652a7f06149743172) - **readme**: update references to sway *(commit by [@JakeStanger](https://github.com/JakeStanger))*

### :boom: BREAKING CHANGES
- due to [`06cfad6`](https://github.com/JakeStanger/ironbar/commit/06cfad62e228f7fc63938f2280206450005cb064) - more positioning options *(PR [#23](https://github.com/JakeStanger/ironbar/pull/23) by [@JakeStanger](https://github.com/JakeStanger))*:

  The `left` and `right` config options have been renamed to `start` and `end`


## [v0.5.2] - 2022-09-07
### :wrench: Chores
- [`b801751`](https://github.com/JakeStanger/ironbar/commit/b801751bdabd8416084f46e6b6d803ea28a259ec) - **release**: v0.5.2 *(commit by [@JakeStanger](https://github.com/JakeStanger))*


## [v0.5.1] - 2022-09-06
### :bug: Bug Fixes
- [`b81927e`](https://github.com/JakeStanger/ironbar/commit/b81927e3a57808188e31419695a36aa4ea3f2830) - **launcher**: opening new instances when focused/urgent *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`a35d255`](https://github.com/JakeStanger/ironbar/commit/a35d25520cd3fd235cdc77ec6209d88499ca3639) - **launcher**: item state changes not handled correctly *(commit by [@JakeStanger](https://github.com/JakeStanger))*

### :wrench: Chores
- [`481adfc`](https://github.com/JakeStanger/ironbar/commit/481adfcaa41c0d3a1ba7d61edb68db49d959c78f) - **intellij**: update run configs *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`6871126`](https://github.com/JakeStanger/ironbar/commit/6871126bd8def89ccbf2934180d615e781ec32c7) - **release**: v0.5.1 *(commit by [@JakeStanger](https://github.com/JakeStanger))*


## [v0.5.0] - 2022-08-25
### :sparkles: New Features
- [`1e38719`](https://github.com/JakeStanger/ironbar/commit/1e387199962b81caeb40ffbd99a956f24abdf4e3) - introduce logging in some areas *(commit by [@JakeStanger](https://github.com/JakeStanger))*

### :bug: Bug Fixes
- [`023c2fb`](https://github.com/JakeStanger/ironbar/commit/023c2fb118f46f3592f1dfe1a6704014c062ab3f) - **workspaces**: not listening to move event *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`6dcae66`](https://github.com/JakeStanger/ironbar/commit/6dcae66570cf5434e077ec823cded33771b4239c) - avoid creating loads of sway/mpd clients *(commit by [@JakeStanger](https://github.com/JakeStanger))*

### :wrench: Chores
- [`015dcd3`](https://github.com/JakeStanger/ironbar/commit/015dcd3204dfa6a1ebcef1b4f3b345ed733fee2f) - **release**: v0.5.0 *(commit by [@JakeStanger](https://github.com/JakeStanger))*


## [v0.4.0] - 2022-08-22
### :sparkles: New Features
- [`ab8f7ec`](https://github.com/JakeStanger/ironbar/commit/ab8f7ecfc8fa4b96fce78518af75794641950140) - logging support and proper error handling *(commit by [@JakeStanger](https://github.com/JakeStanger))*

### :bug: Bug Fixes
- [`f2ee2df`](https://github.com/JakeStanger/ironbar/commit/f2ee2dfe7a0f5575d0c3ec09644ca990b088cd85) - error when using with `swaybar_command` *(commit by [@JakeStanger](https://github.com/JakeStanger))*

### :wrench: Chores
- [`1d7c377`](https://github.com/JakeStanger/ironbar/commit/1d7c3772e4b97c7198043cb55fe9c71695a211ab) - **release**: v0.4.0 *(commit by [@JakeStanger](https://github.com/JakeStanger))*


[v0.4.0]: https://github.com/JakeStanger/ironbar/compare/v0.3.0...v0.4.0
[v0.5.0]: https://github.com/JakeStanger/ironbar/compare/v0.4.0...v0.5.0
[v0.5.1]: https://github.com/JakeStanger/ironbar/compare/v0.5.0...v0.5.1
[v0.5.2]: https://github.com/JakeStanger/ironbar/compare/v0.5.1...v0.5.2
[v0.6.0]: https://github.com/JakeStanger/ironbar/compare/v0.5.2...v0.6.0
[v0.7.0]: https://github.com/JakeStanger/ironbar/compare/v0.6.0...v0.7.0
[v0.8.0]: https://github.com/JakeStanger/ironbar/compare/v0.7.0...v0.8.0