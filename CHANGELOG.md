# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [v0.10.0] - 2023-02-01
### :boom: BREAKING CHANGES
- due to [`3cf9be8`](https://github.com/JakeStanger/ironbar/commit/3cf9be89fd74face31806165f66b68052b093bab) - global icon theme setting *(commit by [@JakeStanger](https://github.com/JakeStanger))*:

  This removes the `icon_theme` option from `launcher` and `focused`. You will need to set this at the top of your config instead.

- due to [`90f57d6`](https://github.com/JakeStanger/ironbar/commit/90f57d61b94c50c98a6f55de18c6edf3d18aa3fa) - remove irrelevant `icon` format token *(commit by [@JakeStanger](https://github.com/JakeStanger))*:

  (Missed from #96141d4) The `{icon}` token has been removed from the `music` module due to incompatibility with the new image/icon support. The icon now always displays as a separate widget before the label and should be removed from your formatting string.


### :sparkles: New Features
- [`8691824`](https://github.com/JakeStanger/ironbar/commit/8691824db1a12c3f3589ff8b5315b8dba5cb8aec) - **music**: ability to truncate button text *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`07dbf78`](https://github.com/JakeStanger/ironbar/commit/07dbf780105027b533b0bb34c9ae3e4e96f29f4a) - **focused**: ability to truncate label text *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`393800a`](https://github.com/JakeStanger/ironbar/commit/393800aaa2093b9257c43fde8bdb8399f26ebc74) - **custom**: image widget *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`3cf9be8`](https://github.com/JakeStanger/ironbar/commit/3cf9be89fd74face31806165f66b68052b093bab) - global icon theme setting *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`b054c17`](https://github.com/JakeStanger/ironbar/commit/b054c17d14628c9188bfa9aed506ea1de3051f9c) - **workspaces**: support for using images in `name_map` *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`96141d4`](https://github.com/JakeStanger/ironbar/commit/96141d49907412ea26d23ef30c10ade8b32b89b9) - **music**: support for using images in `name_map`, additional icon options *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`c347b6c`](https://github.com/JakeStanger/ironbar/commit/c347b6c9449ce4e16e2e133d7dd35544ab9a533c) - add feature flags *(commit by [@JakeStanger](https://github.com/JakeStanger))*

### :bug: Bug Fixes
- [`5772711`](https://github.com/JakeStanger/ironbar/commit/57727111923a419f9b7613103283aa4cf6bd082c) - **music**: remote mpris album art not showing *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`5fb4125`](https://github.com/JakeStanger/ironbar/commit/5fb412572f3da60ac482a1960d891f70bc29287b) - **tray**: some init issues *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`058c8f4`](https://github.com/JakeStanger/ironbar/commit/058c8f4228f9f7faa66cda9dd1636ea32e9de68b) - **hyprland**: issues with tracking workspaces *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`51d2c22`](https://github.com/JakeStanger/ironbar/commit/51d2c2279f50add992def0d58cfaa9890ea3d041) - **images**: incorrectly resolving non-files *(commit by [@JakeStanger](https://github.com/JakeStanger))*

### :recycle: Refactors
- [`012762e`](https://github.com/JakeStanger/ironbar/commit/012762e10203fb2d58160acdae4dc7ca7689b131) - swap out some code for existing macros *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`9750255`](https://github.com/JakeStanger/ironbar/commit/97502559b30c51e77c1dd9a7d794a88756294c83) - **music**: split config code into separate file *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`15f0857`](https://github.com/JakeStanger/ironbar/commit/15f0857859d5d4a590b60b6b1a4347b4b84a58a1) - replace icon loading with improved general image loading *(commit by [@JakeStanger](https://github.com/JakeStanger))*

### :memo: Documentation Changes
- [`1ed3220`](https://github.com/JakeStanger/ironbar/commit/1ed3220733c2dcb7c5e5cbf377b3324d3183609e) - update CHANGELOG.md for v0.9.0 [skip ci] *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`90f57d6`](https://github.com/JakeStanger/ironbar/commit/90f57d61b94c50c98a6f55de18c6edf3d18aa3fa) - **music**: remove irrelevant `icon` format token *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`6a39905`](https://github.com/JakeStanger/ironbar/commit/6a39905b4333582fbcda81a66a9b91055333d698) - **compiling**: add missing full stop *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`7b23e61`](https://github.com/JakeStanger/ironbar/commit/7b23e61e7dedf2736a30580b6c1aa84e002c462c) - **wiki**: update screenshots and examples *(commit by [@JakeStanger](https://github.com/JakeStanger))*


## [v0.9.0] - 2023-01-28
### :boom: BREAKING CHANGES
- due to [`fa67d07`](https://github.com/JakeStanger/ironbar/commit/fa67d077b136b109edf6dbaa11a33aebf3e044b4) - mouse event config options *(commit by [@JakeStanger](https://github.com/JakeStanger))*:

  `on_click` is now called `on_click_left` for consistency with new options.

- due to [`6d8e647`](https://github.com/JakeStanger/ironbar/commit/6d8e647f123e54ba389c5ab2fe908200aa5e4cf6) - mpris support *(commit by [@JakeStanger](https://github.com/JakeStanger))*:

  The `mpd` module has been renamed to `music`. You will need to update the `type` value in your config and add `player_type` to continue using MPD. You will also need to update your styles.


### :sparkles: New Features
- [`1dd5863`](https://github.com/JakeStanger/ironbar/commit/1dd586343143bfd501a44c6556719fac9d582d6b) - better surface some config error messages *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`fa67d07`](https://github.com/JakeStanger/ironbar/commit/fa67d077b136b109edf6dbaa11a33aebf3e044b4) - mouse event config options *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`6d8e647`](https://github.com/JakeStanger/ironbar/commit/6d8e647f123e54ba389c5ab2fe908200aa5e4cf6) - mpris support *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`6e5d0c1`](https://github.com/JakeStanger/ironbar/commit/6e5d0c1e8c0b5d7e330608fc835e1e9733f156de) - **workspaces**: hyprland support *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`9ba28fe`](https://github.com/JakeStanger/ironbar/commit/9ba28fe7faf84e06febc2ffea089442f8f5b90a2) - **workspaces**: better ordering *(commit by [@JakeStanger](https://github.com/JakeStanger))*

### :bug: Bug Fixes
- [`e1f523c`](https://github.com/JakeStanger/ironbar/commit/e1f523cf2a15b74a5c570dd7440db4c1b476d782) - **music**: popup artist label using wrong name *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`08cfbbc`](https://github.com/JakeStanger/ironbar/commit/08cfbbc2eaf6e74780dd7196efcc15ea6d2e7d12) - **music**: unable to go to prev with mpris *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`0cefcbd`](https://github.com/JakeStanger/ironbar/commit/0cefcbd02b0af518352e35060644f281da249d3e) - **music**: wrong widget name on vol slider *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`90cd078`](https://github.com/JakeStanger/ironbar/commit/90cd078973b23b2291cf156e46729842f33c1806) - **mpd**: stops working if connection lost *(commit by [@JakeStanger](https://github.com/JakeStanger))*

### :recycle: Refactors
- [`2c1b292`](https://github.com/JakeStanger/ironbar/commit/2c1b2924d4a103183d3974ac066623a80277a79a) - move most of the horrible `add_module` macro content into proper functions *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`fd2d7e5`](https://github.com/JakeStanger/ironbar/commit/fd2d7e5c7ab8de50c4621b19d07d8b012a451564) - move startup logging code to logging module *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`9d5049d`](https://github.com/JakeStanger/ironbar/commit/9d5049dde01cdb76f4772f8ce8f61a8b5bad3a50) - standardise error messages *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`5e21cbc`](https://github.com/JakeStanger/ironbar/commit/5e21cbcca6cc30d725acdea0f6561cfd6acdcc3c) - macros to reduce repeated code *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`ea2c84d`](https://github.com/JakeStanger/ironbar/commit/ea2c84d1bd15798e32496397c4a6aa42fab39d95) - general code tidy-up *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`0d7ab54`](https://github.com/JakeStanger/ironbar/commit/0d7ab541604691455ed39c73e039ac0635307bc8) - remove redundant clone *(commit by [@JakeStanger](https://github.com/JakeStanger))*

### :memo: Documentation Changes
- [`b97f018`](https://github.com/JakeStanger/ironbar/commit/b97f018e81aa55a871a12aa3e1e4b07b1f8eb50f) - update CHANGELOG.md for v0.8.0 [skip ci] *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`c223892`](https://github.com/JakeStanger/ironbar/commit/c223892a57b29ae56431fc585b8cec503f3206c7) - **workspaces**: update for hyprland/new ordering option *(commit by [@JakeStanger](https://github.com/JakeStanger))*


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
[v0.9.0]: https://github.com/JakeStanger/ironbar/compare/v0.8.0...v0.9.0
[v0.10.0]: https://github.com/JakeStanger/ironbar/compare/v0.9.0...v0.10.0