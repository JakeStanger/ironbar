# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [v0.16.0] - 2024-08-10
### :boom: BREAKING CHANGES
- due to [`9dd7112`](https://github.com/JakeStanger/ironbar/commit/9dd711235f21d9016fec240f1be5c8d6de1596df) - improve CLI structure, add new commands *(commit by [@JakeStanger](https://github.com/JakeStanger))*:

  - `ok_value` responses will no longer print `ok` as the first line when using the CLI
  - All IPC commands have changed. Namely, `type` has been changed to `command`, and bar/var related commands are now under a `subcommand`. The full spec can be found on the wiki.
  - Several CLI commands are now located under the `var` and `bar` categories. Usage of any commands to get/set Ironvars or control bar visibility will need to be updated.
  - The `open_popup` and `close_popup` IPC commands are now called `show_popup` and `hide_popup` respectively.
  - The popup `name` argument has been renamed to `widget_name` on all IPC commands.
  - The `set-visibility` CLI command now takes a `true`/`false` positional argument in place of the `-v` flag.


### :sparkles: New Features
- [`f11da3e`](https://github.com/JakeStanger/ironbar/commit/f11da3eca1b7d1bc5e1904266f285f0e28f290a0) - **music**: pango markup support *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`951576c`](https://github.com/JakeStanger/ironbar/commit/951576ce3c092d187fd6d1d2ff55b7dbf6198a25) - pango markup support in image icons *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`36d724f`](https://github.com/JakeStanger/ironbar/commit/36d724f148ed8ebe84cbb3c3e25cd4a361d94e66) - **config**: json schema support *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`7413f78`](https://github.com/JakeStanger/ironbar/commit/7413f78e04fe9b532397144e49b7545547980723) - **cli**: debug flag *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`a33e0a2`](https://github.com/JakeStanger/ironbar/commit/a33e0a241a8d5f65f7360b5c7e34a116f3f9f092) - **cli**: format flag, json output format *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`9dd7112`](https://github.com/JakeStanger/ironbar/commit/9dd711235f21d9016fec240f1be5c8d6de1596df) - improve CLI structure, add new commands *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`aa45396`](https://github.com/JakeStanger/ironbar/commit/aa4539606241cfd4d7b8e5512866d30ce92e432d) - ability to set bar layer and exclusive zone *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`92ae1a8`](https://github.com/JakeStanger/ironbar/commit/92ae1a8d73d68ebf51e008cd6322a9269cd10325) - **nix**: home-manager option to read style.css file *(commit by [@Alpha-Ursae-Minoris](https://github.com/Alpha-Ursae-Minoris))*
- [`6d0fe4c`](https://github.com/JakeStanger/ironbar/commit/6d0fe4c8ace9c8a4136fb65c9ff9cdb04e9b6408) - add networkmanager module *(commit by [@Zedfrigg](https://github.com/Zedfrigg))*
- [`e307e15`](https://github.com/JakeStanger/ironbar/commit/e307e15dc4462d1bdcaabff2375f5ac0c5a5df7b) - new sway-mode module *(PR [#671](https://github.com/JakeStanger/ironbar/pull/671) by [@Rodrigodd](https://github.com/Rodrigodd))*

### :bug: Bug Fixes
- [`5e7f576`](https://github.com/JakeStanger/ironbar/commit/5e7f576841f94bdfd89d401cb9a2ba1fabb45c1c) - **workspaces**: add support for hyprland rename event *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`c45ea02`](https://github.com/JakeStanger/ironbar/commit/c45ea02a7d39b30fece3986044a44a64aebf5e16) - **workspaces**: regression due to [#572](https://github.com/JakeStanger/ironbar/pull/572) *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`4a37429`](https://github.com/JakeStanger/ironbar/commit/4a37429634a32a2ffaeb1b591240bdb2a564cab9) - **launcher**: ghost windows in reload *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`520a94a`](https://github.com/JakeStanger/ironbar/commit/520a94abfa526c22df0bebecc42b9be8ae20881e) - all bars showing on same display due to GTK bug *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`4ad4b0e`](https://github.com/JakeStanger/ironbar/commit/4ad4b0e070cc4e271251763db7210e70857d68ca) - **ipc**: regression - reload not working due to [#592](https://github.com/JakeStanger/ironbar/pull/592) *(commit by [@SerraPi](https://github.com/SerraPi))*
- [`9a39420`](https://github.com/JakeStanger/ironbar/commit/9a39420ae28b185cb38a33817f1fc91b5c4e9f55) - **launcher**: favourites staying focused when closed in hyprland *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`8dda494`](https://github.com/JakeStanger/ironbar/commit/8dda49477b2ceb268b94c729aadc5986bdca8528) - **cli**: using zero exit code for error responses *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`9d8a3eb`](https://github.com/JakeStanger/ironbar/commit/9d8a3eb370195321d224c0a51a6752c62404ac1b) - correctly escape pango markup *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`277e6b6`](https://github.com/JakeStanger/ironbar/commit/277e6b62965608ae90defa9a2170d414e09d4c59) - **notifications**: unable to click through overlay *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`dbd385d`](https://github.com/JakeStanger/ironbar/commit/dbd385d225e27a7d732d60ba5a6d6f13c1184add) - **launcher**: apps with multiple windows stay focused when window closed *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`176af99`](https://github.com/JakeStanger/ironbar/commit/176af997f442833adcd7ada1919836d54530d7ef) - **music**: tokens with `&` not rendering *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`7e04e30`](https://github.com/JakeStanger/ironbar/commit/7e04e30171a1897de468592fe5c1f6082d12eb69) - **wayland**: exit on event dispatch error *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`461bee8`](https://github.com/JakeStanger/ironbar/commit/461bee8847590e769df186a2f24ab2ce957568f7) - **bar**: do not add start/center/end containers if empty *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`fb6ae42`](https://github.com/JakeStanger/ironbar/commit/fb6ae42f3bcc7ad35066e1182e617c739a8cfa8a) - crash due to clipboard fd incorrectly closed *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`2bc741d`](https://github.com/JakeStanger/ironbar/commit/2bc741d197867cd5f0c391b9532b4cf9c4d378f6) - **tray**: crash when provided empty pixmap *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`f819aec`](https://github.com/JakeStanger/ironbar/commit/f819aec259cfe418f050c57eb51a236a95039b57) - **notifications**: client broken by recent refactor *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`45d5bf5`](https://github.com/JakeStanger/ironbar/commit/45d5bf5feb88d0854a41faa5890b56188b3e051c) - popups not accounting for monitor scaling *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`474e1fe`](https://github.com/JakeStanger/ironbar/commit/474e1fe364ef70fa0afcff476034d5f307dcd54b) - **upower**: avoid panic on client init error *(commit by [@JakeStanger](https://github.com/JakeStanger))*

### :recycle: Refactors
- [`04a694e`](https://github.com/JakeStanger/ironbar/commit/04a694e2ad5998e82de8dd121cc2b486432c0a70) - fix latest clippy warnings *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`c876904`](https://github.com/JakeStanger/ironbar/commit/c876904bda7bb51ef2d3ec1661281df75fad60be) - update `nix` crate to latest version *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`dedb89b`](https://github.com/JakeStanger/ironbar/commit/dedb89bb027c4477620410d9103d64c3f2668517) - **popup**: rename `is_visible` to `visible` *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`a0cb01a`](https://github.com/JakeStanger/ironbar/commit/a0cb01ae5f2121581eb90f73b8f661862da12b03) - make `Ironbar#unique_id` `must_use` *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`b8fdd85`](https://github.com/JakeStanger/ironbar/commit/b8fdd8531e5516598f81e869b9284b8888f1d06b) - explicitly use `set_text` on non-pango labels *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`9d12535`](https://github.com/JakeStanger/ironbar/commit/9d125353c45a7a8ce3fee43192364745a3fba931) - small tidy *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`1899757`](https://github.com/JakeStanger/ironbar/commit/189975791f6424eca85fcfd76b796e5e9f9fb47f) - **mpris**: better logging, avoid panic on dbus error *(commit by [@JakeStanger](https://github.com/JakeStanger))*

### :memo: Documentation Changes
- [`c25440c`](https://github.com/JakeStanger/ironbar/commit/c25440cd3274cb7adf4e8a1c97b4bc88a53841b4) - update CHANGELOG.md for v0.15.1 [skip ci] *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`f7f991b`](https://github.com/JakeStanger/ironbar/commit/f7f991b2e68a19ff66387913b54127fd8808cc21) - **compiling**: fix wrong fedora package for pulse libs *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`c7743b2`](https://github.com/JakeStanger/ironbar/commit/c7743b28c68919e5bb1d8b9c53d63fb53fd3b081) - add rustdoc comments to all module options *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`7d19106`](https://github.com/JakeStanger/ironbar/commit/7d191065fc20e64befca64e8814aa86b2c654a7c) - add notes about nerd fonts *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`9db0cbc`](https://github.com/JakeStanger/ironbar/commit/9db0cbcbdc561ba929c300cec92156c873c3c151) - **upower**: fix incorrect css selectors *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`057fdff`](https://github.com/JakeStanger/ironbar/commit/057fdffc5f3219b60bbc1f095f88a9d8e3e8f750) - **examples**: fix incorrect cpu sensor name *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`076c2df`](https://github.com/JakeStanger/ironbar/commit/076c2df4a29bb3af2183dc1617f101e1e39d3fa4) - add fedora copr package to readme *(commit by [@victorvintorez](https://github.com/victorvintorez))*
- [`860a676`](https://github.com/JakeStanger/ironbar/commit/860a6767f144610d6c1809ddadd52e31c8d8d68d) - **upower**: add note to make clear upower is required *(commit by [@JakeStanger](https://github.com/JakeStanger))*


## [v0.15.1] - 2024-05-05

Release to bump hyprland-rs version due to Hyprland v0.40 socket path breaking change.

### :memo: Documentation Changes
- [`47b6c47`](https://github.com/JakeStanger/ironbar/commit/47b6c477242ad52aae77a6820740d9c5f4bfc263) - **compiling**: add lua deps *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`1971f3b`](https://github.com/JakeStanger/ironbar/commit/1971f3bb1ef3d059b29b99527e77ffaaf92240aa) - **volume**: update deprecated volume token *(PR [#567](https://github.com/JakeStanger/ironbar/pull/567) by [@drendog](https://github.com/drendog))*


## [v0.15.0] - 2024-04-28
### :sparkles: New Features
- [`f4384b6`](https://github.com/JakeStanger/ironbar/commit/f4384b6252e86d4e2558e1c36810d4ef903bd58c) -  enable use of markup in clock module format and format_popup, and update documentation to reflect supporting Pango markup in both *commit by [@Dridus](https://github.com/Dridus))*
- [`76a6816`](https://github.com/JakeStanger/ironbar/commit/76a68165f09a6d07f8e95008cb9fe3d40d99abe0) - **upower**: add new formatting properties *(commit by [@Disr0](https://github.com/Disr0))*
- [`b037a55`](https://github.com/JakeStanger/ironbar/commit/b037a55fb78d05cce0e03bad27a10cbdf743c573) - **tray**: add `direction` option *(commit by [@calops](https://github.com/calops))*
- [`72440e6`](https://github.com/JakeStanger/ironbar/commit/72440e69c9e665f3e82e569e770747fc63765b53) - **tray**: icon size setting *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`a70956b`](https://github.com/JakeStanger/ironbar/commit/a70956bb3b17f559fda1fdca444e271ae9d3c4cd) - add new volume module *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`7742a46`](https://github.com/JakeStanger/ironbar/commit/7742a465780ed5db80cdb518a834200082a5e936) - swaync notifications module *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`ba00445`](https://github.com/JakeStanger/ironbar/commit/ba004455b25fb51d28a5ec0cdf0f510c2157eb94) - **tray**: option to prefer theme-provided icons *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`994f4a4`](https://github.com/JakeStanger/ironbar/commit/994f4a4a123452607dd591e1e358ec218a3cb5ae) - ability to add custom modules instead native modules *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`46cbaca`](https://github.com/JakeStanger/ironbar/commit/46cbaca5e08a5be8945486d007c0f7315d10b351) - option to disable module popup *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`46224d8`](https://github.com/JakeStanger/ironbar/commit/46224d8a541699a04b2311e89766dded781863d6) - **custom**: ability to add modules/widgets to buttons *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`702b0a6`](https://github.com/JakeStanger/ironbar/commit/702b0a63bf75204d03f9229f1667cb2e77c1b8b8) - Add orientation support for clock *(commit by [@ClaireNeveu](https://github.com/ClaireNeveu))*
- [`70b2c59`](https://github.com/JakeStanger/ironbar/commit/70b2c592b284965382182098b0b90b40bdac9965) - Add orientation support for custom label and button *(commit by [@ClaireNeveu](https://github.com/ClaireNeveu))*
- [`44be585`](https://github.com/JakeStanger/ironbar/commit/44be58594b296ff6a1a7d902c88aa01116322538) - Add orientation and direction support for sys info *(commit by [@ClaireNeveu](https://github.com/ClaireNeveu))*
- [`cfaba87`](https://github.com/JakeStanger/ironbar/commit/cfaba87f2fe470667eea4eca0504f6e8651c90f3) - **ipc**: ironvar list command *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`b0a05b7`](https://github.com/JakeStanger/ironbar/commit/b0a05b7cda1d07af6673a5ee9fb8105ed1497a36) - new cairo module *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`d03c752`](https://github.com/JakeStanger/ironbar/commit/d03c752f9a0ac849fe3f1a93d7c3de4f743c7f00) - **launcher**: option to reverse order *(commit by [@SerraPi](https://github.com/SerraPi))*

### :bug: Bug Fixes
- [`30b11db`](https://github.com/JakeStanger/ironbar/commit/30b11db43503f4a78fde8f17fa3af6ce99375cc2) - **tray**: cannot activate menu options with right click *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`f68d95a`](https://github.com/JakeStanger/ironbar/commit/f68d95a740c02434866c662d2cd915a0c5253ba5) - **logging**: log file growing indefinitely *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`6fe9c54`](https://github.com/JakeStanger/ironbar/commit/6fe9c541347b7bdd69e3d735f07a17a5d4b124ca) - **clipboard**: unable to paste large images into xwayland *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`a10466e`](https://github.com/JakeStanger/ironbar/commit/a10466e7e9dafd29e80994eccccdd398e9434b95) - **popup**: re-posiiton on resize due to content change *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`0675b91`](https://github.com/JakeStanger/ironbar/commit/0675b917f2beeed3e6b626dad8fe34b8063d9c83) - **tray**: icons ignoring scaling *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`c62d475`](https://github.com/JakeStanger/ironbar/commit/c62d47555ec31baa1a7094491e2977a832f4cfcc) - **tray**: submenus not working *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`f263849`](https://github.com/JakeStanger/ironbar/commit/f2638497fac4f0e350d069857e6e7437cb756669) - **launcher**: not resolving icon for some apps *(commit by [@slowsage](https://github.com/slowsage))*
- [`cf44c46`](https://github.com/JakeStanger/ironbar/commit/cf44c461db7a3e5093c69c12fcef57cf9675c6e2) - **workspaces**: favourites not persisting for initally opened workspaces *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`180f874`](https://github.com/JakeStanger/ironbar/commit/180f8748bbe52affbbfe8c5ec045c753e63d554d) - **music** - Handle NoActivePlayer (playerctld) , NoReply, NoMethod, ServiceUnknown DBus errors in mpris. *(commit by [@slowsage](https://github.com/slowsage))*
- [`3ba8b4b`](https://github.com/JakeStanger/ironbar/commit/3ba8b4bd9611bd82b251fbaf51f4b313f36f1c89) - regressions introduced by [#505](https://github.com/JakeStanger/ironbar/pull/505) *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`f50a65e`](https://github.com/JakeStanger/ironbar/commit/f50a65eab5edfa3a96e4e3b7e54de754ead1eb21) - upower module should display correctly for vertical bars *(commit by [@ClaireNeveu](https://github.com/ClaireNeveu))*
- [`188abc3`](https://github.com/JakeStanger/ironbar/commit/188abc33e910a708061517b13e36125f9d7736d3) - **tray**: icon colour channels are being incorrectly rendered *(commit by [@rdnelson](https://github.com/rdnelson))*
- [`ea2b208`](https://github.com/JakeStanger/ironbar/commit/ea2b20816d459aafe79578f155454d50684f5fad) - **focused**: incorrectly clearing when unfocused window title changes *(PR [#556](https://github.com/JakeStanger/ironbar/pull/556) by [@JakeStanger](https://github.com/JakeStanger))*
  - :arrow_lower_right: *fixes issue [#488](https://github.com/JakeStanger/ironbar/issues/488) opened by [@bluebyt](https://github.com/bluebyt)*
  - :arrow_lower_right: *fixes issue [#554](https://github.com/JakeStanger/ironbar/issues/554) opened by [@DanteDragan](https://github.com/DanteDragan)*

### :recycle: Refactors
- [`a55ba8c`](https://github.com/JakeStanger/ironbar/commit/a55ba8c523ff19fa607a31bac589a55b48db39ad) - rename `get_orientation` method to `orientation` *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`86c5b69`](https://github.com/JakeStanger/ironbar/commit/86c5b69f18356201db5c3a314f36e0f68e74c357) - **tray**: tidy imports *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`00a6a84`](https://github.com/JakeStanger/ironbar/commit/00a6a84ca6af6f3c64183ec08fdff7430770d39b) - **upower**: cheaper string building *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`b912619`](https://github.com/JakeStanger/ironbar/commit/b912619d61a74921c854ea6464e0922e5c107a27) - **image**: add debug logging *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`c7b6ee8`](https://github.com/JakeStanger/ironbar/commit/c7b6ee8bc00e92d075b8c66105c29e3df0906145) - add dead_code allow to fix build warning *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`004ea76`](https://github.com/JakeStanger/ironbar/commit/004ea76da5af3e8750e5a02a8780f62337b06844) - **tray**: complete client rewrite *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`706e040`](https://github.com/JakeStanger/ironbar/commit/706e040e25b54c128b0364a8e6982f2372da0b99) - split bar/top-level config structs *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`1b35354`](https://github.com/JakeStanger/ironbar/commit/1b353542722ac70b99e5a4f846e68ae68a2870fd) - fix clippy warnings *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`9245188`](https://github.com/JakeStanger/ironbar/commit/9245188af7830b09aa564ab83f1db2e2a4cffb2c) - better error handling for client initialization *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`314bfe7`](https://github.com/JakeStanger/ironbar/commit/314bfe7abecec66692d138b49186865c9132c6ef) - **nix**: simplify flake *(commit by [@JakeStanger](https://github.com/JakeStanger))*

### :memo: Documentation Changes
- [`76a6816`](https://github.com/JakeStanger/ironbar/commit/76a68165f09a6d07f8e95008cb9fe3d40d99abe0) - correct formatting tokens in upower *(commit by [@Disr0](https://github.com/Disr0))*
- [`e26e213`](https://github.com/JakeStanger/ironbar/commit/e26e213c4e409018f3b5c35b0319f5db8c0fa3bb) - improve info about logging *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`163a70e`](https://github.com/JakeStanger/ironbar/commit/163a70e690e2a9950c23ef8164dafd762a6a1020) - **readme**: update nix caching info *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`6a03c46`](https://github.com/JakeStanger/ironbar/commit/6a03c46146b612e53fa866ad98263d7cc29aacc8) - **readme**: add [mixxc](https://github.com/Elvyria/Mixxc) acknowledgement *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`3a3d3d7`](https://github.com/JakeStanger/ironbar/commit/3a3d3d75cd4fd8d474edc4c6ddb2c47bce60df16) - **readme**: add void package *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`fc42f6c`](https://github.com/JakeStanger/ironbar/commit/fc42f6c540131576d6eaf1201e78ba216861947d) - **readme**: add repology badge *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`8e9db14`](https://github.com/JakeStanger/ironbar/commit/8e9db141f8a668063ece3622ec91c3e22c3a87a3) - **macros**: add missing comment *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`bb02a21`](https://github.com/JakeStanger/ironbar/commit/bb02a21d0efcad07ba0598a38ff56abfc7c06107) - **compiling**: add missing notifications feature flag *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`ee8873a`](https://github.com/JakeStanger/ironbar/commit/ee8873a94a904d895a2005fa3593628c9500fc0c) - **custom**: add native examples *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`dffb3e5`](https://github.com/JakeStanger/ironbar/commit/dffb3e5d543d33c10146d43384b8a3c03ef3aab7) - **workspaces**: fix typo that results in a non working config *(commit by [@nyadiia](https://github.com/nyadiia))*
- [`782b955`](https://github.com/JakeStanger/ironbar/commit/782b9554a2a24123acfebcc80401abf051c7dc06) - fix issues with several more toml examples *(commit by [@JakeStanger](https://github.com/JakeStanger))*

### Note for maintainers

The addition of new modules requires the following new build dependencies:

- `libpulse`
- `luajit`

It also requires `lua-lgi` as a runtime dependency.

## [v0.14.1] - 2024-02-10
### :bug: Bug Fixes
- [`1c9c9bb`](https://github.com/JakeStanger/ironbar/commit/1c9c9bbece878286939abacfaec0daaecc559243) - **cli**: error when launched via `swaybar_command` *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`976dd6c`](https://github.com/JakeStanger/ironbar/commit/976dd6c55a5881b2b3c60c6d7e13b0e7d4301599) - **style**: file watcher not working for relative paths *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`4d9d78f`](https://github.com/JakeStanger/ironbar/commit/4d9d78f4caa998b4817de2d77c0f7362de318c52) - **dynamic string**: ironvar parser being too greedy *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`63304a9`](https://github.com/JakeStanger/ironbar/commit/63304a9ddd76b2274b8336eba7e1e5ef7c5d66e6) - **dynamic string**: always sending partial string on initialization *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`8016ec2`](https://github.com/JakeStanger/ironbar/commit/8016ec256de0c3d2290d1446cda45a769a3c5284) - **tray**: crash caused by excess updates *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`b3a70ce`](https://github.com/JakeStanger/ironbar/commit/b3a70ce8fa76b0ae8b06f423e7d5955c6d5d6920) - **tray**: not handling checkbox items *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`0616633`](https://github.com/JakeStanger/ironbar/commit/061663392e01503448fb44a064d172dbf10dc770) - do not panic on full channels *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`133632d`](https://github.com/JakeStanger/ironbar/commit/133632d1ad0778bb93e398e6d2bacf28c364f6c4) - **tray**: vastly improve rendering performance *(commit by [@JakeStanger](https://github.com/JakeStanger))*

### :recycle: Refactors
- [`996ad7e`](https://github.com/JakeStanger/ironbar/commit/996ad7e27f3a397f4650a6a746155cd22d6ccdb7) - **desktop file**: simplify some none-type handling *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`aaef3bf`](https://github.com/JakeStanger/ironbar/commit/aaef3bf96cebb3540b3b020891f88d3c5515034b) - fix new strict clippy warnings *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`f3687c5`](https://github.com/JakeStanger/ironbar/commit/f3687c5f9e69452bbc9c1fa87089f3a8afd9bfc0) - replace deprecated indexmap method *(commit by [@JakeStanger](https://github.com/JakeStanger))*

### :white_check_mark: Tests
- [`af7e037`](https://github.com/JakeStanger/ironbar/commit/af7e037dd5a24cff0959e2fd5f04e3eb49418b23) - **dynamic string**: test pango attributes with ironvars *(commit by [@JakeStanger](https://github.com/JakeStanger))*

### :memo: Documentation Changes
- [`754e339`](https://github.com/JakeStanger/ironbar/commit/754e33952eaf7794d00c831c46aab007684ff0b2) - add info on speeding up compilation time *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`cb2f9b0`](https://github.com/JakeStanger/ironbar/commit/cb2f9b0aaff1519516664ab04a3a195d29983b4e) - **examples**: fix issues with example css *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`1b54276`](https://github.com/JakeStanger/ironbar/commit/1b54276bea6268131fca7c3f453284ca0aee4b9b) - **compilation**: add sccache section *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`9d7cb08`](https://github.com/JakeStanger/ironbar/commit/9d7cb08f41e7290959e17ccd725aeb6ccaeef1a7) - **ironvars**: correct allowed chars in keys *(commit by [@JakeStanger](https://github.com/JakeStanger))*


## [v0.14.0] - 2024-01-20
### :sparkles: New Features
- [`25c490b`](https://github.com/JakeStanger/ironbar/commit/25c490b8b426176c1a4c9d402aafd6783c9b6d48) - **workspaces**: visible CSS selector *(commit by [@malicean](https://github.com/malicean))*
- [`60bb69f`](https://github.com/JakeStanger/ironbar/commit/60bb69feecd9444586cce29b32c4845d9888ad4e) - add `widget` and `widget-container` css classes on all widgets *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`ee04cd0`](https://github.com/JakeStanger/ironbar/commit/ee04cd025aa2a4c35aa8b4947a02a9cf66f87734) - bar auto-hide options *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`659c93d`](https://github.com/JakeStanger/ironbar/commit/659c93dd2aa36d12b720fa5bca84c0d4ec8f4eaf) - use top-level config as fallback when using monitor-based config *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`8371a92`](https://github.com/JakeStanger/ironbar/commit/8371a92204185a78a0ea597462a8dd5112774554) - load bars on monitor when it connects *(commit by [@JakeStanger](https://github.com/JakeStanger))*

### :bug: Bug Fixes
- [`4099847`](https://github.com/JakeStanger/ironbar/commit/40998475e2500c3ba04a0e5dad59a4fa5f891961) - **styles**: hot reload not working when edited with vim *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`f24b21d`](https://github.com/JakeStanger/ironbar/commit/f24b21d24226da12361c63e080382fbedfb4d114) - **focused**: clear when no window is focused *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`34ed6a9`](https://github.com/JakeStanger/ironbar/commit/34ed6a9e11861af3cfb647a06e1fde63c1d0d569) - **focused**: not clearing when switching to empty workspace *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`518c2ef`](https://github.com/JakeStanger/ironbar/commit/518c2ef023f97311f0d307448bc98700470dd41f) - Home Manager systemdIntegration warnings *(commit by [@delliottxyz](https://github.com/delliottxyz))*
- [`5f82b6e`](https://github.com/JakeStanger/ironbar/commit/5f82b6e9e0966cf22c2420fb1f584b5f94746afb) - **tray**: existing icons rendering as text *(commit by [@chmanie](https://github.com/chmanie))*
- [`80de5dd`](https://github.com/JakeStanger/ironbar/commit/80de5dd824011b0eabf573cf4546c8e59b251bf7) - some modules crashing due to recent gtk refactor *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`c356b22`](https://github.com/JakeStanger/ironbar/commit/c356b22401ad412c9a6a7f0092f2f2214e13f5f0) - **workspaces**: favourites missing `inactive` class on startup *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`b4d7545`](https://github.com/JakeStanger/ironbar/commit/b4d75450acacc36a71e0ed8365f82bd88d2a55e8) - **regression**: GTK refactor causing updates to be missed *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`b004d50`](https://github.com/JakeStanger/ironbar/commit/b004d5007cca051bab4e9b8eb8b3471deacc9512) - **launcher**: favourites not focused when opened *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`963f8ed`](https://github.com/JakeStanger/ironbar/commit/963f8edc4590af5a182a2b3eb2e5088de638715c) - **script**: spawning outside of tokio runtime causing crash *(commit by [@JakeStanger](https://github.com/JakeStanger))*

### :recycle: Refactors
- [`fea1f18`](https://github.com/JakeStanger/ironbar/commit/fea1f1852484c0ac2686152be26817d57e19146e) - fix new clippy warnings, fmt *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`4e67b73`](https://github.com/JakeStanger/ironbar/commit/4e67b73a83be038914407210a19ce5a38da23e99) - **wlr data control**: update to new nix epoll bindings *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`5582dcf`](https://github.com/JakeStanger/ironbar/commit/5582dcf373dfceabd02e3dcab0bdfcccf7563c44) - fix new clippy warning *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`08e354e`](https://github.com/JakeStanger/ironbar/commit/08e354e019d9e14a6df6ae8d29bb883b21bdb882) - fix new clippy warnings *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`b2fa19a`](https://github.com/JakeStanger/ironbar/commit/b2fa19ab6ce93e8865e9450ec58bf891e7380dd8) - begin restructuring core code to better encapsulate *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`ed5a162`](https://github.com/JakeStanger/ironbar/commit/ed5a16237d23decbbc12e310bbcfbc6975647006) - update wayland crates to latest versions *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`bea442e`](https://github.com/JakeStanger/ironbar/commit/bea442ed960f513288cf857e8ee9a5c61f742dfa) - update gtk/glib, remove glib channels *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`e847a84`](https://github.com/JakeStanger/ironbar/commit/e847a84c21763164d7f90b7c85e48c386b41002c) - fix casting based clippy warnings *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`967801d`](https://github.com/JakeStanger/ironbar/commit/967801dc322c8edbc5335e4e23d70a2442b5280c) - **workspaces**: avoid sending unknown update info *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`b2a37a3`](https://github.com/JakeStanger/ironbar/commit/b2a37a32b07d46fe56cb7c6b81b9e7da3fe27a15) - fix clippy warning *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`57b57ed`](https://github.com/JakeStanger/ironbar/commit/57b57ed002c394eae6caa87836aa8769345781bc) - pass context into modules controllers *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`c702f6f`](https://github.com/JakeStanger/ironbar/commit/c702f6fffa538037c47399dc2b5f7377252fff1b) - major client code changes *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`6f531a5`](https://github.com/JakeStanger/ironbar/commit/6f531a5654b1554d1bbba5b56ce623d2bb4f4d83) - remove `lazy_static` and `async_once` *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`e0dc5e1`](https://github.com/JakeStanger/ironbar/commit/e0dc5e104a773024fc8124672c954a984b0a9f1e) - **wayland**: remove unused request type *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`18b4784`](https://github.com/JakeStanger/ironbar/commit/18b47844f94067bbf029cf4b6b94153a742d6af1) - **wayland**: simplify task spawning code *(commit by [@JakeStanger](https://github.com/JakeStanger))*

### :memo: Documentation Changes
- [`b9c41af`](https://github.com/JakeStanger/ironbar/commit/b9c41af0f73c85c0daf6f0af2fd1339c79e66758) - **workspaces**: add missing `.inactive` selector *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`abd1f80`](https://github.com/JakeStanger/ironbar/commit/abd1f8054821cedef594ebcb22a914feb230c9f1) - **examples**: update discord icon, temporarily disable random label *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`ccc6ff2`](https://github.com/JakeStanger/ironbar/commit/ccc6ff2d943ba46d0f9a36478933cda8b14b7ab1) - **readme**: add nixpkgs details *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`c2306d6`](https://github.com/JakeStanger/ironbar/commit/c2306d668007d5f1e69b8b652443c04d2b9190fa) - **styling**: add another example for selecting gtk widgets *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`a768164`](https://github.com/JakeStanger/ironbar/commit/a7681645158ff4b6eae38dede2a4ec77344314c9) - **styling guide**: add explanation on specificity *(commit by [@Schweber](https://github.com/Schweber))*
- [`917c1bd`](https://github.com/JakeStanger/ironbar/commit/917c1bd52efef2dace262fc7affe7501c654a557) - **dynamic values**: link to scripts/ironvars pages *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`74930df`](https://github.com/JakeStanger/ironbar/commit/74930df83bbc7ba59e11912f301a77ae0f364b52) - **compiling**: fix fedora instructions *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`f2c4ddb`](https://github.com/JakeStanger/ironbar/commit/f2c4ddb91456ed6053edd66f18ffda708feb4489) - **sys info**: fix cpu temp examples *(commit by [@cyhyraethz](https://github.com/cyhyraethz))*
- [`a159825`](https://github.com/JakeStanger/ironbar/commit/a1598259eb11ebe0a9cc26d0230d36ecc257a7f4) - fix nerdfont icons *(commit by [@eclairevoyant](https://github.com/eclairevoyant))*


## [v0.13.0] - 2023-08-16
### :sparkles: New Features
- [`c3e9654`](https://github.com/JakeStanger/ironbar/commit/c3e9654cd3dfcea4276f5b114112b7541dd847fd) - **upower**: icon size option *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`f5bdc5a`](https://github.com/JakeStanger/ironbar/commit/f5bdc5a0272fefca4c91336699ea63913cdf3c08) - ipc server and cli *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`ded50cc`](https://github.com/JakeStanger/ironbar/commit/ded50cca6f01f08a8e44257394fdde634d421e8e) - support for 'ironvar' dynamic variables *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`c6319b7`](https://github.com/JakeStanger/ironbar/commit/c6319b78fd3992ad43327e90b6326ab653238f2e) - **ipc**: support for injecting additional stylesheets *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`27f920d`](https://github.com/JakeStanger/ironbar/commit/27f920d01217bedba279003291ad48c1aaa56bb0) - **launcher**: slightly improve focus logic when clicking item with multiple windows *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`bd90167`](https://github.com/JakeStanger/ironbar/commit/bd90167f4ea90cb97984b9f3b5e6f65b375d0c4a) - **clock**: format option for popup header *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`12053f1`](https://github.com/JakeStanger/ironbar/commit/12053f111a6f05a59e33396b9042821b98b9bc5c) - **music**: progress/seek bar in popup *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`7d3bb02`](https://github.com/JakeStanger/ironbar/commit/7d3bb02b4612f278bcc8a268a48c61b239c63e82) - **ipc**: reload config command *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`b310ea7`](https://github.com/JakeStanger/ironbar/commit/b310ea76362bcdf10e187d6b00cd2b8ed2870c41) - **clock**: localization support *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`738b9e3`](https://github.com/JakeStanger/ironbar/commit/738b9e3da714c9b998030e9f60b9b6f50c62ec76) - **config**: use default fallback with config instructions *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`2ccb263`](https://github.com/JakeStanger/ironbar/commit/2ccb2633c6c4d7f6eb264a6c49951853b728c9f3) - IPC for get_visible, set_visible, new bar `name` config option *(commit by [@A-Cloud-Ninja](https://github.com/A-Cloud-Ninja))*
- [`b7ee794`](https://github.com/JakeStanger/ironbar/commit/b7ee794bfc86730e7921c8a930cf8d8bb44474ad) - **ipc**: commands for opening/closing popups *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`ef443e6`](https://github.com/JakeStanger/ironbar/commit/ef443e6978949479388129760dabc3f8930c0b0f) - **image resolver**: add fallback image *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`9f65cf2`](https://github.com/JakeStanger/ironbar/commit/9f65cf293d9527a2c536847f0005957421a9be33) - **workspaces**: add `favorites` and `hidden` options *(commit by [@yavko](https://github.com/yavko))*
- [`19c684e`](https://github.com/JakeStanger/ironbar/commit/19c684e49facb57e3e2acf9cafecf177c2e1bfbf) - **nix**: automatic development environment with direnv *(commit by [@yavko](https://github.com/yavko))*

### :bug: Bug Fixes
- [`6db7742`](https://github.com/JakeStanger/ironbar/commit/6db7742e068dc03d67dbf35e0d9db27f900392fe) - crash on startup introduced by recent refactors *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`f78c7f9`](https://github.com/JakeStanger/ironbar/commit/f78c7f9b981c98676e855ff6a63e33a51927c709) - not resolving flatpak application icons *(commit by [@body20002](https://github.com/body20002))*
- [`1759945`](https://github.com/JakeStanger/ironbar/commit/1759945912e376581e5fcd5ed2916f89e2090f2b) - **music**: correctly show/hide popup elements based on player capabilities *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`a9ac29d`](https://github.com/JakeStanger/ironbar/commit/a9ac29d8857256d13e14104db235117e3c752972) - clipboard partially behind wrong feature flag *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`c711dd8`](https://github.com/JakeStanger/ironbar/commit/c711dd858554140bcfb6df515a43b40ee2baee67) - failing to resolve icons with home_manager *(commit by [@christoph00](https://github.com/christoph00))*
- [`1a272e0`](https://github.com/JakeStanger/ironbar/commit/1a272e00fbeca4b5e39b527ffed439bc51fd4080) - **label**: not using markup *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`4ca17d1`](https://github.com/JakeStanger/ironbar/commit/4ca17d1337acfbbc21c04058d97f689a1cce60a6) - **launcher**: incorrectly resolving some applications *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`eee2182`](https://github.com/JakeStanger/ironbar/commit/eee2182ab90fdc22cd05da9417cbee17e4c67088) - **ipc**: command/response casing *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`c582bc3`](https://github.com/JakeStanger/ironbar/commit/c582bc33905702a9ebe323e6dfa9413485f48fb7) - **cli**: `set-visible` command causing panic *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`87dd764`](https://github.com/JakeStanger/ironbar/commit/87dd7646fc9223ac7e67842934f3bd104b4eea80) - **launcher**: not clearing focused state when closing window *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`6f57ad4`](https://github.com/JakeStanger/ironbar/commit/6f57ad47ac30348c0ae2b7dba35d5ffdbf96f72d) - **launcher**: not setting focus state when opening favourite *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`2367faa`](https://github.com/JakeStanger/ironbar/commit/2367faab0440327620052de845c6a0d3032f2f05) - **image**: using fallback in places it shouldn't *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`7f6fef6`](https://github.com/JakeStanger/ironbar/commit/7f6fef6338d7a8c909f3224b32426dfc2aacc295) - **image**: matching desktop file names too eagerly *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`89ec06f`](https://github.com/JakeStanger/ironbar/commit/89ec06fc7b252052f96e45f5d0f6d6504878a13a) - **music**: hide album art widget when no image *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`2902331`](https://github.com/JakeStanger/ironbar/commit/2902331af00f2e52fdea06964fbd89d72fe2ebbb) - **dynamic string**: incorrectly handling strings containing multipoint utf-8 chars *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`901a86c`](https://github.com/JakeStanger/ironbar/commit/901a86caa491648268f0618e17a25b978552db0c) - **custom**: crash when clicking non-popup button *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`54f0f23`](https://github.com/JakeStanger/ironbar/commit/54f0f232f208602699e5021942c3d0f3947ca6de) - **launcher**: popup not closing when hover leaves widget *(commit by [@JakeStanger](https://github.com/JakeStanger))*

### :recycle: Refactors
- [`d121dc3`](https://github.com/JakeStanger/ironbar/commit/d121dc3d1e9468a67deb528c35fc3897c3840f77) - fix unused var warning *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`cc181a8`](https://github.com/JakeStanger/ironbar/commit/cc181a8b6d0ac1cccd4ed2f9f420c138ed5383d2) - fix new clippy warnings *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`7016f7f`](https://github.com/JakeStanger/ironbar/commit/7016f7f79e7e29a3318b535ba224aa78bd91688a) - use new smart pointer macros throughout codebase *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`06251e2`](https://github.com/JakeStanger/ironbar/commit/06251e293e8f56e1897fed80335f114fdea57183) - fix new pedantic clippy warnings *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`36f3db7`](https://github.com/JakeStanger/ironbar/commit/36f3db741178b959070ee90bcd6448e1b2a6ef26) - **image**: do not try to read desktop files where definitely not necessary *(commit by [@JakeStanger](https://github.com/JakeStanger))*

### :memo: Documentation Changes
- [`607c728`](https://github.com/JakeStanger/ironbar/commit/607c7285d7e01265a8c8417e2941b2099e61aa42) - update for ipc/cli, tidy a bit *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`4b88079`](https://github.com/JakeStanger/ironbar/commit/4b88079561e5c9fec63480afe56a1f89c76dc094) - fix header *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`4620f29`](https://github.com/JakeStanger/ironbar/commit/4620f29d381394aef8b241b03009ef8c3b8d0145) - **examples**: update stylesheet *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`3d94987`](https://github.com/JakeStanger/ironbar/commit/3d949874de90b0e5c06cb62726629133d0ea76e3) - **ipc**: add link to luajit library *(commit by [@JakeStanger](https://github.com/JakeStanger))*


## [v0.12.1] - 2023-06-18
### :boom: BREAKING CHANGES
- due to [`e11177f`](https://github.com/JakeStanger/ironbar/commit/e11177fea3095560057278d71cebca01bed295d6) - add sensible class names for icon labels *(commit by [@JakeStanger](https://github.com/JakeStanger))*:

  Where both textual and image icons are supported, CSS classes have changed to better reflect their targets. `.icon` has changed to `.icon-box` and `.icon` now targets the underlying element. `.label` has been changed to `.icon.text-icon`. This affects icons on the **music**, **workspaces**, and **clipboard** modules.


### :bug: Bug Fixes
- [`31a57ae`](https://github.com/JakeStanger/ironbar/commit/31a57ae637fa5918f163c8b191916867395912f3) - scripts don't work while running ironbar under a systemd service *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`f82f897`](https://github.com/JakeStanger/ironbar/commit/f82f897982e87906e2c9156d4115013bc8e99763) - **upower**: popup always empty *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`9012fee`](https://github.com/JakeStanger/ironbar/commit/9012feee4f9b60b2c22a956de732847892331222) - **image**: still blurry on hidpi *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`0e65f93`](https://github.com/JakeStanger/ironbar/commit/0e65f93a230cb5ab010b43962fd2e829945c291b) - excess popup windows *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`87ca399`](https://github.com/JakeStanger/ironbar/commit/87ca399220e5d48eefe2f295d1dba1b9452c4472) - poor error handling for missing images *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`22b630a`](https://github.com/JakeStanger/ironbar/commit/22b630a10b9836531a8b03eb904e6f9fcf839fe6) - broken nerd font icons *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`48d6af0`](https://github.com/JakeStanger/ironbar/commit/48d6af0281f460d3ed3745a2ffb2b61848430ecb) - **music**: showing when no mpris player found *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`b9740cb`](https://github.com/JakeStanger/ironbar/commit/b9740cba8f2fa9dfa18a57345027283610f6487e) - upower icon too large *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`a6b6866`](https://github.com/JakeStanger/ironbar/commit/a6b686624b750863aa1c26ca4f1688dfa8c81a61) - **upower**: icon outside button *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`a5ecb36`](https://github.com/JakeStanger/ironbar/commit/a5ecb363fdb2eb3ab543ad56c55c186414500469) - popups occasionally getting jumbled with multiple bars *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`e11177f`](https://github.com/JakeStanger/ironbar/commit/e11177fea3095560057278d71cebca01bed295d6) - add sensible class names for icon labels *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`ac34c05`](https://github.com/JakeStanger/ironbar/commit/ac34c05d2ecb07fd871ed03ef6ee545dc2e6743d) - **focused**: empty icon rendered when `show_icon = false` *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`de3aa5d`](https://github.com/JakeStanger/ironbar/commit/de3aa5d7b10e0bf6d5ff3a39b009ff53a3316a5e) - **focused**: previous icon does not clear if new icon fails to load *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`de98cf3`](https://github.com/JakeStanger/ironbar/commit/de98cf3daee816a0ff72d1f6ba6bc0e15ec53fca) - **tray**: (maybe?) sometimes bus name is taken *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`103a224`](https://github.com/JakeStanger/ironbar/commit/103a224355e8f700904a2b8fbc87cd7be4f64566) - **launcher**: crash when focusing newly opened window in popup *(commit by [@JakeStanger](https://github.com/JakeStanger))*

### :memo: Documentation Changes
- [`327e345`](https://github.com/JakeStanger/ironbar/commit/327e345630a5a89a6f7e464d873c16666d929c0f) - **examples**: fix css button styles *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`13d3923`](https://github.com/JakeStanger/ironbar/commit/13d39235ad032623745baecb6911057ec057ff11) - **examples**: fix casing of steam in launcher favourites *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`cdeafbd`](https://github.com/JakeStanger/ironbar/commit/cdeafbdc7245d37120e3e8338b6f933a39d4e428) - **sys info**: add typical temperature sensors for intel/amd cpus *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`ff315ff`](https://github.com/JakeStanger/ironbar/commit/ff315ff5dbd545d8b72b6aa10087c940cb8a5eee) - **music**: fix incorrect type for `host`/`music_dir` options *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`bd144e8`](https://github.com/JakeStanger/ironbar/commit/bd144e87a8f6668c877d42697ebbedbe5a374c3d) - **readme**: make prettier *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`242b70e`](https://github.com/JakeStanger/ironbar/commit/242b70ed3988b85455b0dbbcb3243b31f89d2ee1) - **contributing**: enforce conventional commits *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`96d36c4`](https://github.com/JakeStanger/ironbar/commit/96d36c43d43ba2f9e9d9441ae01c0743cc56f627) - add missing icon/image selectors *(commit by [@JakeStanger](https://github.com/JakeStanger))*


## [v0.12.0] - 2023-05-06
### :boom: BREAKING CHANGES
- due to [`dea6641`](https://github.com/JakeStanger/ironbar/commit/dea66415c2e11e34ba44d016aaa6cfb4ef7b9f9b) - module-level `name` and `class` options *(commit by [@JakeStanger](https://github.com/JakeStanger))*:

  To allow for the `name` property, any widgets that were previously targeted by name should be targeted by class instead. This affects **all modules and all popups**, as well as several widgets inside modules. **This will break a lot of rules in your stylesheet**. To attempt to mitigate the damage, a migration script can be found [here](https://raw.githubusercontent.com/JakeStanger/ironbar/master/scripts/migrate-styles.sh) that should get you most of the way.


### :sparkles: New Features
- [`6c62286`](https://github.com/JakeStanger/ironbar/commit/6c622864b388548eaaa595f41993606cc151d585) - new label module *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`cac064f`](https://github.com/JakeStanger/ironbar/commit/cac064f4795e9f418cc0820f04944f91121c426a) - ability to configure popup gap *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`dfe1964`](https://github.com/JakeStanger/ironbar/commit/dfe1964abf9ca54beb38cad0bcf02bd9fb0b5c4d) - **custom**: slider widget *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`72b14b6`](https://github.com/JakeStanger/ironbar/commit/72b14b6c4ed3dccfe7b4b23b220ab0a87ec79aa2) - **custom**: progress bar widget. *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`a9d1233`](https://github.com/JakeStanger/ironbar/commit/a9d12339097cbe0fef1628460ef538319a048223) - **custom**: support dynamic strings on buttons *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`3d308ab`](https://github.com/JakeStanger/ironbar/commit/3d308ab572a39ada2501ddc1b822e50e1f8a8363) - **custom**: support dynamic string in image source *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`4a09b70`](https://github.com/JakeStanger/ironbar/commit/4a09b70854dad33bf890a3fe766f854d9195e786) - **custom**: support common options in widgets *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`83f44fd`](https://github.com/JakeStanger/ironbar/commit/83f44fd92fe74b45fcdfc242fb90fc932dd2b00b) - wrap modules in a revealer to support animated show/hide *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`1fa0c0e`](https://github.com/JakeStanger/ironbar/commit/1fa0c0e9774c302727d414f5aef999ab71a7acb8) - **custom**: support mouse wheel on slider *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`2da28b9`](https://github.com/JakeStanger/ironbar/commit/2da28b9bf5790adfc46c58b6f6d5fdd13cc17195) - ability to configure image icon sizes *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`033d0f7`](https://github.com/JakeStanger/ironbar/commit/033d0f7e6e450b3f2d62d9a75210d52611cf346d) - **custom**: option to toggle slider label *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`76e2b7b`](https://github.com/JakeStanger/ironbar/commit/76e2b7ba3e788f273039d74635881ddb96264258) - **music**: option to hide status icon on widget *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`ad3c171`](https://github.com/JakeStanger/ironbar/commit/ad3c171ecacaebf10408c2583ed7361ed029075e) - implement upower module *(commit by [@p00f](https://github.com/p00f))*
- [`2a155b9`](https://github.com/JakeStanger/ironbar/commit/2a155b9aa8a3634908512d9b83680925962d478f) - **music**: add css selector for button contents *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`c1ea5fa`](https://github.com/JakeStanger/ironbar/commit/c1ea5fad7ec308895f0454b6de05a3177563626c) - **logging**: include line numbers *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`dea6641`](https://github.com/JakeStanger/ironbar/commit/dea66415c2e11e34ba44d016aaa6cfb4ef7b9f9b) - module-level `name` and `class` options *(commit by [@JakeStanger](https://github.com/JakeStanger))*

### :bug: Bug Fixes
- [`9109453`](https://github.com/JakeStanger/ironbar/commit/910945306c3261190a16300da2ed28efb945a6ac) - **dynamic string**: parser issue related to incorrectly matching braces *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`7355db7`](https://github.com/JakeStanger/ironbar/commit/7355db74ec9118c2cb46899534a3adac8d7165d9) - **image**: http provider not handling non-success codes *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`a87d8d5`](https://github.com/JakeStanger/ironbar/commit/a87d8d5c3071a1d8ab149deae17d261ae97368ea) - **tray**: icons sometimes not showing *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`15a9d8d`](https://github.com/JakeStanger/ironbar/commit/15a9d8d42c9319a7062e6a90086e0c1c3323f5d8) - **script**: parser incorrectly handling colons *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`68bc823`](https://github.com/JakeStanger/ironbar/commit/68bc8230ddf3352cc0de9f8cc770632744c22747) - **tray**: icons sometimes not showing *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`b038e76`](https://github.com/JakeStanger/ironbar/commit/b038e7671af4bfa41060adf724deb8c6151fac1f) - **tray**: icons sometimes not showing *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`7926bb0`](https://github.com/JakeStanger/ironbar/commit/7926bb07eb181edaf6da2f11a7dc00f8be2240eb) - **nix**: Fix `nix run` support *(commit by [@yavko](https://github.com/yavko))*
- [`2c88c99`](https://github.com/JakeStanger/ironbar/commit/2c88c99cb605d312e2d76d620f502c7e7cd8866e) - **dynamic string**: crash when last segment is static and a single char *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`338f5a0`](https://github.com/JakeStanger/ironbar/commit/338f5a0e1b58dc9b52caee61d6a9748cf13153c5) - **nix**: Attempt to fix image blurriness *(commit by [@yavko](https://github.com/yavko))*
- [`db0868a`](https://github.com/JakeStanger/ironbar/commit/db0868a3fc0734daa61067e377018c692599ebff) - **image**: not scaling icons for hidpi *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`14b6c1a`](https://github.com/JakeStanger/ironbar/commit/14b6c1a69f28836ed9e3b74eeb97a42ea60ffc27) - bars duplicate when starting second instance *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`98aaaa0`](https://github.com/JakeStanger/ironbar/commit/98aaaa0d1407681b3d790c933c4972b8122f8007) - fallback to default icon theme for notifier items *(commit by [@oknozor](https://github.com/oknozor))*
- [`735f5cc`](https://github.com/JakeStanger/ironbar/commit/735f5cc9f1518c256785d42f3d21ed5c68b11711) - **launcher**: crash when focusing window *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`e1abadc`](https://github.com/JakeStanger/ironbar/commit/e1abadcf39a2d39078e75179a167e9277ee5e550) - **clipboard**: copying large images filling write pipe *(commit by [@JakeStanger](https://github.com/JakeStanger))*

### :recycle: Refactors
- [`2ab06f0`](https://github.com/JakeStanger/ironbar/commit/2ab06f044ec300628d6648852d395889b6752b76) - **custom**: split into enum with separate file per widget *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`3613aef`](https://github.com/JakeStanger/ironbar/commit/3613aef5c5a4051b5a44e33342c0eaaab3d4a690) - **custom**: reduce a lot of repeated code *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`c214f65`](https://github.com/JakeStanger/ironbar/commit/c214f65ecb86a0da6559025203701661924f65bb) - fix strict clippy warnings *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`27d11de`](https://github.com/JakeStanger/ironbar/commit/27d11de6616c410422d7abd579d09b3abc02f43a) - **config**: split common code into separate file *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`6fd69d6`](https://github.com/JakeStanger/ironbar/commit/6fd69d657c6224bc47c9b3cb5affcf74b63a6aa6) - move module creation code to module module *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`e63509a`](https://github.com/JakeStanger/ironbar/commit/e63509a3a7673ea41b4c937089a1cf6d2362fed3) - fix a few new clippy warnings *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`7f46cb4`](https://github.com/JakeStanger/ironbar/commit/7f46cb49767bd722be8d42999a9ba69887efcd40) - **wayland**: update to 0.30.0 *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`38da59c`](https://github.com/JakeStanger/ironbar/commit/38da59cd419fa0023d0ea0b435b11f0f9dea3f15) - fix a few pedantic clippy warnings *(commit by [@JakeStanger](https://github.com/JakeStanger))*

### :memo: Documentation Changes
- [`e928b30`](https://github.com/JakeStanger/ironbar/commit/e928b30f9927aa7c895c0d9855ee3ef09e559dc7) - **custom**: rewrite widget options to be clearer *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`138b5b3`](https://github.com/JakeStanger/ironbar/commit/138b5b39038a005d17069830a04b88d52730bed5) - **custom**: fix potential error in progress example *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`07df51c`](https://github.com/JakeStanger/ironbar/commit/07df51c2497977a31b2f5ef5bc7d051e0bd88564) - include readme in rust docs *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`dd7c9f3`](https://github.com/JakeStanger/ironbar/commit/dd7c9f30db6e4e1ede4d57255122b359636b8f58) - add transition module-level options *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`610c352`](https://github.com/JakeStanger/ironbar/commit/610c3528af98b8c6b02af7ce5c07190776522c3a) - add missing link to upower page *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`ea9f7ca`](https://github.com/JakeStanger/ironbar/commit/ea9f7caaf7a35eebd603ce2854672d5af2901018) - add missing `upower` feature flag *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`618b7ef`](https://github.com/JakeStanger/ironbar/commit/618b7ef5520de6f3796b66e42422a36c5a191ab0) - improve example css *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`139bc5d`](https://github.com/JakeStanger/ironbar/commit/139bc5d23f7f887b7b65d50adc21fa6679ea291e) - **compiling**: improve requirements list *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`cf32870`](https://github.com/JakeStanger/ironbar/commit/cf32870f8a380c305a436593950c3da524a2296f) - **compiling**: add ron feature flag *(commit by [@JakeStanger](https://github.com/JakeStanger))*


## [v0.11.0] - 2023-04-01
### :boom: BREAKING CHANGES
- due to [`ca4fe42`](https://github.com/JakeStanger/ironbar/commit/ca4fe422f22866748f2cb6239b31170a974d254b) - ability to set fixed length *(commit by [@JakeStanger](https://github.com/JakeStanger))*:

  This changes the behaviour of `truncate.length`. A new property, `truncate.max_length`, has been introduced that uses the old behaviour.


### :sparkles: New Features
- [`d253c4b`](https://github.com/JakeStanger/ironbar/commit/d253c4bd7f306c7b8fef223d1beb7b1f6e77629b) - add configurable margins around bar *(commit by [@ttoino](https://github.com/ttoino))*
- [`ca4fe42`](https://github.com/JakeStanger/ironbar/commit/ca4fe422f22866748f2cb6239b31170a974d254b) - **truncate**: ability to set fixed length *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`575d6cc`](https://github.com/JakeStanger/ironbar/commit/575d6cc30f9e28079aed8425566048abd3d9e022) - new clipboard manager module *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`9984b63`](https://github.com/JakeStanger/ironbar/commit/9984b638b55adea11ba90412346fbb8220f05682) - **nix**: initial nix feature flags impl *(commit by [@yavko](https://github.com/yavko))*
- [`b1475a1`](https://github.com/JakeStanger/ironbar/commit/b1475a1affd2f101f1f707ab1a0e8e5509a1d99f) - **nix**: use cargo default features *(commit by [@yavko](https://github.com/yavko))*
- [`102d247`](https://github.com/JakeStanger/ironbar/commit/102d2478a9d0ecc8be12c5ea6019a5a5411cc6ab) - module hover options *(commit by [@JakeStanger](https://github.com/JakeStanger))*

### :bug: Bug Fixes
- [`2ac5071`](https://github.com/JakeStanger/ironbar/commit/2ac507144b42a80507f8d2df214889c114c069df) - not setting layer shell namespace *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`7dff3e6`](https://github.com/JakeStanger/ironbar/commit/7dff3e6f8b989132ff0c4406caa72f063dd57c9f) - **image**: widgets missing names *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`54b9b28`](https://github.com/JakeStanger/ironbar/commit/54b9b28c75b2fe300e2bad1436d315da1950953e) - make readme more concise *(commit by [@yavko](https://github.com/yavko))*
- [`8cbb73b`](https://github.com/JakeStanger/ironbar/commit/8cbb73b75e7aca1aa163406f4583273e6ff4bac2) - **dynamic string**: dynamic sections not respecting ordering *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`d0b7bdb`](https://github.com/JakeStanger/ironbar/commit/d0b7bdbafcc34967dd5b048ea12e6267ba293566) - **nix**: home manager module, and features *(commit by [@yavko](https://github.com/yavko))*

### :recycle: Refactors
- [`d84139a`](https://github.com/JakeStanger/ironbar/commit/d84139a914f9b35054dc6048715e1ed7e79d7441) - general tidy up *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`7212bbc`](https://github.com/JakeStanger/ironbar/commit/7212bbcf61e097b35a7ab341e19e9daefd2edf95) - **dynamic string**: use vec instead of indexmap *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`ecdd71a`](https://github.com/JakeStanger/ironbar/commit/ecdd71a43d267161f84e3c4a3c22e9454c0f7184) - **config**: use `universal-config` crate. *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`6221f74`](https://github.com/JakeStanger/ironbar/commit/6221f7454a2da2ec8a5a7f84e6fd35a8dc1a1548) - fix new clippy warnings *(commit by [@JakeStanger](https://github.com/JakeStanger))*

### :memo: Documentation Changes
- [`7c36f5c`](https://github.com/JakeStanger/ironbar/commit/7c36f5cb0cf03191c9b03e2455b63829a64e402e) - fix a couple of issues *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`83a4916`](https://github.com/JakeStanger/ironbar/commit/83a49165c42fa793ef1224f93cbc147bc69de894) - **compiling**: add info about build deps *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`5bbe64b`](https://github.com/JakeStanger/ironbar/commit/5bbe64bb86fb2db0921e284a1560db2f6c1a1920) - **clock**: format table *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`2b26eaf`](https://github.com/JakeStanger/ironbar/commit/2b26eaf41036609be4dfc57689ca8d770dcb6b9b) - **clipboard**: fix incorrect setting description *(commit by [@JakeStanger](https://github.com/JakeStanger))*
- [`0125ce5`](https://github.com/JakeStanger/ironbar/commit/0125ce5916c003d1ea9a141fe5a0f6a54b2778ab) - **examples**: update styles example *(commit by [@JakeStanger](https://github.com/JakeStanger))*


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
[v0.11.0]: https://github.com/JakeStanger/ironbar/compare/v0.10.0...v0.11.0
[v0.12.0]: https://github.com/JakeStanger/ironbar/compare/v0.11.0...v0.12.0
[v0.12.1]: https://github.com/JakeStanger/ironbar/compare/v0.12.0...v0.12.1
[v0.13.0]: https://github.com/JakeStanger/ironbar/compare/v0.12.1...v0.13.0
[v0.14.0]: https://github.com/JakeStanger/ironbar/compare/v0.13.0...v0.14.0
[v0.14.1]: https://github.com/JakeStanger/ironbar/compare/v0.14.0...v0.14.1
[v0.15.0]: https://github.com/JakeStanger/ironbar/compare/v0.14.3...v0.15.0
[v0.15.1]: https://github.com/JakeStanger/ironbar/compare/v0.15.0...v0.15.1
[v0.16.0]: https://github.com/JakeStanger/ironbar/compare/v0.15.1...v0.16.0
