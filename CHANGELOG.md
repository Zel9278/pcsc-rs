# Changelog

## [2.0.0](https://github.com/Zel9278/pcsc-rs/compare/v1.4.22...v2.0.0) (2026-10-08)


### ⚠ BREAKING CHANGES

* the client needs a server that accepts WebSocket (pc-status with routes/server.ts). socket.io-only servers no longer work.

### Features

* survive connection and GPU errors, check for updates while running ([#653](https://github.com/Zel9278/pcsc-rs/issues/653)) ([5896dc2](https://github.com/Zel9278/pcsc-rs/commit/5896dc28e7034243974520057ee9d19c68b03270))
* talk WebSocket to the server, send every NVIDIA GPU ([#656](https://github.com/Zel9278/pcsc-rs/issues/656)) ([51db87d](https://github.com/Zel9278/pcsc-rs/commit/51db87dcec4572e645702f81ebcef7496d9f3d55))


### Dependencies

* update rust crate cfg-if to v1.0.1 ([#651](https://github.com/Zel9278/pcsc-rs/issues/651)) ([f42ddb7](https://github.com/Zel9278/pcsc-rs/commit/f42ddb7a23865b87f1eb30b832dcadbdb31d73ca))

## [1.4.22](https://github.com/eoeo-org/pcsc-rs/compare/v1.4.21...v1.4.22) (2025-06-05)


### Dependencies

* update rust crate sysinfo to v0.35.2 ([#648](https://github.com/eoeo-org/pcsc-rs/issues/648)) ([946faff](https://github.com/eoeo-org/pcsc-rs/commit/946faffc59111dca86c69094f669a36d5e363d1e))

## [1.4.21](https://github.com/eoeo-org/pcsc-rs/compare/v1.4.20...v1.4.21) (2025-06-02)


### Dependencies

* lock file maintenance ([#645](https://github.com/eoeo-org/pcsc-rs/issues/645)) ([b51c500](https://github.com/eoeo-org/pcsc-rs/commit/b51c500c45097f9f34de77732842f059548f89a7))
* lock file maintenance ([#647](https://github.com/eoeo-org/pcsc-rs/issues/647)) ([c7f880e](https://github.com/eoeo-org/pcsc-rs/commit/c7f880ec370d4694757412108a7d2aa3b96f38c9))

## [1.4.20](https://github.com/eoeo-org/pcsc-rs/compare/v1.4.19...v1.4.20) (2025-05-29)


### Dependencies

* lock file maintenance ([#644](https://github.com/eoeo-org/pcsc-rs/issues/644)) ([6f11ec1](https://github.com/eoeo-org/pcsc-rs/commit/6f11ec1689b2d8849210c046c9d8e508baab42fd))
* update rust crate openssl to v0.10.73 ([#642](https://github.com/eoeo-org/pcsc-rs/issues/642)) ([b0a8279](https://github.com/eoeo-org/pcsc-rs/commit/b0a8279c5675cbeb63d57a82bd64c94447f70e6a))

## [1.4.19](https://github.com/eoeo-org/pcsc-rs/compare/v1.4.18...v1.4.19) (2025-05-21)


### Dependencies

* lock file maintenance ([#640](https://github.com/eoeo-org/pcsc-rs/issues/640)) ([f75e8a3](https://github.com/eoeo-org/pcsc-rs/commit/f75e8a30b52563a2aa729f66a95dad2ca04d2bc1))

## [1.4.18](https://github.com/eoeo-org/pcsc-rs/compare/v1.4.17...v1.4.18) (2025-05-12)


### Dependencies

* lock file maintenance ([#638](https://github.com/eoeo-org/pcsc-rs/issues/638)) ([01d6f46](https://github.com/eoeo-org/pcsc-rs/commit/01d6f46b6e02ec41e140f956d90d998dd17911ab))
* update rust crate sysinfo to v0.35.1 ([#636](https://github.com/eoeo-org/pcsc-rs/issues/636)) ([7aaf724](https://github.com/eoeo-org/pcsc-rs/commit/7aaf7244c3a8b284fcf4e2a754ef1a455fa6c45d))

## [1.4.17](https://github.com/eoeo-org/pcsc-rs/compare/v1.4.16...v1.4.17) (2025-05-08)


### Dependencies

* lock file maintenance ([#627](https://github.com/eoeo-org/pcsc-rs/issues/627)) ([ae99709](https://github.com/eoeo-org/pcsc-rs/commit/ae99709f3c12607ef5cfb1e1c65437ec3726a0e0))
* update rust crate sysinfo to v0.35.0 ([#629](https://github.com/eoeo-org/pcsc-rs/issues/629)) ([3539699](https://github.com/eoeo-org/pcsc-rs/commit/3539699de402a5c26d66a0e91e44a28c96dea4d9))


### Build System

* **deps:** update rust crate git2 to v0.20.2 ([#634](https://github.com/eoeo-org/pcsc-rs/issues/634)) ([d0efe2e](https://github.com/eoeo-org/pcsc-rs/commit/d0efe2e2df3048bba36abedcd943b55ddc1abeae))

## [1.4.16](https://github.com/eoeo-org/pcsc-rs/compare/v1.4.15...v1.4.16) (2025-04-27)


### Dependencies

* lock file maintenance ([#616](https://github.com/eoeo-org/pcsc-rs/issues/616)) ([aeec7a6](https://github.com/eoeo-org/pcsc-rs/commit/aeec7a6e9ced45923dffff2a07b5d3b47d82c9a7))
* lock file maintenance ([#620](https://github.com/eoeo-org/pcsc-rs/issues/620)) ([feb6c12](https://github.com/eoeo-org/pcsc-rs/commit/feb6c1253e5ce83f73686b99c09199b01bf4f85a))
* lock file maintenance ([#624](https://github.com/eoeo-org/pcsc-rs/issues/624)) ([89b5d42](https://github.com/eoeo-org/pcsc-rs/commit/89b5d4258de82e281e0a1c1e22b8ddcae7416738))
* lock file maintenance ([#625](https://github.com/eoeo-org/pcsc-rs/issues/625)) ([9c89a35](https://github.com/eoeo-org/pcsc-rs/commit/9c89a35890594746a601b5b5ebb5dc4a0b62f37a))
* update rust crate openssl to v0.10.72 ([#623](https://github.com/eoeo-org/pcsc-rs/issues/623)) ([edbb808](https://github.com/eoeo-org/pcsc-rs/commit/edbb808228933a0e6f501bb9a2ff10a5f3de1393))
* update rust crate sysinfo to v0.34.2 ([#619](https://github.com/eoeo-org/pcsc-rs/issues/619)) ([22d85d9](https://github.com/eoeo-org/pcsc-rs/commit/22d85d93e5144312976eb047287bb43ae629c55f))

## [1.4.15](https://github.com/eoeo-org/pcsc-rs/compare/v1.4.14...v1.4.15) (2025-03-18)


### Dependencies

* lock file maintenance ([#608](https://github.com/eoeo-org/pcsc-rs/issues/608)) ([9b25904](https://github.com/eoeo-org/pcsc-rs/commit/9b25904a5b8c9fb9c3f986c2c20d45bfbcca0b56))
* lock file maintenance ([#610](https://github.com/eoeo-org/pcsc-rs/issues/610)) ([41858d8](https://github.com/eoeo-org/pcsc-rs/commit/41858d898a49bf973959428ebb433dc85d9cf9e1))


### Build System

* **deps:** update rust crate git2 to v0.20.1 ([#611](https://github.com/eoeo-org/pcsc-rs/issues/611)) ([5285564](https://github.com/eoeo-org/pcsc-rs/commit/52855641c9a4e66618988a921ab26da835cd2dc5))

## [1.4.14](https://github.com/eoeo-org/pcsc-rs/compare/v1.4.13...v1.4.14) (2025-03-10)


### Dependencies

* lock file maintenance ([#602](https://github.com/eoeo-org/pcsc-rs/issues/602)) ([101ec0f](https://github.com/eoeo-org/pcsc-rs/commit/101ec0f92b5475eedb6db6ce75d000893fec36d3))
* update rust crate serde_json to v1.0.140 ([#603](https://github.com/eoeo-org/pcsc-rs/issues/603)) ([8240e9d](https://github.com/eoeo-org/pcsc-rs/commit/8240e9dec4d2d417e9982be53eb0054facd52465))
* update serde monorepo to v1.0.219 ([#607](https://github.com/eoeo-org/pcsc-rs/issues/607)) ([a158918](https://github.com/eoeo-org/pcsc-rs/commit/a158918ac184898645ff346cf40996d1bae39315))

## [1.4.13](https://github.com/eoeo-org/pcsc-rs/compare/v1.4.12...v1.4.13) (2025-03-01)


### Dependencies

* lock file maintenance ([#598](https://github.com/eoeo-org/pcsc-rs/issues/598)) ([31f2129](https://github.com/eoeo-org/pcsc-rs/commit/31f2129818d76acc46f30cc60ecf4f982f35576f))

## [1.4.12](https://github.com/eoeo-org/pcsc-rs/compare/v1.4.11...v1.4.12) (2025-02-20)


### Dependencies

* update rust crate serde_json to v1.0.139 ([#594](https://github.com/eoeo-org/pcsc-rs/issues/594)) ([a743b80](https://github.com/eoeo-org/pcsc-rs/commit/a743b80cf4de8053ce7729125c0f2c41d3740433))
* update serde monorepo to v1.0.218 ([#596](https://github.com/eoeo-org/pcsc-rs/issues/596)) ([59ca6a8](https://github.com/eoeo-org/pcsc-rs/commit/59ca6a85996b8cf4d100175df427b5221c9b613e))

## [1.4.11](https://github.com/eoeo-org/pcsc-rs/compare/v1.4.10...v1.4.11) (2025-02-19)


### Dependencies

* lock file maintenance ([#592](https://github.com/eoeo-org/pcsc-rs/issues/592)) ([3b1263d](https://github.com/eoeo-org/pcsc-rs/commit/3b1263d0960bde181788e3f1f8a7216374387be3))

## [1.4.10](https://github.com/eoeo-org/pcsc-rs/compare/v1.4.9...v1.4.10) (2025-02-16)


### Dependencies

* lock file maintenance ([#591](https://github.com/eoeo-org/pcsc-rs/issues/591)) ([d904fe1](https://github.com/eoeo-org/pcsc-rs/commit/d904fe196b04f12b14bcd93e857b5be4dc45dee4))
* update rust crate openssl to v0.10.71 ([#589](https://github.com/eoeo-org/pcsc-rs/issues/589)) ([3a3468f](https://github.com/eoeo-org/pcsc-rs/commit/3a3468fffebd004dcea9dcb31ac1d718beaab8f8))

## [1.4.9](https://github.com/eoeo-org/pcsc-rs/compare/v1.4.8...v1.4.9) (2025-02-11)


### Dependencies

* lock file maintenance ([#586](https://github.com/eoeo-org/pcsc-rs/issues/586)) ([46ac833](https://github.com/eoeo-org/pcsc-rs/commit/46ac833e332ddc6b58925e13afabe985bd8501c1))

## [1.4.8](https://github.com/eoeo-org/pcsc-rs/compare/v1.4.7...v1.4.8) (2025-02-10)


### Dependencies

* lock file maintenance ([#584](https://github.com/eoeo-org/pcsc-rs/issues/584)) ([32b53ca](https://github.com/eoeo-org/pcsc-rs/commit/32b53cab3775851910ae901b4e8b828971748810))

## [1.4.7](https://github.com/eoeo-org/pcsc-rs/compare/v1.4.6...v1.4.7) (2025-02-03)


### Dependencies

* lock file maintenance ([#580](https://github.com/eoeo-org/pcsc-rs/issues/580)) ([5fcd0d4](https://github.com/eoeo-org/pcsc-rs/commit/5fcd0d47f3f70c67b0d459f311135a10235f06d4))
* update rust crate openssl to v0.10.70 [security] ([#581](https://github.com/eoeo-org/pcsc-rs/issues/581)) ([d905211](https://github.com/eoeo-org/pcsc-rs/commit/d90521192a36293eaf4eeb4bcef3630159fc61a7))

## [1.4.6](https://github.com/eoeo-org/pcsc-rs/compare/v1.4.5...v1.4.6) (2025-01-30)


### Dependencies

* update rust crate serde_json to v1.0.138 ([#576](https://github.com/eoeo-org/pcsc-rs/issues/576)) ([61cbf70](https://github.com/eoeo-org/pcsc-rs/commit/61cbf7016848e17d6d47e37396e47b0dac2cb3cb))

## [1.4.5](https://github.com/eoeo-org/pcsc-rs/compare/v1.4.4...v1.4.5) (2025-01-27)


### Dependencies

* lock file maintenance ([#568](https://github.com/eoeo-org/pcsc-rs/issues/568)) ([ffbbcef](https://github.com/eoeo-org/pcsc-rs/commit/ffbbcef1798a0e8feb2471b2fd8ad1f0b7327be8))
* update rust crate openssl to v0.10.69 ([#572](https://github.com/eoeo-org/pcsc-rs/issues/572)) ([58d37ad](https://github.com/eoeo-org/pcsc-rs/commit/58d37ad7b57e18642be12adf1fd501aa7459db71))
* update rust crate serde_json to v1.0.137 ([#567](https://github.com/eoeo-org/pcsc-rs/issues/567)) ([00cfc00](https://github.com/eoeo-org/pcsc-rs/commit/00cfc0096f8be968f6133b411fef01cd5f79d780))

## [1.4.4](https://github.com/eoeo-org/pcsc-rs/compare/v1.4.3...v1.4.4) (2025-01-19)


### Dependencies

* update rust crate serde_json to v1.0.136 ([#563](https://github.com/eoeo-org/pcsc-rs/issues/563)) ([9d6ce89](https://github.com/eoeo-org/pcsc-rs/commit/9d6ce897e970e0f77b1f3c937958fcdeb531ca07))

## [1.4.3](https://github.com/eoeo-org/pcsc-rs/compare/v1.4.2...v1.4.3) (2025-01-18)


### Dependencies

* lock file maintenance ([#560](https://github.com/eoeo-org/pcsc-rs/issues/560)) ([6018150](https://github.com/eoeo-org/pcsc-rs/commit/6018150440155e169c51b9ee0e75d6d9ae18ce48))

## [1.4.2](https://github.com/eoeo-org/pcsc-rs/compare/v1.4.1...v1.4.2) (2025-01-10)


### Dependencies

* lock file maintenance ([#543](https://github.com/eoeo-org/pcsc-rs/issues/543)) ([2e188b6](https://github.com/eoeo-org/pcsc-rs/commit/2e188b6fd25cde533a6e0550a34f59e7baf3a0cc))
* lock file maintenance ([#550](https://github.com/eoeo-org/pcsc-rs/issues/550)) ([f1b1055](https://github.com/eoeo-org/pcsc-rs/commit/f1b10550463ade78769f7eedfddace198000b056))
* lock file maintenance ([#556](https://github.com/eoeo-org/pcsc-rs/issues/556)) ([d05f3aa](https://github.com/eoeo-org/pcsc-rs/commit/d05f3aa5517f16a86f5a294bc369dd54a6f85f42))
* update rust crate itertools to v0.14.0 ([#554](https://github.com/eoeo-org/pcsc-rs/issues/554)) ([39271ba](https://github.com/eoeo-org/pcsc-rs/commit/39271baab6256ad6d626fe1b68c29e4d01ff964a))
* update rust crate self_update to v0.42.0 ([#555](https://github.com/eoeo-org/pcsc-rs/issues/555)) ([63b3e92](https://github.com/eoeo-org/pcsc-rs/commit/63b3e92c8ac66b1cc8826268e412413b21a026a8))
* update rust crate serde_json to v1.0.135 ([#548](https://github.com/eoeo-org/pcsc-rs/issues/548)) ([73db11b](https://github.com/eoeo-org/pcsc-rs/commit/73db11bbe375188b7d4aa778352c5abffffb202c))
* update rust crate sysinfo to v0.33.1 ([#549](https://github.com/eoeo-org/pcsc-rs/issues/549)) ([63d02fc](https://github.com/eoeo-org/pcsc-rs/commit/63d02fcf26965b9b667d7bb0e222ca87701b6d5e))
* update serde monorepo to v1.0.217 ([#551](https://github.com/eoeo-org/pcsc-rs/issues/551)) ([125488d](https://github.com/eoeo-org/pcsc-rs/commit/125488d9fa4277322576a3c0d2fcc16afa8a31bb))

## [1.4.1](https://github.com/eoeo-org/pcsc-rs/compare/v1.4.0...v1.4.1) (2024-12-11)


### Dependencies

* lock file maintenance ([#535](https://github.com/eoeo-org/pcsc-rs/issues/535)) ([3f0da52](https://github.com/eoeo-org/pcsc-rs/commit/3f0da528577519487a591b47ca58a67ec525f266))
* update serde monorepo to v1.0.216 ([#540](https://github.com/eoeo-org/pcsc-rs/issues/540)) ([f3eb0a5](https://github.com/eoeo-org/pcsc-rs/commit/f3eb0a5419a285e9511161e40517553b5ac42832))

## [1.4.0](https://github.com/eoeo-org/pcsc-rs/compare/v1.3.5...v1.4.0) (2024-12-05)


### Features

* ensure StorageData is not duplicated ([#529](https://github.com/eoeo-org/pcsc-rs/issues/529)) ([a68367c](https://github.com/eoeo-org/pcsc-rs/commit/a68367c4da4b57ba5d374867b0318036b9a8441f))
* exclude storage with a total disk size of zero ([#531](https://github.com/eoeo-org/pcsc-rs/issues/531)) ([40dbab6](https://github.com/eoeo-org/pcsc-rs/commit/40dbab6127d6cfac59ed9ac1b55ee15f96e75a7a))


### Dependencies

* lock file maintenance ([#518](https://github.com/eoeo-org/pcsc-rs/issues/518)) ([d521f59](https://github.com/eoeo-org/pcsc-rs/commit/d521f59a4d0eff1ead6937297813f2b812281bf0))
* lock file maintenance ([#532](https://github.com/eoeo-org/pcsc-rs/issues/532)) ([1c8e3d5](https://github.com/eoeo-org/pcsc-rs/commit/1c8e3d521afae43cd93af3e84f2309e39ea31ee7))
* pin rust crate itertools to =0.13.0 ([#530](https://github.com/eoeo-org/pcsc-rs/issues/530)) ([565f111](https://github.com/eoeo-org/pcsc-rs/commit/565f111cc98b4a6ad6ce9700490ced9444536e66))
* update rust crate sysinfo to v0.33.0 ([#526](https://github.com/eoeo-org/pcsc-rs/issues/526)) ([76d00ad](https://github.com/eoeo-org/pcsc-rs/commit/76d00add56fe8a65832e9cd3f981820a6ba947c5))
* update sysinfo to disable default features and enable ones ([#528](https://github.com/eoeo-org/pcsc-rs/issues/528)) ([0959626](https://github.com/eoeo-org/pcsc-rs/commit/0959626769ca7ae5e96a60098e52664052729cfe))


### Code Refactoring

* move to lib.rs ([#522](https://github.com/eoeo-org/pcsc-rs/issues/522)) ([01092fa](https://github.com/eoeo-org/pcsc-rs/commit/01092fa01d9c7596efbfe810edc6f7bb9efb1068))
* simplify system status management with SysinfoInstance ([#525](https://github.com/eoeo-org/pcsc-rs/issues/525)) ([d23832a](https://github.com/eoeo-org/pcsc-rs/commit/d23832a4903cbe5399d94cd31f41d843c5be37cb))

## [1.3.5](https://github.com/eoeo-org/pcsc-rs/compare/v1.3.4...v1.3.5) (2024-11-27)


### Dependencies

* update rust crate sysinfo to v0.32.1 ([#512](https://github.com/eoeo-org/pcsc-rs/issues/512)) ([c40aa2e](https://github.com/eoeo-org/pcsc-rs/commit/c40aa2e6f144d2c29e606dc89deb2c03f8a432b1))

## [1.3.4](https://github.com/eoeo-org/pcsc-rs/compare/v1.3.3...v1.3.4) (2024-11-26)


### Dependencies

* lock file maintenance ([#508](https://github.com/eoeo-org/pcsc-rs/issues/508)) ([a265f0e](https://github.com/eoeo-org/pcsc-rs/commit/a265f0e9efa926bda303a12c9d9584f903a4b5fc))


### Code Refactoring

* remove unused event handling ([#511](https://github.com/eoeo-org/pcsc-rs/issues/511)) ([f84732b](https://github.com/eoeo-org/pcsc-rs/commit/f84732bee47b8f2d6f50c737dbbac11a3a390173))

## [1.3.3](https://github.com/eoeo-org/pcsc-rs/compare/v1.3.2...v1.3.3) (2024-11-23)


### Dependencies

* lock file maintenance ([#502](https://github.com/eoeo-org/pcsc-rs/issues/502)) ([25ad69b](https://github.com/eoeo-org/pcsc-rs/commit/25ad69bcb1731122523e8810c76226de812ef7aa))
* update rust crate serde_json to v1.0.133 ([#501](https://github.com/eoeo-org/pcsc-rs/issues/501)) ([792d528](https://github.com/eoeo-org/pcsc-rs/commit/792d528c98fda47d09fbd781ced53b46cdd92386))

## [1.3.2](https://github.com/eoeo-org/pcsc-rs/compare/v1.3.1...v1.3.2) (2024-11-16)


### Performance Improvements

* get only necessary information ([#499](https://github.com/eoeo-org/pcsc-rs/issues/499)) ([5589fca](https://github.com/eoeo-org/pcsc-rs/commit/5589fcaa55fbac936674eb9d619868412212b854))

## [1.3.1](https://github.com/eoeo-org/pcsc-rs/compare/v1.3.0...v1.3.1) (2024-11-16)


### Code Refactoring

* improve the process of getting disks information ([#497](https://github.com/eoeo-org/pcsc-rs/issues/497)) ([e86e9e1](https://github.com/eoeo-org/pcsc-rs/commit/e86e9e1cb6a6024924401a9a355d0d2a28ada0e0))

## [1.3.0](https://github.com/eoeo-org/pcsc-rs/compare/v1.2.1...v1.3.0) (2024-11-15)


### Features

* re-add self updater ([#494](https://github.com/eoeo-org/pcsc-rs/issues/494)) ([26ce901](https://github.com/eoeo-org/pcsc-rs/commit/26ce901a520d5434c908ca010784efe26b402d66))


### Dependencies

* lock file maintenance ([#491](https://github.com/eoeo-org/pcsc-rs/issues/491)) ([2a3f98b](https://github.com/eoeo-org/pcsc-rs/commit/2a3f98bb608ddcb452052f578e5a7cc7680fbf93))
* lock file maintenance ([#493](https://github.com/eoeo-org/pcsc-rs/issues/493)) ([4e035e6](https://github.com/eoeo-org/pcsc-rs/commit/4e035e6cbdc440613084901d2f60e278792652b3))
* update rust crate regex to v1.11.1 ([#488](https://github.com/eoeo-org/pcsc-rs/issues/488)) ([c69da6f](https://github.com/eoeo-org/pcsc-rs/commit/c69da6f7e88c523d0344aee21112a5e5be0504ca))
* update serde monorepo to v1.0.215 ([#492](https://github.com/eoeo-org/pcsc-rs/issues/492)) ([b35b767](https://github.com/eoeo-org/pcsc-rs/commit/b35b7679c1c3fde9e172b4eae5f103065eee6ee4))


### Reverts

* self updator ([#486](https://github.com/eoeo-org/pcsc-rs/issues/486)) ([1a4250b](https://github.com/eoeo-org/pcsc-rs/commit/1a4250ba6d868aa990c23bd6c078c315f2551a33))

## [1.2.1](https://github.com/eoeo-org/pcsc-rs/compare/v1.2.0...v1.2.1) (2024-10-23)


### Dependencies

* lock file maintenance ([#467](https://github.com/eoeo-org/pcsc-rs/issues/467)) ([4b796ee](https://github.com/eoeo-org/pcsc-rs/commit/4b796ee6ca015e9365d7012b1020c81a39cb4977))
* lock file maintenance ([#477](https://github.com/eoeo-org/pcsc-rs/issues/477)) ([cc96c8f](https://github.com/eoeo-org/pcsc-rs/commit/cc96c8fe0757282a5754334382091d1f514bd3b3))
* update rust crate openssl to v0.10.67 ([#472](https://github.com/eoeo-org/pcsc-rs/issues/472)) ([1037b75](https://github.com/eoeo-org/pcsc-rs/commit/1037b7580eb18dd3ad777a388655c8fa80eb71db))
* update rust crate openssl to v0.10.68 ([#474](https://github.com/eoeo-org/pcsc-rs/issues/474)) ([d4f27e6](https://github.com/eoeo-org/pcsc-rs/commit/d4f27e65a0c4a2c28fc91a34b116a588f8640d90))
* update rust crate serde_json to v1.0.132 ([#475](https://github.com/eoeo-org/pcsc-rs/issues/475)) ([820092c](https://github.com/eoeo-org/pcsc-rs/commit/820092c870fa7fd1d6827b83e1080b58c1c69adb))
* update serde monorepo to v1.0.211 ([#479](https://github.com/eoeo-org/pcsc-rs/issues/479)) ([30d6539](https://github.com/eoeo-org/pcsc-rs/commit/30d6539d43529c4f1e90c61891c020ba6fa66423))
* update serde monorepo to v1.0.213 ([#483](https://github.com/eoeo-org/pcsc-rs/issues/483)) ([49eadb9](https://github.com/eoeo-org/pcsc-rs/commit/49eadb97cacafdf1a186e608a4330808bf3f8fac))

## [1.2.0](https://github.com/eoeo-org/pcsc-rs/compare/v1.1.10...v1.2.0) (2024-10-06)


### Features

* add self updator ([#453](https://github.com/eoeo-org/pcsc-rs/issues/453)) ([9d0c686](https://github.com/eoeo-org/pcsc-rs/commit/9d0c686af8c8f84a04712537abea3f9bb61e672a))
* display git describe version ([#445](https://github.com/eoeo-org/pcsc-rs/issues/445)) ([fbf9a7c](https://github.com/eoeo-org/pcsc-rs/commit/fbf9a7cecf8b26fee7f456500cfc0254feff9e5e))


### Bug Fixes

* **deps:** pin rust crate self_update to =0.41.0 ([#464](https://github.com/eoeo-org/pcsc-rs/issues/464)) ([7fd1784](https://github.com/eoeo-org/pcsc-rs/commit/7fd1784bc76342b99c8f3610e935374ec33c5a70))
* **deps:** update rust crate regex to v1.11.0 ([#459](https://github.com/eoeo-org/pcsc-rs/issues/459)) ([457cd4d](https://github.com/eoeo-org/pcsc-rs/commit/457cd4d08bda037fa3916ddbc433481a4d186523))
* **deps:** update rust crate sysinfo to v0.32.0 ([#462](https://github.com/eoeo-org/pcsc-rs/issues/462)) ([009c29e](https://github.com/eoeo-org/pcsc-rs/commit/009c29e294db87af50d8924556f034b248176f6a))

## [1.1.10](https://github.com/eoeo-org/pcsc-rs/compare/v1.1.9...v1.1.10) (2024-09-18)


### Build System

* update release profile ([#442](https://github.com/eoeo-org/pcsc-rs/issues/442)) ([1caf21c](https://github.com/eoeo-org/pcsc-rs/commit/1caf21c4021b4e3fad4dc49264a195a1e18d8220))

## [1.1.9](https://github.com/eoeo-org/pcsc-rs/compare/v1.1.8...v1.1.9) (2024-09-18)


### Bug Fixes

* **deps:** update rust crate serde_json to v1.0.128 ([#437](https://github.com/eoeo-org/pcsc-rs/issues/437)) ([d0e6f4d](https://github.com/eoeo-org/pcsc-rs/commit/d0e6f4d7992d04e9eb0e5dd6f5ef1ece87ff06f6))
* **deps:** update rust crate sysinfo to v0.31.4 ([#435](https://github.com/eoeo-org/pcsc-rs/issues/435)) ([11b616f](https://github.com/eoeo-org/pcsc-rs/commit/11b616fd396938d7ba88b41ab93abf14d0cec8d3))
* **deps:** update serde monorepo to v1.0.210 ([#438](https://github.com/eoeo-org/pcsc-rs/issues/438)) ([c4caf75](https://github.com/eoeo-org/pcsc-rs/commit/c4caf7563d47e8fa52430868ebfb580e77d63702))

## [1.1.8](https://github.com/eoeo-org/pcsc-rs/compare/v1.1.7...v1.1.8) (2024-08-26)


### Bug Fixes

* **deps:** update rust crate openssl to v0.10.66 ([#423](https://github.com/eoeo-org/pcsc-rs/issues/423)) ([0c4e132](https://github.com/eoeo-org/pcsc-rs/commit/0c4e132d46ee8ad9c6d6244e2d33b73a567d35ea))
* **deps:** update rust crate regex to v1.10.6 ([#428](https://github.com/eoeo-org/pcsc-rs/issues/428)) ([08bebc3](https://github.com/eoeo-org/pcsc-rs/commit/08bebc38bbeb6cdcf0273bc6b153eb7e0bb594a3))
* **deps:** update rust crate serde_json to v1.0.127 ([#429](https://github.com/eoeo-org/pcsc-rs/issues/429)) ([797678d](https://github.com/eoeo-org/pcsc-rs/commit/797678d4c9eb743348ede7fa9779462fa2b538cf))
* **deps:** update rust crate sysinfo to v0.31.3 ([#431](https://github.com/eoeo-org/pcsc-rs/issues/431)) ([6a60c2c](https://github.com/eoeo-org/pcsc-rs/commit/6a60c2c9a674f4c777096db36ed1669b9c7f12a3))
* **deps:** update serde monorepo to v1.0.209 ([#430](https://github.com/eoeo-org/pcsc-rs/issues/430)) ([e121735](https://github.com/eoeo-org/pcsc-rs/commit/e121735fcf8ea75f2539a44272f7c1e4866cfbc4))

## [1.1.7](https://github.com/eoeo-org/pcsc-rs/compare/v1.1.6...v1.1.7) (2024-07-21)


### Bug Fixes

* **deps:** update rust crate openssl to v0.10.65 ([#422](https://github.com/eoeo-org/pcsc-rs/issues/422)) ([7734061](https://github.com/eoeo-org/pcsc-rs/commit/77340610fc3c68ff0da1315fbdc1911eed1f8a7a))
* **deps:** update rust crate serde_json to v1.0.120 ([#412](https://github.com/eoeo-org/pcsc-rs/issues/412)) ([5d68b6f](https://github.com/eoeo-org/pcsc-rs/commit/5d68b6f4b461f240506a13432fb0cd69a493a003))
* **deps:** update rust crate sysinfo to v0.30.13 ([#418](https://github.com/eoeo-org/pcsc-rs/issues/418)) ([9294466](https://github.com/eoeo-org/pcsc-rs/commit/929446607067e384abace212e0aca3b0f9717664))
* **deps:** update serde monorepo to v1.0.204 ([#420](https://github.com/eoeo-org/pcsc-rs/issues/420)) ([83d20fb](https://github.com/eoeo-org/pcsc-rs/commit/83d20fb294b87646b214be78faa505c322dbd713))

## [1.1.6](https://github.com/j-eoeo/pcsc-rs/compare/v1.1.5...v1.1.6) (2024-07-01)


### Bug Fixes

* **deps:** update rust crate regex to v1.10.5 ([#404](https://github.com/j-eoeo/pcsc-rs/issues/404)) ([d33e5bf](https://github.com/j-eoeo/pcsc-rs/commit/d33e5bff4dd64344eb00d27b4bcb7ff29119f413))
* **deps:** update rust crate serde_json to v1.0.119 ([#408](https://github.com/j-eoeo/pcsc-rs/issues/408)) ([424cc8f](https://github.com/j-eoeo/pcsc-rs/commit/424cc8fe8ac78c39b3cc2cb6928333019c4e7190))
* **deps:** update serde monorepo to v1.0.203 ([#399](https://github.com/j-eoeo/pcsc-rs/issues/399)) ([7e941da](https://github.com/j-eoeo/pcsc-rs/commit/7e941da8e9c33b92c5dc85d9dcf6456ca699b6ff))

## [1.1.5](https://github.com/j-eoeo/pcsc-rs/compare/v1.1.4...v1.1.5) (2024-05-16)


### Bug Fixes

* **deps:** update rust crate arc-swap to v1.7.1 ([#390](https://github.com/j-eoeo/pcsc-rs/issues/390)) ([4136819](https://github.com/j-eoeo/pcsc-rs/commit/41368195ff4239f04ee530c33a1b6b07895afac8))
* **deps:** update rust crate openssl to v0.10.64 ([#386](https://github.com/j-eoeo/pcsc-rs/issues/386)) ([309fd6d](https://github.com/j-eoeo/pcsc-rs/commit/309fd6d2a9653365213277f6717ee70dc1a918ca))
* **deps:** update rust crate regex to v1.10.4 ([#387](https://github.com/j-eoeo/pcsc-rs/issues/387)) ([fd27bf3](https://github.com/j-eoeo/pcsc-rs/commit/fd27bf34b7b0be9adaca068acdc701f3777435d8))
* **deps:** update rust crate rust_socketio to v0.6.0 ([#391](https://github.com/j-eoeo/pcsc-rs/issues/391)) ([0acb7bd](https://github.com/j-eoeo/pcsc-rs/commit/0acb7bd686312aa7d55b06900347df39cad6f705))
* **deps:** update rust crate serde_json to v1.0.113 ([#377](https://github.com/j-eoeo/pcsc-rs/issues/377)) ([36deae6](https://github.com/j-eoeo/pcsc-rs/commit/36deae6293886db9280d1e3b0e5b8792d81e5807))
* **deps:** update rust crate serde_json to v1.0.117 ([#388](https://github.com/j-eoeo/pcsc-rs/issues/388)) ([43f20fc](https://github.com/j-eoeo/pcsc-rs/commit/43f20fc4f3d727162f7290c99d79516d4540ad7e))
* **deps:** update rust crate sysinfo to v0.30.12 ([#389](https://github.com/j-eoeo/pcsc-rs/issues/389)) ([d0da6c4](https://github.com/j-eoeo/pcsc-rs/commit/d0da6c4f27b13f9db962d1f5db3c0ef486435f2e))
* **deps:** update serde monorepo to v1.0.201 ([#378](https://github.com/j-eoeo/pcsc-rs/issues/378)) ([28c4317](https://github.com/j-eoeo/pcsc-rs/commit/28c4317884bf83d3d02ae74a22e006fcee8559e6))
* **deps:** update serde monorepo to v1.0.202 ([#394](https://github.com/j-eoeo/pcsc-rs/issues/394)) ([1545980](https://github.com/j-eoeo/pcsc-rs/commit/1545980cbd0a83c60a636d3e0871b616306aceab))
* update readme preview screenshots and ALT ([#382](https://github.com/j-eoeo/pcsc-rs/issues/382)) ([db47aba](https://github.com/j-eoeo/pcsc-rs/commit/db47aba14164b7de8cffa1e5c66faf9ad984e2a2))

## [1.1.4](https://github.com/j-eoeo/pcsc-rs/compare/v1.1.3...v1.1.4) (2024-01-26)


### Bug Fixes

* Update version in SystemStatus struct ([#374](https://github.com/j-eoeo/pcsc-rs/issues/374)) ([efa4e60](https://github.com/j-eoeo/pcsc-rs/commit/efa4e6096e54dd749bd34377df0b07a16368669d))

## [1.1.3](https://github.com/j-eoeo/pcsc-rs/compare/v1.1.2...v1.1.3) (2024-01-26)


### Bug Fixes

* **deps:** update rust crate openssl to v0.10.60 ([#299](https://github.com/j-eoeo/pcsc-rs/issues/299)) ([d50f6f6](https://github.com/j-eoeo/pcsc-rs/commit/d50f6f69ba4ea930e80c930db339785d59383c3d))
* **deps:** update rust crate openssl to v0.10.61 ([#312](https://github.com/j-eoeo/pcsc-rs/issues/312)) ([bb0b2f9](https://github.com/j-eoeo/pcsc-rs/commit/bb0b2f94647b1fa091e6005a7cd152cb33dc9a02))
* **deps:** update rust crate openssl to v0.10.62 ([#331](https://github.com/j-eoeo/pcsc-rs/issues/331)) ([85e802d](https://github.com/j-eoeo/pcsc-rs/commit/85e802d1f2d8aed74cf396373fd43bae6c73e96b))
* **deps:** update rust crate openssl to v0.10.63 ([#365](https://github.com/j-eoeo/pcsc-rs/issues/365)) ([9cda417](https://github.com/j-eoeo/pcsc-rs/commit/9cda417afbeeb46d0e8a1652e649763b58cdc04b))
* **deps:** update rust crate regex to v1.10.3 ([#368](https://github.com/j-eoeo/pcsc-rs/issues/368)) ([95ac252](https://github.com/j-eoeo/pcsc-rs/commit/95ac25243f7dd9e45a66a0fb571f32d2d016cbdf))
* **deps:** update rust crate rust_socketio to v0.4.4 ([#292](https://github.com/j-eoeo/pcsc-rs/issues/292)) ([1387c39](https://github.com/j-eoeo/pcsc-rs/commit/1387c39b2b853e330de49e80b36a515d53e7915f))
* **deps:** update rust crate serde_json to v1.0.109 ([#341](https://github.com/j-eoeo/pcsc-rs/issues/341)) ([2ce04cc](https://github.com/j-eoeo/pcsc-rs/commit/2ce04cc554432f50cba9231b6cb115458640f311))
* **deps:** update rust crate serde_json to v1.0.110 ([#343](https://github.com/j-eoeo/pcsc-rs/issues/343)) ([9515b61](https://github.com/j-eoeo/pcsc-rs/commit/9515b61476287e8ff7baef2fb3f7f8685d2980dd))
* **deps:** update rust crate serde_json to v1.0.111 ([#345](https://github.com/j-eoeo/pcsc-rs/issues/345)) ([c3efc60](https://github.com/j-eoeo/pcsc-rs/commit/c3efc60171dd0768a1fdfe0f0bca622a5e36b4b0))
* **deps:** update rust crate sysinfo to v0.29.11 ([#301](https://github.com/j-eoeo/pcsc-rs/issues/301)) ([7977a8e](https://github.com/j-eoeo/pcsc-rs/commit/7977a8ebc66ef84593bd7ed75a23aa8f09cb01bf))
* **deps:** update rust crate sysinfo to v0.30.5 ([#328](https://github.com/j-eoeo/pcsc-rs/issues/328)) ([3a5607b](https://github.com/j-eoeo/pcsc-rs/commit/3a5607bf3f1a5c65d8add731ee4d0c024c4c3517))
* **deps:** update serde monorepo to v1.0.193 ([#298](https://github.com/j-eoeo/pcsc-rs/issues/298)) ([1a18376](https://github.com/j-eoeo/pcsc-rs/commit/1a18376cebab5a71aa6fc2d4f9ca1c2d1a9f115b))
* **deps:** update serde monorepo to v1.0.194 ([#344](https://github.com/j-eoeo/pcsc-rs/issues/344)) ([1161a02](https://github.com/j-eoeo/pcsc-rs/commit/1161a024ab529041ce7e32a5206a030c8586a3ef))
* **deps:** update serde monorepo to v1.0.195 ([#348](https://github.com/j-eoeo/pcsc-rs/issues/348)) ([325935c](https://github.com/j-eoeo/pcsc-rs/commit/325935c2c8db235680e7c0f1afb5ed35792d1185))

## [1.1.2](https://github.com/kazukazu123123/pcsc-rs/compare/v1.1.1...v1.1.2) (2023-11-12)


### Bug Fixes

* **deps:** update rust crate openssl to v0.10.58 ([#279](https://github.com/kazukazu123123/pcsc-rs/issues/279)) ([06e19e9](https://github.com/kazukazu123123/pcsc-rs/commit/06e19e90dfa8ece72f6278cca72afd9a8bcd6185))
* **deps:** update rust crate openssl to v0.10.59 ([#280](https://github.com/kazukazu123123/pcsc-rs/issues/280)) ([d507704](https://github.com/kazukazu123123/pcsc-rs/commit/d507704722251ff649842133c8351cf65d7337c0))
* **deps:** update rust crate regex to v1.10.2 ([#263](https://github.com/kazukazu123123/pcsc-rs/issues/263)) ([e03a76d](https://github.com/kazukazu123123/pcsc-rs/commit/e03a76dd21496c6374490de70d93a2ac28c16e2f))
* **deps:** update rust crate serde_json to v1.0.108 ([#277](https://github.com/kazukazu123123/pcsc-rs/issues/277)) ([260ee59](https://github.com/kazukazu123123/pcsc-rs/commit/260ee590f1452566079d63273d1493779fec8413))
* **deps:** update serde monorepo to v1.0.190 ([#271](https://github.com/kazukazu123123/pcsc-rs/issues/271)) ([8db458a](https://github.com/kazukazu123123/pcsc-rs/commit/8db458a5881914d54ff3981a329c2ae98e30cdda))
* **deps:** update serde monorepo to v1.0.192 ([#282](https://github.com/kazukazu123123/pcsc-rs/issues/282)) ([47ba177](https://github.com/kazukazu123123/pcsc-rs/commit/47ba1775db4d3965e3a3e630da0b5ce06f4a4b6f))

## [1.1.1](https://github.com/kazukazu123123/pcsc-rs/compare/v1.1.0...v1.1.1) (2023-10-16)


### Bug Fixes

* **deps:** update rust crate regex to v1.10.0 ([#258](https://github.com/kazukazu123123/pcsc-rs/issues/258)) ([dec6949](https://github.com/kazukazu123123/pcsc-rs/commit/dec6949d5fd7cecc0c295ede88004cb1ef3eb5c7))
* **deps:** update rust crate regex to v1.10.1 ([#261](https://github.com/kazukazu123123/pcsc-rs/issues/261)) ([74b85c1](https://github.com/kazukazu123123/pcsc-rs/commit/74b85c1e9714fc793da163d1d8a90ec61ea40d64))
* **deps:** update rust crate regex to v1.9.6 ([#250](https://github.com/kazukazu123123/pcsc-rs/issues/250)) ([6f58010](https://github.com/kazukazu123123/pcsc-rs/commit/6f5801085aefb1cdaa9f11cfea813ee308a41786))
* **deps:** update serde monorepo to v1.0.189 ([#260](https://github.com/kazukazu123123/pcsc-rs/issues/260)) ([2c88aeb](https://github.com/kazukazu123123/pcsc-rs/commit/2c88aeb9639a22fb700eed18ec882de489e42e2f))

## [1.1.0](https://github.com/kazukazu123123/pcsc-rs/compare/v1.0.0...v1.1.0) (2023-09-22)


### Features

* support multiple mount points ([65e18fe](https://github.com/kazukazu123123/pcsc-rs/commit/65e18fe02ef3f1c6b396ee8061c988efc1a95cde))


### Bug Fixes

* **deps:** update rust crate openssl to v0.10.54 ([#98](https://github.com/kazukazu123123/pcsc-rs/issues/98)) ([a83b81c](https://github.com/kazukazu123123/pcsc-rs/commit/a83b81c429b005c182eb3b48d8e2a4e43bf7cb50))
* **deps:** update rust crate openssl to v0.10.55 ([#129](https://github.com/kazukazu123123/pcsc-rs/issues/129)) ([429807c](https://github.com/kazukazu123123/pcsc-rs/commit/429807cb164c3ca0f7cf25f71ab637de7b1bf288))
* **deps:** update rust crate openssl to v0.10.56 ([#185](https://github.com/kazukazu123123/pcsc-rs/issues/185)) ([3644598](https://github.com/kazukazu123123/pcsc-rs/commit/36445980d365c6982ef902bf76abcf523d380994))
* **deps:** update rust crate openssl to v0.10.57 ([#204](https://github.com/kazukazu123123/pcsc-rs/issues/204)) ([00f248f](https://github.com/kazukazu123123/pcsc-rs/commit/00f248fd9564871042463778f54d35413f2d38b5))
* **deps:** update rust crate regex to v1.8.4 ([#104](https://github.com/kazukazu123123/pcsc-rs/issues/104)) ([05ff36e](https://github.com/kazukazu123123/pcsc-rs/commit/05ff36e0cc93195da2a5631a530782eb1a08e52e))
* **deps:** update rust crate regex to v1.9.0 ([#157](https://github.com/kazukazu123123/pcsc-rs/issues/157)) ([c5d3cd9](https://github.com/kazukazu123123/pcsc-rs/commit/c5d3cd94702416dc6ed6e43ae98c3bdb75f368f2))
* **deps:** update rust crate regex to v1.9.1 ([#159](https://github.com/kazukazu123123/pcsc-rs/issues/159)) ([0091a82](https://github.com/kazukazu123123/pcsc-rs/commit/0091a82d12e1b87ed807528a1a61eb612b072f4e))
* **deps:** update rust crate regex to v1.9.3 ([#183](https://github.com/kazukazu123123/pcsc-rs/issues/183)) ([f89e270](https://github.com/kazukazu123123/pcsc-rs/commit/f89e270cc688c0cb3ab60b729b499b56574dde1c))
* **deps:** update rust crate regex to v1.9.4 ([#202](https://github.com/kazukazu123123/pcsc-rs/issues/202)) ([223b28d](https://github.com/kazukazu123123/pcsc-rs/commit/223b28dcf09a9ddc454b776f75d16cf59153608e))
* **deps:** update rust crate regex to v1.9.5 ([#210](https://github.com/kazukazu123123/pcsc-rs/issues/210)) ([1dab625](https://github.com/kazukazu123123/pcsc-rs/commit/1dab625088279c5470d173bc162083afc4d2cec1))
* **deps:** update rust crate rust_socketio to v0.4.2 ([#135](https://github.com/kazukazu123123/pcsc-rs/issues/135)) ([7b62dd1](https://github.com/kazukazu123123/pcsc-rs/commit/7b62dd1ce4a767e2831220893e571d03530dc625))
* **deps:** update rust crate rust_socketio to v0.4.3 ([#161](https://github.com/kazukazu123123/pcsc-rs/issues/161)) ([0eb3cea](https://github.com/kazukazu123123/pcsc-rs/commit/0eb3ceaf266787ab70e8935be08214bb3efceaa5))
* **deps:** update rust crate serde_json to v1.0.106 ([#156](https://github.com/kazukazu123123/pcsc-rs/issues/156)) ([b96a50c](https://github.com/kazukazu123123/pcsc-rs/commit/b96a50cfeed138daf59e44bb024104027424098a))
* **deps:** update rust crate serde_json to v1.0.107 ([#227](https://github.com/kazukazu123123/pcsc-rs/issues/227)) ([3a1a8f8](https://github.com/kazukazu123123/pcsc-rs/commit/3a1a8f85cb034d00a629eaa7a5c03108200df173))
* **deps:** update rust crate serde_json to v1.0.97 ([#122](https://github.com/kazukazu123123/pcsc-rs/issues/122)) ([108b2b7](https://github.com/kazukazu123123/pcsc-rs/commit/108b2b7b91b7cfc67dc5624eb2134cbc084bc956))
* **deps:** update rust crate serde_json to v1.0.98 ([#132](https://github.com/kazukazu123123/pcsc-rs/issues/132)) ([a2e0ecd](https://github.com/kazukazu123123/pcsc-rs/commit/a2e0ecdec822f97e2e8f670ebaf144f07cb4ca41))
* **deps:** update rust crate serde_json to v1.0.99 ([#133](https://github.com/kazukazu123123/pcsc-rs/issues/133)) ([efea98c](https://github.com/kazukazu123123/pcsc-rs/commit/efea98c712fa20b1afb412242180d14caa8bc62f))
* **deps:** update rust crate sysinfo to v0.29.1 ([#105](https://github.com/kazukazu123123/pcsc-rs/issues/105)) ([6118219](https://github.com/kazukazu123123/pcsc-rs/commit/61182193fc0a843a4a7e5ccd8ff48e384581bbaa))
* **deps:** update rust crate sysinfo to v0.29.10 ([#214](https://github.com/kazukazu123123/pcsc-rs/issues/214)) ([faeda01](https://github.com/kazukazu123123/pcsc-rs/commit/faeda01169e4e6a0cc1a063424d54a0b5a6472f1))
* **deps:** update rust crate sysinfo to v0.29.2 ([#106](https://github.com/kazukazu123123/pcsc-rs/issues/106)) ([af0eaaf](https://github.com/kazukazu123123/pcsc-rs/commit/af0eaaf4fa48536d97c460e041a313d2085f96a9))
* **deps:** update rust crate sysinfo to v0.29.3 ([#142](https://github.com/kazukazu123123/pcsc-rs/issues/142)) ([e6c4222](https://github.com/kazukazu123123/pcsc-rs/commit/e6c4222bea89db8dbe203157a95fe94d45a5fb7e))
* **deps:** update rust crate sysinfo to v0.29.4 ([#154](https://github.com/kazukazu123123/pcsc-rs/issues/154)) ([25069d0](https://github.com/kazukazu123123/pcsc-rs/commit/25069d040f66a7236121f93b18333d68c572115c))
* **deps:** update rust crate sysinfo to v0.29.6 ([#165](https://github.com/kazukazu123123/pcsc-rs/issues/165)) ([050ed9a](https://github.com/kazukazu123123/pcsc-rs/commit/050ed9a92352293b841386a9ed3903f509a24153))
* **deps:** update rust crate sysinfo to v0.29.7 ([#171](https://github.com/kazukazu123123/pcsc-rs/issues/171)) ([ef5d476](https://github.com/kazukazu123123/pcsc-rs/commit/ef5d47689e0b5f0df3a2410fbb4ac3f093f6d1b6))
* **deps:** update rust crate sysinfo to v0.29.8 ([#190](https://github.com/kazukazu123123/pcsc-rs/issues/190)) ([86bc65d](https://github.com/kazukazu123123/pcsc-rs/commit/86bc65d037ae23789d1e35d1b99932ad407e2051))
* **deps:** update rust crate sysinfo to v0.29.9 ([#206](https://github.com/kazukazu123123/pcsc-rs/issues/206)) ([11cf0c3](https://github.com/kazukazu123123/pcsc-rs/commit/11cf0c3557863d346a7ad067f2fda9e85650f8f8))
* **deps:** update serde monorepo to v1.0.188 ([#215](https://github.com/kazukazu123123/pcsc-rs/issues/215)) ([676e146](https://github.com/kazukazu123123/pcsc-rs/commit/676e14611a7cacea5a5c242b0041b0ead6b2c3c9))
