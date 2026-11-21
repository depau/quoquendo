# Mirrored files

Every file the voice mirror serves under `https://quoquendo.depau.eu/v1/`, as its [`catalog.json`](https://quoquendo.depau.eu/v1/catalog.json) lists them: 48 voices and 15 language folders unpacked from Loquendo 7's installers and language distributions (`quoquendo --unpack`), 564 files, 2.2 GB in all. The mirror workflow writes this page (`quoquendo --mirror-table`); don't edit it by hand.

Each path is relative to an LTTS7 folder: a voice needs its own files and its language folder's, placed as the paths say (`curl --create-dirs -o "LTTS7/<path>" <link>`). The installers they came from are linked from each voice.

## Voices

| Voice | Language | Installer | Files | Size |
|---|---|---|---:|---:|
| [Afroditi](#afroditi) | [Greek](#greek) | [Afroditi - Greek.exe](https://archive.org/download/text-to-speech-oddcast-voices-pack-2024/Loquendo%207/Afroditi%20-%20Greek.exe) | 10 | 33.8 MB |
| [Alan](#alan) | [EnglishAu](#english) | [Alan - Australian English.exe](https://archive.org/download/text-to-speech-oddcast-voices-pack-2024/Loquendo%207/Alan%20-%20Australian%20English.exe) | 9 | 58.1 MB |
| [Allison](#allison) | [EnglishUs](#english) | [Allison - US English.exe](https://archive.org/download/text-to-speech-oddcast-voices-pack-2024/Loquendo%207/Allison%20-%20US%20English.exe) | 10 | 40.6 MB |
| [Amalia](#amalia) | [PortuguesePt](#portuguese) | [Amalia - European Portuguese.exe](https://archive.org/download/text-to-speech-oddcast-voices-pack-2024/Loquendo%207/Amalia%20-%20European%20Portuguese.exe) | 10 | 51.6 MB |
| [Annika](#annika) | [Swedish](#swedish) | [Annika - Swedish.exe](https://archive.org/download/text-to-speech-oddcast-voices-pack-2024/Loquendo%207/Annika%20-%20Swedish.exe) | 10 | 30.3 MB |
| [Bernard](#bernard) | [French](#french) | [Bernard - French.exe](https://archive.org/download/text-to-speech-oddcast-voices-pack-2024/Loquendo%207/Bernard%20-%20French.exe) | 10 | 37.8 MB |
| [Carlos](#carlos) | [SpanishAm](#spanish) | [Carlos - Colombia Spanish.exe](https://archive.org/download/text-to-speech-oddcast-voices-pack-2024/Loquendo%207/Carlos%20-%20Colombia%20Spanish.exe) | 10 | 41.2 MB |
| [Carmela](#carmela) | [Galician](#galician) | [Carmela - Galician.exe](https://archive.org/download/text-to-speech-oddcast-voices-pack-2024/Loquendo%207/Carmela%20-%20Galician.exe) | 9 | 42.8 MB |
| [Carmen](#carmen) | [SpanishEs](#spanish) | [Carmen - European Spanish.exe](https://archive.org/download/text-to-speech-oddcast-voices-pack-2024/Loquendo%207/Carmen%20-%20European%20Spanish.exe) | 10 | 35.1 MB |
| [Dave](#dave) | [EnglishUs](#english) | [Dave - US English.exe](https://archive.org/download/text-to-speech-oddcast-voices-pack-2024/Loquendo%207/Dave%20-%20US%20English.exe) | 10 | 50.1 MB |
| [Diego](#diego) | [SpanishAr](#spanish) | [Diego - Argentine Spanish.exe](https://archive.org/download/text-to-speech-oddcast-voices-pack-2024/Loquendo%207/Diego%20-%20Argentine%20Spanish.exe) | 5 | 29.2 MB |
| [Dmitri](#dmitri) | [Russian](#russian) | [Dmitri - Russian.exe](https://archive.org/download/text-to-speech-oddcast-voices-pack-2024/Loquendo%207/Dmitri%20-%20Russian.exe) | 9 | 50.7 MB |
| [Elizabeth](#elizabeth) | [EnglishGb](#english) | [Elizabeth - UK English.exe](https://archive.org/download/text-to-speech-oddcast-voices-pack-2024/Loquendo%207/Elizabeth%20-%20UK%20English.exe) | 10 | 40.6 MB |
| [Empar](#empar) | [Valencian](#catalan) | [Empar - Valencian Catalan.exe](https://archive.org/download/text-to-speech-oddcast-voices-pack-2024/Loquendo%207/Empar%20-%20Valencian%20Catalan.exe) | 9 | 63.6 MB |
| [Esperanza](#esperanza) | [SpanishMx](#spanish) | [Esperanza - Mexican Spanish.exe](https://archive.org/download/text-to-speech-oddcast-voices-pack-2024/Loquendo%207/Esperanza%20-%20Mexican%20Spanish.exe) | 5 | 34.9 MB |
| [Eusebio](#eusebio) | [PortuguesePt](#portuguese) | [Eusebio - European Portuguese.exe](https://archive.org/download/text-to-speech-oddcast-voices-pack-2024/Loquendo%207/Eusebio%20-%20European%20Portuguese.exe) | 10 | 50.7 MB |
| [Federica](#federica) | [Italian](#italian) | [Federica - Italian.exe](https://archive.org/download/loquendo-7-federica-italian/Federica%20-%20Italian.exe) | 11 | 93.2 MB |
| [Felipe](#felipe) | [PortugueseBr](#portuguese) | [Felipe - Brazilian Portuguese.EXE](https://archive.org/download/text-to-speech-oddcast-voices-pack-2024/Loquendo%207/Felipe%20-%20Brazilian%20Portuguese.EXE) | 10 | 50.6 MB |
| [Fernanda](#fernanda) | [PortugueseBr](#portuguese) | [Fernanda - Brazilian Portuguese.exe](https://archive.org/download/text-to-speech-oddcast-voices-pack-2024/Loquendo%207/Fernanda%20-%20Brazilian%20Portuguese.exe) | 10 | 51.8 MB |
| [Florence](#florence) | [French](#french) | [Florence - French.exe](https://archive.org/download/text-to-speech-oddcast-voices-pack-2024/Loquendo%207/Florence%20-%20French.exe) | 9 | 50.7 MB |
| [Francisca](#francisca) | [SpanishCl](#spanish) | [Francisca - Chilean Spanish.exe](https://archive.org/download/text-to-speech-oddcast-voices-pack-2024/Loquendo%207/Francisca%20-%20Chilean%20Spanish.exe) | 5 | 30.7 MB |
| [Frida](#frida) | [Danish](#danish) | [Frida - Danish.exe](https://archive.org/download/text-to-speech-oddcast-voices-pack-2024/Loquendo%207/Frida%20-%20Danish.exe) | 10 | 38.9 MB |
| [Gabriela](#gabriela) | [PortugueseBr](#portuguese) | [Gabriela - Brazilian Portuguese.exe](https://archive.org/download/text-to-speech-oddcast-voices-pack-2024/Loquendo%207/Gabriela%20-%20Brazilian%20Portuguese.exe) | 10 | 47.0 MB |
| [Giulia](#giulia) | [Italian](#italian) | [Giulia - Italian.exe](https://archive.org/download/text-to-speech-oddcast-voices-pack-2024/Loquendo%207/Giulia%20-%20Italian.exe) | 10 | 43.8 MB |
| [Grace](#grace) | [EnglishAu](#english) | [Grace - Australian English.exe](https://archive.org/download/text-to-speech-oddcast-voices-pack-2024/Loquendo%207/Grace%20-%20Australian%20English.exe) | 9 | 64.4 MB |
| [Jordi](#jordi) | [Catalan](#catalan) | [Jordi - Catalan.exe](https://archive.org/download/text-to-speech-oddcast-voices-pack-2024/Loquendo%207/Jordi%20-%20Catalan.exe) | 9 | 48.9 MB |
| [Jorge](#jorge) | [SpanishEs](#spanish) | [Jorge - European Spanish.exe](https://archive.org/download/text-to-speech-oddcast-voices-pack-2024/Loquendo%207/Jorge%20-%20European%20Spanish.exe) | 10 | 31.3 MB |
| [Juliette](#juliette) | [French](#french) | [Jolie - French.exe](https://archive.org/download/text-to-speech-oddcast-voices-pack-2024/Loquendo%207/Jolie%20-%20French.exe) | 10 | 38.7 MB |
| [Kate](#kate) | [EnglishGb](#english) | [Catherine - UK English.exe](https://archive.org/download/text-to-speech-oddcast-voices-pack-2024/Loquendo%207/Catherine%20-%20UK%20English.exe) | 10 | 53.5 MB |
| [Katrin](#katrin) | [German](#german) | [Katrin - German.exe](https://archive.org/download/text-to-speech-oddcast-voices-pack-2024/Loquendo%207/Katrin%20-%20German.exe) | 10 | 52.4 MB |
| [Krzysztof](#krzysztof) | [Polish](#polish) | [Krzysztof - Polish.exe](https://archive.org/download/text-to-speech-oddcast-voices-pack-2024/Loquendo%207/Krzysztof%20-%20Polish.exe) | 10 | 41.1 MB |
| [Leonor](#leonor) | [SpanishEs](#spanish) | [Leonor - European Spanish.exe](https://archive.org/download/text-to-speech-oddcast-voices-pack-2024/Loquendo%207/Leonor%20-%20European%20Spanish.exe) | 10 | 40.6 MB |
| [Linlin](#linlin) | [Chinese](#chinese) | [LinLin - Chinese.exe](https://archive.org/download/text-to-speech-oddcast-voices-pack-2024/Loquendo%207/LinLin%20-%20Chinese.exe) | 5 | 30.4 MB |
| [Luca](#luca) | [Italian](#italian) | [Luca - Italian.exe](https://archive.org/download/text-to-speech-oddcast-voices-pack-2024/Loquendo%207/Luca%20-%20Italian.exe) | 10 | 42.4 MB |
| [Marcello](#marcello) | [Italian](#italian) | [Marcello - Italian.exe](https://archive.org/download/text-to-speech-oddcast-voices-pack-2024/Loquendo%207/Marcello%20-%20Italian.exe) | 5 | 47.7 MB |
| [Matteo](#matteo) | [Italian](#italian) | [Matteo - Italian.exe](https://archive.org/download/text-to-speech-oddcast-voices-pack-2024/Loquendo%207/Matteo%20-%20Italian.exe) | 10 | 41.3 MB |
| [Olga](#olga) | [Russian](#russian) | [Olga - Russian.exe](https://archive.org/download/text-to-speech-oddcast-voices-pack-2024/Loquendo%207/Olga%20-%20Russian.exe) | 9 | 49.1 MB |
| [Paola](#paola) | [Italian](#italian) | [Paola - Italian.exe](https://archive.org/download/text-to-speech-oddcast-voices-pack-2024/Loquendo%207/Paola%20-%20Italian.exe) | 10 | 44.5 MB |
| [Roberto](#roberto) | [Italian](#italian) | [Raffaele - Italian.exe](https://archive.org/download/text-to-speech-oddcast-voices-pack-2024/Loquendo%207/Raffaele%20-%20Italian.exe) | 5 | 36.9 MB |
| [Simon](#simon) | [EnglishGb](#english) | [Simon - UK English.exe](https://archive.org/download/text-to-speech-oddcast-voices-pack-2024/Loquendo%207/Simon%20-%20UK%20English.exe) | 10 | 50.8 MB |
| [Soledad](#soledad) | [SpanishAm](#spanish) | [Soledad - Colombia Spanish.exe](https://archive.org/download/text-to-speech-oddcast-voices-pack-2024/Loquendo%207/Soledad%20-%20Colombia%20Spanish.exe) | 10 | 36.5 MB |
| [Stefan](#stefan) | [German](#german) | [Stefan - German.exe](https://archive.org/download/text-to-speech-oddcast-voices-pack-2024/Loquendo%207/Stefan%20-%20German.exe) | 10 | 48.8 MB |
| [Steven](#steven) | [EnglishUs](#english) | [Steven - US English.exe](https://archive.org/download/text-to-speech-oddcast-voices-pack-2024/Loquendo%207/Steven%20-%20US%20English.exe) | 10 | 39.3 MB |
| [Susan](#susan) | [EnglishUs](#english) | [Susan - US English.exe](https://archive.org/download/text-to-speech-oddcast-voices-pack-2024/Loquendo%207/Susan%20-%20US%20English.exe) | 10 | 51.9 MB |
| [Sven](#sven) | [Swedish](#swedish) | [Sven - Swedish.exe](https://archive.org/download/text-to-speech-oddcast-voices-pack-2024/Loquendo%207/Sven%20-%20Swedish.exe) | 10 | 55.6 MB |
| [Willem](#willem) | [Dutch](#dutch) | [Willem - Dutch.exe](https://archive.org/download/text-to-speech-oddcast-voices-pack-2024/Loquendo%207/Willem%20-%20Dutch.exe) | 9 | 66.7 MB |
| [Ximena](#ximena) | [SpanishAm](#spanish) | [Ximena - Colombia Spanish.exe](https://archive.org/download/text-to-speech-oddcast-voices-pack-2024/Loquendo%207/Ximena%20-%20Colombia%20Spanish.exe) | 9 | 52.1 MB |
| [Zosia](#zosia) | [Polish](#polish) | [Zosia - Polish.exe](https://archive.org/download/text-to-speech-oddcast-voices-pack-2024/Loquendo%207/Zosia%20-%20Polish.exe) | 10 | 44.5 MB |

### Afroditi

| File | Size | SHA-256 |
|---|---:|---|
| [`bin/LTTS7Afroditi.dll`](https://quoquendo.depau.eu/v1/bin/LTTS7Afroditi.dll) | 56 kB | `5f98ecbc14b639a955f80cce4e8402c73a4d0d3ffe87f7bbbf66fe4b8917f45a` |
| [`data/Afroditi.vcf`](https://quoquendo.depau.eu/v1/data/Afroditi.vcf) | 347 B | `f17471d0477a3e28051fc823b09760093c95ec2afd00d59264f9c320ba525830` |
| [`data/voices/Afroditi/Afroditi.32000.LTTS7Codec210.44000.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Afroditi/Afroditi.32000.LTTS7Codec210.44000.10.bin) | 29.6 MB | `7213865ba7f2e0339bbd2ccc59dfd826422a3ec60b12237abe86042e16af6fd4` |
| [`data/voices/Afroditi/Afroditi.sde`](https://quoquendo.depau.eu/v1/data/voices/Afroditi/Afroditi.sde) | 1.4 MB | `764804ddad3f41e31903da78d47788d4fc584cb22cdd4f5af3b67f27755913d6` |
| [`data/voices/Afroditi/Afroditi.vde`](https://quoquendo.depau.eu/v1/data/voices/Afroditi/Afroditi.vde) | 473 B | `455a7cdeabad91b3e0fbaa59a8b06c20c3a1dd9f4a4f8fedbd47d3a61840e91c` |
| [`data/voices/Afroditi/AfroditiGilded.gde`](https://quoquendo.depau.eu/v1/data/voices/Afroditi/AfroditiGilded.gde) | 13 kB | `850c8f8859f81506b8e15467a833f549dfb311fe9cbd5b678858c4301b955dda` |
| [`data/voices/Afroditi/AfroditiGildedParalinguistics.32000.LTTS7Codec210.44000.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Afroditi/AfroditiGildedParalinguistics.32000.LTTS7Codec210.44000.10.bin) | 572 kB | `f847c631697819b7cd03199213a3ecb8267db63cb26406939a355e83c4509603` |
| [`data/voices/Afroditi/AfroditiGildedParalinguistics.sde`](https://quoquendo.depau.eu/v1/data/voices/Afroditi/AfroditiGildedParalinguistics.sde) | 4 kB | `6698df27ae37b895faa5cadd14897dbe60609bba2322837337ff085c5a284764` |
| [`data/voices/Afroditi/AfroditiGildedPhrases.32000.LTTS7Codec210.44000.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Afroditi/AfroditiGildedPhrases.32000.LTTS7Codec210.44000.10.bin) | 2.1 MB | `d9f6b44608b83c60235d62b72405efbbc3763956afe474975be077c5e52df5c0` |
| [`data/voices/Afroditi/AfroditiGildedPhrases.sde`](https://quoquendo.depau.eu/v1/data/voices/Afroditi/AfroditiGildedPhrases.sde) | 119 kB | `950722787e95fd351102609e1ebefcd42fbab0ee4b935e97746da0f40dc1da46` |

### Alan

| File | Size | SHA-256 |
|---|---:|---|
| [`bin/LTTS7Alan.dll`](https://quoquendo.depau.eu/v1/bin/LTTS7Alan.dll) | 57 kB | `fe67ca587a1d55b476fcfde423e7417431f069ef63bcf7b1840a8d07203c1832` |
| [`data/Alan.vcf`](https://quoquendo.depau.eu/v1/data/Alan.vcf) | 272 B | `266397f587fd2646ff5c1d72b61fad9cba0df23d13b59289f67726cecc3d32ef` |
| [`data/voices/Alan/Alan.44000.LTTS7CodecMsx.58300.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Alan/Alan.44000.LTTS7CodecMsx.58300.10.bin) | 50.9 MB | `b44f0942536f2f1dcba0854ecc9ae23b94469edbe2e961ab47a00f2b243213fc` |
| [`data/voices/Alan/Alan.sde`](https://quoquendo.depau.eu/v1/data/voices/Alan/Alan.sde) | 1.8 MB | `3d7532f39c4a0ccdac2d9f333ffee8c2d44528f4695cc13e3be0c0bf4e619c98` |
| [`data/voices/Alan/Alan.vde`](https://quoquendo.depau.eu/v1/data/voices/Alan/Alan.vde) | 465 B | `3de7bcbb7f6f2a0990fd89ce5902ec4eaaf3cdfa02cce957b569bc6b76d08c69` |
| [`data/voices/Alan/AlanGildedParalinguistics.44000.LTTS7CodecMsx.58300.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Alan/AlanGildedParalinguistics.44000.LTTS7CodecMsx.58300.10.bin) | 1.7 MB | `b1d4e0d0206ff623d126a6ca28a82b8f4d83c6e1f52aea4711544e7451f80c49` |
| [`data/voices/Alan/AlanGildedParalinguistics.sde`](https://quoquendo.depau.eu/v1/data/voices/Alan/AlanGildedParalinguistics.sde) | 8 kB | `3238b7c7a6df84bbec83b24ae74cfd82123b35452770b3726be26eb1d73453ce` |
| [`data/voices/Alan/AlanGildedPhrases.44000.LTTS7CodecMsx.58300.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Alan/AlanGildedPhrases.44000.LTTS7CodecMsx.58300.10.bin) | 3.5 MB | `0442242b8c9a1de57c6991292f425b7066d7017b57f70c3391d67afe2b90380c` |
| [`data/voices/Alan/AlanGildedPhrases.sde`](https://quoquendo.depau.eu/v1/data/voices/Alan/AlanGildedPhrases.sde) | 126 kB | `0f5270e110a20d5b1629fdacd1cbd566f4bd97040b06d1afe2e4b7661f9e6d8b` |

### Allison

| File | Size | SHA-256 |
|---|---:|---|
| [`bin/LTTS7Allison.dll`](https://quoquendo.depau.eu/v1/bin/LTTS7Allison.dll) | 100 kB | `bb9c78d59605ae173d077fe9256282465f94e7b3a7ae01da6b814a5ff1c4663e` |
| [`data/Allison.vcf`](https://quoquendo.depau.eu/v1/data/Allison.vcf) | 416 B | `c2731ad0c91127a9619a4896d223b1e9e820638c985ccd5bd8f9f0d03fc945f9` |
| [`data/voices/Allison/Allison.32000.LTTS7Codec210.44000.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Allison/Allison.32000.LTTS7Codec210.44000.10.bin) | 35.1 MB | `2aee801142d5f1a6ff64e96370a882b9fdccf9094f416265816659a6596b1c65` |
| [`data/voices/Allison/Allison.sde`](https://quoquendo.depau.eu/v1/data/voices/Allison/Allison.sde) | 1.6 MB | `fc14f736082672ac74d602d5a68e826d6a9a55c3c03b904cd1e7fc0b88af973e` |
| [`data/voices/Allison/Allison.vde`](https://quoquendo.depau.eu/v1/data/voices/Allison/Allison.vde) | 524 B | `c0e2d1ac07aa2fabb947f19a6964b8f2d74579e5708b3da180536d1f3e093686` |
| [`data/voices/Allison/AllisonGilded.gde`](https://quoquendo.depau.eu/v1/data/voices/Allison/AllisonGilded.gde) | 11 kB | `111198235fd228f1e7bd000cf0e166ca3da98784241af647f009e99272bd18fe` |
| [`data/voices/Allison/AllisonGildedParalinguistics.32000.LTTS7Codec210.44000.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Allison/AllisonGildedParalinguistics.32000.LTTS7Codec210.44000.10.bin) | 882 kB | `73a44914ec4f907729a978425cff6d175e33dea5486b499a0aa47a09064ec2ae` |
| [`data/voices/Allison/AllisonGildedParalinguistics.sde`](https://quoquendo.depau.eu/v1/data/voices/Allison/AllisonGildedParalinguistics.sde) | 6 kB | `5ac47a21b882d82aaeac6f1e4c2ad278245bc22af89382a15fa351be135d1d36` |
| [`data/voices/Allison/AllisonGildedPhrases.32000.LTTS7Codec210.44000.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Allison/AllisonGildedPhrases.32000.LTTS7Codec210.44000.10.bin) | 2.8 MB | `a8ea85e8c0eeeb0e0be40be7d1dd05c66cb845405565ce1e6d3b404eaa06a282` |
| [`data/voices/Allison/AllisonGildedPhrases.sde`](https://quoquendo.depau.eu/v1/data/voices/Allison/AllisonGildedPhrases.sde) | 134 kB | `714b241794f205f85ed0f540d52bca393c541716cf05a7c21a7fff6b95790fdb` |

### Amalia

| File | Size | SHA-256 |
|---|---:|---|
| [`bin/LTTS7Amalia.dll`](https://quoquendo.depau.eu/v1/bin/LTTS7Amalia.dll) | 56 kB | `8d43251529ac87fdd4ff9fb852192c51ea73c45ce6b4bf80488430ef84881535` |
| [`data/Amalia.vcf`](https://quoquendo.depau.eu/v1/data/Amalia.vcf) | 342 B | `9b35e6d40ac77039ca31b26a97bc10f62291c254316091dc486f986ceafc5aa9` |
| [`data/voices/Amalia/Amalia.32000.LTTS7Codec210.44000.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Amalia/Amalia.32000.LTTS7Codec210.44000.10.bin) | 47.7 MB | `8a12e103a8431dbe2675745ea164f6438cad0dc1814646669838a7e9912290d8` |
| [`data/voices/Amalia/Amalia.sde`](https://quoquendo.depau.eu/v1/data/voices/Amalia/Amalia.sde) | 2.0 MB | `1da77d0e300a90ad126da6b14fca09261b0658d28218443435ee9cfd16a6b5a3` |
| [`data/voices/Amalia/Amalia.vde`](https://quoquendo.depau.eu/v1/data/voices/Amalia/Amalia.vde) | 506 B | `b40c1a86f7b6f30f7837a5581cfe2af525c5b09228fa24b8f0825e6eccc9835e` |
| [`data/voices/Amalia/AmaliaGilded.gde`](https://quoquendo.depau.eu/v1/data/voices/Amalia/AmaliaGilded.gde) | 5 kB | `c399206cc8fcec4029f4d0402717fa05b5105dc690b7fceae2e4473a037c8761` |
| [`data/voices/Amalia/AmaliaGildedParalinguistics.32000.LTTS7Codec210.44000.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Amalia/AmaliaGildedParalinguistics.32000.LTTS7Codec210.44000.10.bin) | 648 kB | `9f37a2559660a2d4c5371f0228e05e69c02972168fca17efae0ada65955f1eb3` |
| [`data/voices/Amalia/AmaliaGildedParalinguistics.sde`](https://quoquendo.depau.eu/v1/data/voices/Amalia/AmaliaGildedParalinguistics.sde) | 4 kB | `7ccd5f7f933285510be3fbaac5d0c00da327d92aa5ebdc37986136da69ba1e23` |
| [`data/voices/Amalia/AmaliaGildedPhrases.32000.LTTS7Codec210.44000.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Amalia/AmaliaGildedPhrases.32000.LTTS7Codec210.44000.10.bin) | 1.1 MB | `8cdda1019fd5aaea5a6a5ac68d494657f03a4f6dc8f000526a9ffc646e475070` |
| [`data/voices/Amalia/AmaliaGildedPhrases.sde`](https://quoquendo.depau.eu/v1/data/voices/Amalia/AmaliaGildedPhrases.sde) | 56 kB | `d4794c499bea07d257b5763c9f003a7f9071246172226fd6a1b27ac2797c94b8` |

### Annika

| File | Size | SHA-256 |
|---|---:|---|
| [`bin/LTTS7Annika.dll`](https://quoquendo.depau.eu/v1/bin/LTTS7Annika.dll) | 60 kB | `468aa991ce5f9756ad7ec2cc37437b4bfc4a770d277ef2113f41fab5ce5e2ad2` |
| [`data/Annika.vcf`](https://quoquendo.depau.eu/v1/data/Annika.vcf) | 402 B | `99e8be5586358a02a1cd805b23eac3dee2d444483f063c28cfc5f409c747e97a` |
| [`data/voices/Annika/Annika.32000.LTTS7Codec210.44000.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Annika/Annika.32000.LTTS7Codec210.44000.10.bin) | 26.2 MB | `712402c162a02dac47e4bfd26706eaa92d91d813e81a82c6819f473d256efc89` |
| [`data/voices/Annika/Annika.sde`](https://quoquendo.depau.eu/v1/data/voices/Annika/Annika.sde) | 1.1 MB | `0c4ae3d6c76a1d46e171bf28a1bf42c0f2e02df5593ab8d245a39f61491d96f2` |
| [`data/voices/Annika/Annika.vde`](https://quoquendo.depau.eu/v1/data/voices/Annika/Annika.vde) | 595 B | `4fa8c27cdbb8b3f0fe6d3110656c2f8f35d39b79ed4d1335c18467df67465376` |
| [`data/voices/Annika/AnnikaGilded.gde`](https://quoquendo.depau.eu/v1/data/voices/Annika/AnnikaGilded.gde) | 8 kB | `a282c7c6b28a318865f1e00cc7a3522e2cb670ae0b67e17c5ba246c2445dbc5e` |
| [`data/voices/Annika/AnnikaGildedParalinguistics.32000.LTTS7Codec210.44000.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Annika/AnnikaGildedParalinguistics.32000.LTTS7Codec210.44000.10.bin) | 1.2 MB | `9f6afa0a9f29aa111dfc4e751e3f3b30a123e1a786f0ab23f5c887ec8c3c75b9` |
| [`data/voices/Annika/AnnikaGildedParalinguistics.sde`](https://quoquendo.depau.eu/v1/data/voices/Annika/AnnikaGildedParalinguistics.sde) | 8 kB | `69506d561631f66c92ee34e0f6098bd85dd1c0da610d1a92eb0c349b9dfd5a40` |
| [`data/voices/Annika/AnnikaGildedPhrases.32000.LTTS7Codec210.44000.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Annika/AnnikaGildedPhrases.32000.LTTS7Codec210.44000.10.bin) | 1.7 MB | `0ea2eeadf2400acac185d66c3c4dd07e4647cfd7b293fb0d8bf692eda74ea045` |
| [`data/voices/Annika/AnnikaGildedPhrases.sde`](https://quoquendo.depau.eu/v1/data/voices/Annika/AnnikaGildedPhrases.sde) | 82 kB | `2a4803b0a520182f87c90bce48de79296e67e69e2f432aea0a61918131a4f4c9` |

### Bernard

| File | Size | SHA-256 |
|---|---:|---|
| [`bin/LTTS7Bernard.dll`](https://quoquendo.depau.eu/v1/bin/LTTS7Bernard.dll) | 56 kB | `7ba0a79b4dda88f4510f8272d8aaaa1f8dd848e08249e2452407b0e234dcc7b6` |
| [`data/Bernard.vcf`](https://quoquendo.depau.eu/v1/data/Bernard.vcf) | 354 B | `f19463554413fcf412e5a9464d918dac9db3378139dc40baea40fced9ad6d97c` |
| [`data/voices/Bernard/Bernard.32000.LTTS7Codec210.44000.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Bernard/Bernard.32000.LTTS7Codec210.44000.10.bin) | 34.2 MB | `f3be5ae8bf12b0a06ba5df12e1b0033c00d34c3a2bd60d8294fb25d1fbb5b930` |
| [`data/voices/Bernard/Bernard.sde`](https://quoquendo.depau.eu/v1/data/voices/Bernard/Bernard.sde) | 1.5 MB | `85e9309a0310d0f43ab3402ef62fb0a78e4d57853e28c3061d6aadd1c8de5daf` |
| [`data/voices/Bernard/Bernard.vde`](https://quoquendo.depau.eu/v1/data/voices/Bernard/Bernard.vde) | 483 B | `b95056ff32189888d19c247e6c08ce715213bafbc8602e6720c3acf2130c1960` |
| [`data/voices/Bernard/BernardGilded.gde`](https://quoquendo.depau.eu/v1/data/voices/Bernard/BernardGilded.gde) | 8 kB | `e23057b96645e0e2ea9f46dce0c35cbe63a7ba4e7eb0b3eca423c2fe77693798` |
| [`data/voices/Bernard/BernardGildedParalinguistics.32000.LTTS7Codec210.44000.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Bernard/BernardGildedParalinguistics.32000.LTTS7Codec210.44000.10.bin) | 547 kB | `27fe097cf51a6ce98d3359f961d100adffa49429561cfd3de27e91012711e179` |
| [`data/voices/Bernard/BernardGildedParalinguistics.sde`](https://quoquendo.depau.eu/v1/data/voices/Bernard/BernardGildedParalinguistics.sde) | 2 kB | `bab23a1d60c3c09245fa2f31ba7b6f64da0049a1f73c74e8ee5c72c87fade344` |
| [`data/voices/Bernard/BernardGildedPhrases.32000.LTTS7Codec210.44000.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Bernard/BernardGildedPhrases.32000.LTTS7Codec210.44000.10.bin) | 1.4 MB | `f3184f1a503df6d6db261b3ca295559b88a9ec9a622dffc1763c3fd1f9b9b275` |
| [`data/voices/Bernard/BernardGildedPhrases.sde`](https://quoquendo.depau.eu/v1/data/voices/Bernard/BernardGildedPhrases.sde) | 62 kB | `da27c064c571959eb9ade32d2cf7c48b6ce91c73652d31136574866dde2955b1` |

### Carlos

| File | Size | SHA-256 |
|---|---:|---|
| [`bin/LTTS7Carlos.dll`](https://quoquendo.depau.eu/v1/bin/LTTS7Carlos.dll) | 56 kB | `fe1c257b9bd7280a49d1f5a212ae8c6c09c712a314e3320833e2abb4a92dcf61` |
| [`data/Carlos.vcf`](https://quoquendo.depau.eu/v1/data/Carlos.vcf) | 424 B | `e83d25791165b93590ab485e8ed8f706b922c5d3921b44df8a6fde7570433e21` |
| [`data/voices/Carlos/Carlos.32000.LTTS7Codec210.44000.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Carlos/Carlos.32000.LTTS7Codec210.44000.10.bin) | 35.7 MB | `ddc484c42c401673f5996b282e6fbca1c4cc0a0b5f00788a1603c6a6d3872cc4` |
| [`data/voices/Carlos/Carlos.sde`](https://quoquendo.depau.eu/v1/data/voices/Carlos/Carlos.sde) | 1.6 MB | `38b6ce278cfaf8c3a9168f066558435b14d8d33455541d92986d5bd229d0cc4c` |
| [`data/voices/Carlos/Carlos.vde`](https://quoquendo.depau.eu/v1/data/voices/Carlos/Carlos.vde) | 523 B | `b70d75505a7afc9d67e1dcaf20b414b8348bddb07d2c129798b7f291c4a94b72` |
| [`data/voices/Carlos/CarlosGilded.gde`](https://quoquendo.depau.eu/v1/data/voices/Carlos/CarlosGilded.gde) | 10 kB | `6e5118ef09f0de9ffc64bb11cbfae0e656d3aef2bd89b3562ff2d7e9d88066bd` |
| [`data/voices/Carlos/CarlosGildedParalinguistics.32000.LTTS7Codec210.44000.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Carlos/CarlosGildedParalinguistics.32000.LTTS7Codec210.44000.10.bin) | 1.1 MB | `7ca2c2ca33c647a8d133854290d4289766516be67220dcb48bc4b9975414a0f9` |
| [`data/voices/Carlos/CarlosGildedParalinguistics.sde`](https://quoquendo.depau.eu/v1/data/voices/Carlos/CarlosGildedParalinguistics.sde) | 6 kB | `c7c0ab523dd4e5683871abdeea8776118453989548f64446a0a89229c2cf97ac` |
| [`data/voices/Carlos/CarlosGildedPhrases.32000.LTTS7Codec210.44000.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Carlos/CarlosGildedPhrases.32000.LTTS7Codec210.44000.10.bin) | 2.6 MB | `0a91781443e5da7a3813143e913a8bac8c4a4a14ac92336cc1b3dec59d607383` |
| [`data/voices/Carlos/CarlosGildedPhrases.sde`](https://quoquendo.depau.eu/v1/data/voices/Carlos/CarlosGildedPhrases.sde) | 127 kB | `c7b7b81939ab916c4578308f7cb73c482e0f011fd81f6685efb2474f91e2953b` |

### Carmela

| File | Size | SHA-256 |
|---|---:|---|
| [`bin/LTTS7Carmela.dll`](https://quoquendo.depau.eu/v1/bin/LTTS7Carmela.dll) | 46 kB | `4773454e1e6bccbec8e1609c5ed01b394a1aa2559b43f9876a2e45c5034064fa` |
| [`data/Carmela.vcf`](https://quoquendo.depau.eu/v1/data/Carmela.vcf) | 279 B | `222314a594ae332c85ebf17c9a8d9c6082519b6050ccfc9d4234c443ea48d479` |
| [`data/voices/Carmela/Carmela.44000.LTTS7CodecMsx.58300.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Carmela/Carmela.44000.LTTS7CodecMsx.58300.10.bin) | 38.4 MB | `da6332b43f37d10df5feba9cca1bc3f601e6b597a66dd42eb4fe68483165b7c7` |
| [`data/voices/Carmela/Carmela.sde`](https://quoquendo.depau.eu/v1/data/voices/Carmela/Carmela.sde) | 1.4 MB | `d2ed9fa4561be222eff0fb60792e5890d6bcdb12219eae6d314dbf3ba6387e3d` |
| [`data/voices/Carmela/Carmela.vde`](https://quoquendo.depau.eu/v1/data/voices/Carmela/Carmela.vde) | 436 B | `476e93c3b52d9308ecce1fb09196bfcbb747bbc1835732bfa559e0972a927cb1` |
| [`data/voices/Carmela/CarmelaGildedParalinguistics.44000.LTTS7CodecMsx.58300.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Carmela/CarmelaGildedParalinguistics.44000.LTTS7CodecMsx.58300.10.bin) | 1.3 MB | `588e45f0f767a823e0ba44c325446434b79a4a603d92a67d50201842b3a6ad85` |
| [`data/voices/Carmela/CarmelaGildedParalinguistics.sde`](https://quoquendo.depau.eu/v1/data/voices/Carmela/CarmelaGildedParalinguistics.sde) | 5 kB | `8fb1fbc53ee6c9457d8cfabade24e396a36fd71fd2acf576f15acf79e4b410ce` |
| [`data/voices/Carmela/CarmelaGildedPhrases.44000.LTTS7CodecMsx.58300.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Carmela/CarmelaGildedPhrases.44000.LTTS7CodecMsx.58300.10.bin) | 1.7 MB | `b468b7965f81d4e46af014cb1a2f606d265a9557abdf506d9c327c0c4d7aa0b5` |
| [`data/voices/Carmela/CarmelaGildedPhrases.sde`](https://quoquendo.depau.eu/v1/data/voices/Carmela/CarmelaGildedPhrases.sde) | 58 kB | `323ea6e629207662c0fc51c0d402073b0dea8c618e91f101b52501fcf55be3f3` |

### Carmen

| File | Size | SHA-256 |
|---|---:|---|
| [`bin/LTTS7Carmen.dll`](https://quoquendo.depau.eu/v1/bin/LTTS7Carmen.dll) | 56 kB | `11cdeb8da837e80f11ab9b213b34aec4531152432a7a3ee3e4f82fafe47e183b` |
| [`data/Carmen.vcf`](https://quoquendo.depau.eu/v1/data/Carmen.vcf) | 422 B | `6c864063fdabcb456dd18eb1b59c3db65178e2ec33993e6abda5226aafded75e` |
| [`data/voices/Carmen/Carmen.32000.LTTS7Codec210.44000.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Carmen/Carmen.32000.LTTS7Codec210.44000.10.bin) | 31.9 MB | `89f962e21cc7c1a8f3bbbff0b3ba583ec9d21d0a2ed27734feb43205bd4bd206` |
| [`data/voices/Carmen/Carmen.sde`](https://quoquendo.depau.eu/v1/data/voices/Carmen/Carmen.sde) | 1.5 MB | `f432c642511197463cd69b0e8bb936c6708243a602fe652383fd271f809ebc42` |
| [`data/voices/Carmen/Carmen.vde`](https://quoquendo.depau.eu/v1/data/voices/Carmen/Carmen.vde) | 470 B | `d614be3769d830bb3bce9f2c668480f7c2bac83ecea75b6650ee35e363ab54a0` |
| [`data/voices/Carmen/CarmenGilded.gde`](https://quoquendo.depau.eu/v1/data/voices/Carmen/CarmenGilded.gde) | 5 kB | `e67cac0c5d4c0205761c2b897667905f14f117fc0c1e664715d88ff513a2cbaf` |
| [`data/voices/Carmen/CarmenGildedParalinguistics.32000.LTTS7Codec210.44000.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Carmen/CarmenGildedParalinguistics.32000.LTTS7Codec210.44000.10.bin) | 604 kB | `3f27150f4e85e7117e41be57c821d2cec40a47018b30d515dae13d6f5039f68a` |
| [`data/voices/Carmen/CarmenGildedParalinguistics.sde`](https://quoquendo.depau.eu/v1/data/voices/Carmen/CarmenGildedParalinguistics.sde) | 2 kB | `ad59a6589c92f1cc4214efb075233bb314a1c58d44197ac32fcfc8c22443a599` |
| [`data/voices/Carmen/CarmenGildedPhrases.32000.LTTS7Codec210.44000.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Carmen/CarmenGildedPhrases.32000.LTTS7Codec210.44000.10.bin) | 1.0 MB | `6662d5e27865ff39abbcbfffd9a34ec9e77645ff4a617252740404b0f1b10575` |
| [`data/voices/Carmen/CarmenGildedPhrases.sde`](https://quoquendo.depau.eu/v1/data/voices/Carmen/CarmenGildedPhrases.sde) | 53 kB | `1854efd8bfd8365377a593c68ab3be90d41b41bc6f934720c4f97df291fda052` |

### Dave

| File | Size | SHA-256 |
|---|---:|---|
| [`bin/LTTS7Dave.dll`](https://quoquendo.depau.eu/v1/bin/LTTS7Dave.dll) | 72 kB | `aac5575480ad55e6d760e1057c349405b6b8c313b024eea1694dd790b1508898` |
| [`data/Dave.vcf`](https://quoquendo.depau.eu/v1/data/Dave.vcf) | 409 B | `d30891c2e5285361d0fa98a3578b5e3fb9ff69abebd5e2b64a5b1ac11878ae04` |
| [`data/voices/Dave/Dave.32000.LTTS7Codec210.44000.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Dave/Dave.32000.LTTS7Codec210.44000.10.bin) | 44.4 MB | `3076db114d44c5d0e72b9e7bd71411eac693d3751897598e31b008d115357d94` |
| [`data/voices/Dave/Dave.sde`](https://quoquendo.depau.eu/v1/data/voices/Dave/Dave.sde) | 2.1 MB | `fbb35d349a069b8db995b092eb53819ef21c7237bb25d68709cf4256f6c31df3` |
| [`data/voices/Dave/Dave.vde`](https://quoquendo.depau.eu/v1/data/voices/Dave/Dave.vde) | 532 B | `3f8a792a6cb7069efcfb93e87d9bafb5655608106150540b3d55d5076a5a0cd8` |
| [`data/voices/Dave/DaveGilded.gde`](https://quoquendo.depau.eu/v1/data/voices/Dave/DaveGilded.gde) | 11 kB | `8c37f8368f324769f99985e777720639edc58a806f77310367b2797a242a2ea8` |
| [`data/voices/Dave/DaveGildedParalinguistics.32000.LTTS7Codec210.44000.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Dave/DaveGildedParalinguistics.32000.LTTS7Codec210.44000.10.bin) | 587 kB | `67ca8245e0ef1b8696e032099c6df0018e4cbe11e198b22a19181297e9d9099b` |
| [`data/voices/Dave/DaveGildedParalinguistics.sde`](https://quoquendo.depau.eu/v1/data/voices/Dave/DaveGildedParalinguistics.sde) | 5 kB | `d09dc7bc4cb7ec59b500693fe01cef6cc9c037e490bee1e56253b2e38c730b46` |
| [`data/voices/Dave/DaveGildedPhrases.32000.LTTS7Codec210.44000.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Dave/DaveGildedPhrases.32000.LTTS7Codec210.44000.10.bin) | 2.7 MB | `3dbf91ca0bfdb39a51e356d8c7affc81bc1c5dbed4ec7357d5f9a1e9022a4e39` |
| [`data/voices/Dave/DaveGildedPhrases.sde`](https://quoquendo.depau.eu/v1/data/voices/Dave/DaveGildedPhrases.sde) | 126 kB | `d60fdc712260974870d1de57b6dcbf3daffb4f51fa0a10c5ebe46847fdbb9af5` |

### Diego

| File | Size | SHA-256 |
|---|---:|---|
| [`bin/LTTS7Diego.dll`](https://quoquendo.depau.eu/v1/bin/LTTS7Diego.dll) | 56 kB | `b65f8a5b453e7d267aad279a05be761fe2f911f8a965bf66f8dd0cde373c8abc` |
| [`data/Diego.vcf`](https://quoquendo.depau.eu/v1/data/Diego.vcf) | 333 B | `fa07e09b54ae748b0df98fbb8367876a8307aaa908a8c654237d3d0bc2243bc7` |
| [`data/voices/Diego/Diego.32000.LTTS7Codec210.44000.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Diego/Diego.32000.LTTS7Codec210.44000.10.bin) | 27.7 MB | `04086ed02edb09630c635efcee18e4a261450370938710b73476ece18084a9c2` |
| [`data/voices/Diego/Diego.sde`](https://quoquendo.depau.eu/v1/data/voices/Diego/Diego.sde) | 1.5 MB | `5bef4166f747dba9706b645641bd206fb42682b9ed61ec46610dd40aa4221138` |
| [`data/voices/Diego/Diego.vde`](https://quoquendo.depau.eu/v1/data/voices/Diego/Diego.vde) | 430 B | `700d761e25ef4097c92dc332b44343ed9e98d50892e52915911c86e19b243818` |

### Dmitri

| File | Size | SHA-256 |
|---|---:|---|
| [`bin/LTTS7Dmitri.dll`](https://quoquendo.depau.eu/v1/bin/LTTS7Dmitri.dll) | 46 kB | `8b8af6b2bd923ac193bffb49f437756372d4cd0601f73dda2b8979824de62c13` |
| [`data/Dmitri.vcf`](https://quoquendo.depau.eu/v1/data/Dmitri.vcf) | 271 B | `e70a03a8551945fb2005a8f2d725d8778411e50f79112d71acc6fe9a4a9129d0` |
| [`data/voices/Dmitri/Dmitri.44000.LTTS7CodecMsx.58300.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Dmitri/Dmitri.44000.LTTS7CodecMsx.58300.10.bin) | 44.2 MB | `45fc99ee59f8aebc2ece2e123cea1e2b8530c5f04eacc28656a413802710fbe7` |
| [`data/voices/Dmitri/Dmitri.sde`](https://quoquendo.depau.eu/v1/data/voices/Dmitri/Dmitri.sde) | 1.7 MB | `5f267ff22f99942b6034ee873150356b5ef3128338d34ea25dfbcade2d88938a` |
| [`data/voices/Dmitri/Dmitri.vde`](https://quoquendo.depau.eu/v1/data/voices/Dmitri/Dmitri.vde) | 528 B | `d32ad4f0452444078149ba4141f137e79d76a8a6bc647b0502b04a02492b8c82` |
| [`data/voices/Dmitri/DmitriGildedParalinguistics.44000.LTTS7CodecMsx.58300.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Dmitri/DmitriGildedParalinguistics.44000.LTTS7CodecMsx.58300.10.bin) | 1.6 MB | `02ae7c6883838c837f2194877d7ed8ccb00656399c56d76ce1eee623fe9c030c` |
| [`data/voices/Dmitri/DmitriGildedParalinguistics.sde`](https://quoquendo.depau.eu/v1/data/voices/Dmitri/DmitriGildedParalinguistics.sde) | 7 kB | `a2f3263e9c3bd1f6f599c2b3c69cc8c3a74614f32b5e4881389f044d98c3b378` |
| [`data/voices/Dmitri/DmitriGildedPhrases.44000.LTTS7CodecMsx.58300.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Dmitri/DmitriGildedPhrases.44000.LTTS7CodecMsx.58300.10.bin) | 2.9 MB | `4787556d25c3d4dcdaee097be1026a62ede7ff567b8b52f6df5868fa274b54c4` |
| [`data/voices/Dmitri/DmitriGildedPhrases.sde`](https://quoquendo.depau.eu/v1/data/voices/Dmitri/DmitriGildedPhrases.sde) | 118 kB | `d3bd65a194346d68fb06bed6a8dec9e4dae3d64b41a0ee62d4465429c2ee301d` |

### Elizabeth

| File | Size | SHA-256 |
|---|---:|---|
| [`bin/LTTS7Elizabeth.dll`](https://quoquendo.depau.eu/v1/bin/LTTS7Elizabeth.dll) | 68 kB | `1732e1f3b375c07d9d3e104f6c5fc51d95d68c13aab747da8b6966a5ebc60ab5` |
| [`data/Elizabeth.vcf`](https://quoquendo.depau.eu/v1/data/Elizabeth.vcf) | 436 B | `be6114580c37464123f5836265ecc5f98801e89a2c0960e57b1c3d41f89a6a75` |
| [`data/voices/Elizabeth/Elizabeth.32000.LTTS7Codec210.44000.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Elizabeth/Elizabeth.32000.LTTS7Codec210.44000.10.bin) | 35.4 MB | `9fca6eabca295688a2e9790cc2e8e7535a7e4a6d6fd0ad1f3e6d8d74674be12a` |
| [`data/voices/Elizabeth/Elizabeth.sde`](https://quoquendo.depau.eu/v1/data/voices/Elizabeth/Elizabeth.sde) | 1.6 MB | `ce4ed1f9b340ba034de7a3a5e75a12c1817477c435a7278ff2161408b312395f` |
| [`data/voices/Elizabeth/Elizabeth.vde`](https://quoquendo.depau.eu/v1/data/voices/Elizabeth/Elizabeth.vde) | 633 B | `b996e3eb72e821832fc93421755da673fa127de48b5664bfe95795d8d8186ebb` |
| [`data/voices/Elizabeth/ElizabethGilded.gde`](https://quoquendo.depau.eu/v1/data/voices/Elizabeth/ElizabethGilded.gde) | 12 kB | `6c8f8fbc5b209a25944b56de825538a8252e92f2231bb9f6e559ed4b9268e690` |
| [`data/voices/Elizabeth/ElizabethGildedParalinguistics.32000.LTTS7Codec210.44000.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Elizabeth/ElizabethGildedParalinguistics.32000.LTTS7Codec210.44000.10.bin) | 396 kB | `bf8e87b4cf378891f011aa4ffde41dce85784e2eeeb6c3ae8b98e675d2fcad55` |
| [`data/voices/Elizabeth/ElizabethGildedParalinguistics.sde`](https://quoquendo.depau.eu/v1/data/voices/Elizabeth/ElizabethGildedParalinguistics.sde) | 3 kB | `6dc1368f6474d47c9160012b5bf324e331247feedbd8469ac890e09bbb76dc95` |
| [`data/voices/Elizabeth/ElizabethGildedPhrases.32000.LTTS7Codec210.44000.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Elizabeth/ElizabethGildedPhrases.32000.LTTS7Codec210.44000.10.bin) | 2.9 MB | `9ad9165a296f0f42ac86130353a23ef6e6c106abfeaa8277c25863be136b7018` |
| [`data/voices/Elizabeth/ElizabethGildedPhrases.sde`](https://quoquendo.depau.eu/v1/data/voices/Elizabeth/ElizabethGildedPhrases.sde) | 147 kB | `25fb09e89c57888a9345a568ece2daab683ff4b5d8a326e8300129924f00399c` |

### Empar

| File | Size | SHA-256 |
|---|---:|---|
| [`bin/LTTS7Empar.dll`](https://quoquendo.depau.eu/v1/bin/LTTS7Empar.dll) | 44 kB | `67c14715dcb0326ec4440fa8a765787d5885a98d17be19f36ea914c65311cbd5` |
| [`data/Empar.vcf`](https://quoquendo.depau.eu/v1/data/Empar.vcf) | 270 B | `55d882105fb36cc59b3dfc42816f2ffe94066d150a6d81850e455c7b83a070e6` |
| [`data/voices/Empar/Empar.44000.LTTS7CodecMsx.58300.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Empar/Empar.44000.LTTS7CodecMsx.58300.10.bin) | 58.4 MB | `7f90040fa6cf5e2bdcfb50d7f147772f6b618bed128360971e88bf17c587169c` |
| [`data/voices/Empar/Empar.sde`](https://quoquendo.depau.eu/v1/data/voices/Empar/Empar.sde) | 1.9 MB | `ea3c5c22ccf011397d9c499fc90d31724353d2174081daba3e431d45f191432e` |
| [`data/voices/Empar/Empar.vde`](https://quoquendo.depau.eu/v1/data/voices/Empar/Empar.vde) | 472 B | `aae05e8a81bb7d9fec38b7f30a1a580e12880dd414a9c0d98a9b489472c0eeb9` |
| [`data/voices/Empar/EmparGildedParalinguistics.44000.LTTS7CodecMsx.58300.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Empar/EmparGildedParalinguistics.44000.LTTS7CodecMsx.58300.10.bin) | 1.4 MB | `9b751c0ade9400cb39ff444ace9ce7723e12798911da05ef8676de9f3226710e` |
| [`data/voices/Empar/EmparGildedParalinguistics.sde`](https://quoquendo.depau.eu/v1/data/voices/Empar/EmparGildedParalinguistics.sde) | 5 kB | `c4e5bd8dac93891462ecf9e9a81e141d9450cf2b6d0edabdd787444305a8071d` |
| [`data/voices/Empar/EmparGildedPhrases.44000.LTTS7CodecMsx.58300.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Empar/EmparGildedPhrases.44000.LTTS7CodecMsx.58300.10.bin) | 1.8 MB | `c1faa0386cebab5d50258df5318935aaf3a8bf1218aff169d91893c3af48b217` |
| [`data/voices/Empar/EmparGildedPhrases.sde`](https://quoquendo.depau.eu/v1/data/voices/Empar/EmparGildedPhrases.sde) | 67 kB | `477360283a2d3f51c7ba1d80f33955d2d659b03ba198317f0c6c3a2aea4bb74e` |

### Esperanza

| File | Size | SHA-256 |
|---|---:|---|
| [`bin/LTTS7Esperanza.dll`](https://quoquendo.depau.eu/v1/bin/LTTS7Esperanza.dll) | 56 kB | `d5e5c539e6d89fee156fde41790b20c913f64af51202af625bb3a8082159e550` |
| [`data/Esperanza.vcf`](https://quoquendo.depau.eu/v1/data/Esperanza.vcf) | 326 B | `721a48ed3a7d08fecac336b8c84d001d4d81acd87b535f60156b3f11ebd417a4` |
| [`data/voices/Esperanza/Esperanza.32000.LTTS7Codec210.44000.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Esperanza/Esperanza.32000.LTTS7Codec210.44000.10.bin) | 33.2 MB | `7e81ee9e46f2bf0df2fec8d0d1086f239d4302f2405e1222a3f68dbb8a489c7c` |
| [`data/voices/Esperanza/Esperanza.sde`](https://quoquendo.depau.eu/v1/data/voices/Esperanza/Esperanza.sde) | 1.6 MB | `690f9096ae0eb1329f6fa637ebde6fe4e180157335897a207cb5b5257889e747` |
| [`data/voices/Esperanza/Esperanza.vde`](https://quoquendo.depau.eu/v1/data/voices/Esperanza/Esperanza.vde) | 427 B | `2a7d442e7909de668ac17eaebe6e704ed5821828145d56a0e7e6afa898439fbe` |

### Eusebio

| File | Size | SHA-256 |
|---|---:|---|
| [`bin/LTTS7Eusebio.dll`](https://quoquendo.depau.eu/v1/bin/LTTS7Eusebio.dll) | 56 kB | `45a441df7022a2807884fe349b4d0d036ec9bfc30f8190a91a5701e137e33c0c` |
| [`data/Eusebio.vcf`](https://quoquendo.depau.eu/v1/data/Eusebio.vcf) | 408 B | `eca5ac10dd6bbf51f3436b77903444081865fe1e306e91f7119074c974d5ea51` |
| [`data/voices/Eusebio/Eusebio.32000.LTTS7Codec210.44000.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Eusebio/Eusebio.32000.LTTS7Codec210.44000.10.bin) | 47.0 MB | `e08ffb53fc4765e895cd295d75e6e59f562b9c807ec8446c643392a0be1cdda4` |
| [`data/voices/Eusebio/Eusebio.sde`](https://quoquendo.depau.eu/v1/data/voices/Eusebio/Eusebio.sde) | 2.0 MB | `cc9b7767d04bdae17a2e13f6418ab113c8ad3bec76f9c000416eea40a4ae0d05` |
| [`data/voices/Eusebio/Eusebio.vde`](https://quoquendo.depau.eu/v1/data/voices/Eusebio/Eusebio.vde) | 507 B | `fe5b15c2acca76de5682278064c9675d399cbf7edadadefa9495ada39cbaf67b` |
| [`data/voices/Eusebio/EusebioGilded.gde`](https://quoquendo.depau.eu/v1/data/voices/Eusebio/EusebioGilded.gde) | 4 kB | `6f5ccab47e40932547bfdc760cbbf93eb9f95f8687b4b7ba8e6b73280d040603` |
| [`data/voices/Eusebio/EusebioGildedParalinguistics.32000.LTTS7Codec210.44000.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Eusebio/EusebioGildedParalinguistics.32000.LTTS7Codec210.44000.10.bin) | 620 kB | `ccc9eacff05f23d3d59ae0ad44dae2f4d1a5556894c3d3dc3d2a0ff09dba6983` |
| [`data/voices/Eusebio/EusebioGildedParalinguistics.sde`](https://quoquendo.depau.eu/v1/data/voices/Eusebio/EusebioGildedParalinguistics.sde) | 3 kB | `43cb9bda4c35d9cfe10f6d0bcfd2dcd8c75629100268c64f352f2c80bd85fd9d` |
| [`data/voices/Eusebio/EusebioGildedPhrases.32000.LTTS7Codec210.44000.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Eusebio/EusebioGildedPhrases.32000.LTTS7Codec210.44000.10.bin) | 1.0 MB | `62bd073cc623e0a4ba98b7c1086931bcc6b066572c7c5eb64c86c1dcecd72fbe` |
| [`data/voices/Eusebio/EusebioGildedPhrases.sde`](https://quoquendo.depau.eu/v1/data/voices/Eusebio/EusebioGildedPhrases.sde) | 54 kB | `eca3e9c202a452f8ed8c6b6a2a34ee9df795400f022a8a0dc95ffee4a034f926` |

### Federica

| File | Size | SHA-256 |
|---|---:|---|
| [`bin/LTTS7Federica.dll`](https://quoquendo.depau.eu/v1/bin/LTTS7Federica.dll) | 44 kB | `29601bb58c32808dd904609082a5f290ff0bddd2e14577d5552a825bddc6d3f3` |
| [`data/Federica.vcf`](https://quoquendo.depau.eu/v1/data/Federica.vcf) | 339 B | `5ee1092105551d001803fe0baaf4bba96be7103e775cb325d607815f38c27db8` |
| [`data/voices/Federica/Federica.vde`](https://quoquendo.depau.eu/v1/data/voices/Federica/Federica.vde) | 445 B | `464206d16cb0e978f402a0482f36b8e62b1e94a3f490d4800fc21edc423445fa` |
| [`data/voices/Federica/FedericaGildedParalinguistics.44000.LTTS7CodecMsx.58300.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Federica/FedericaGildedParalinguistics.44000.LTTS7CodecMsx.58300.10.bin) | 2.0 MB | `38273c66df631ef705819d88b78cf0497ea02c6ee47006a95d392f2444236315` |
| [`data/voices/Federica/FedericaGildedParalinguistics.sde`](https://quoquendo.depau.eu/v1/data/voices/Federica/FedericaGildedParalinguistics.sde) | 11 kB | `f2af8d4dda108999e10bb3934773614d517222393c84e060e5911f62f64e980e` |
| [`data/voices/Federica/FedericaGildedPhrases.44000.LTTS7CodecMsx.58300.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Federica/FedericaGildedPhrases.44000.LTTS7CodecMsx.58300.10.bin) | 4.1 MB | `ef4452b9c13c435fb8ec1e0783e6b7122acd0bd6deeec618938b528b324be6ad` |
| [`data/voices/Federica/FedericaGildedPhrases.sde`](https://quoquendo.depau.eu/v1/data/voices/Federica/FedericaGildedPhrases.sde) | 152 kB | `b187d3559fc2a80d977bf024fd617940ab6e3414150158950ddd36d068ac4d84` |
| [`data/voices/Federica/Federica_1.44000.LTTS7CodecMsx.58300.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Federica/Federica_1.44000.LTTS7CodecMsx.58300.10.bin) | 48.2 MB | `51bec7ad5ffe5a0be82ba494eb94a523b049759d8bd4fe496dd85f5fc9f9f09f` |
| [`data/voices/Federica/Federica_1.sde`](https://quoquendo.depau.eu/v1/data/voices/Federica/Federica_1.sde) | 1.7 MB | `f15cb31193f0c537850628102df990babd10b63cb8ff68dd308faa29190fe1dc` |
| [`data/voices/Federica/Federica_2.44000.LTTS7CodecMsx.58300.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Federica/Federica_2.44000.LTTS7CodecMsx.58300.10.bin) | 35.9 MB | `a925dc97d76fadcee59490f8ebd1e7f0973242a90c7ce0b18b68121657e71b0b` |
| [`data/voices/Federica/Federica_2.sde`](https://quoquendo.depau.eu/v1/data/voices/Federica/Federica_2.sde) | 1.2 MB | `4639ea1ef5e6aef5ce1448bee004869349e91d2816c696bf36c903b7e6b06399` |

### Felipe

| File | Size | SHA-256 |
|---|---:|---|
| [`bin/LTTS7Felipe.dll`](https://quoquendo.depau.eu/v1/bin/LTTS7Felipe.dll) | 56 kB | `dc5eb1147bb972874aa04d010d9f54601574230e7a15e29bb59a22a4ad9f46ae` |
| [`data/Felipe.vcf`](https://quoquendo.depau.eu/v1/data/Felipe.vcf) | 402 B | `2cacd9cbfa19f5678628ee15bcf2c05b08d69ecbab7d5e19afae4d67f98ce52c` |
| [`data/voices/Felipe/Felipe.32000.LTTS7Codec210.44000.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Felipe/Felipe.32000.LTTS7Codec210.44000.10.bin) | 46.1 MB | `2f8966fd27b5c1b9bc5d0edac8100cdfcb8c6c2ff5c1a8c06692dc58206f4783` |
| [`data/voices/Felipe/Felipe.sde`](https://quoquendo.depau.eu/v1/data/voices/Felipe/Felipe.sde) | 2.1 MB | `f0f438fdd020ede5eb39c9a62cdb04271e0ae0a44371cba4f3def57bf8cdb1f6` |
| [`data/voices/Felipe/Felipe.vde`](https://quoquendo.depau.eu/v1/data/voices/Felipe/Felipe.vde) | 629 B | `291d1a52c791d880170e9682e50d6cfa9282ebacf4aa30af9514c3bda8ec0c45` |
| [`data/voices/Felipe/FelipeGilded.gde`](https://quoquendo.depau.eu/v1/data/voices/Felipe/FelipeGilded.gde) | 6 kB | `a3c128a44eb8c0992c08ee28d3734c2c4c185823514d16921969d56371f76d78` |
| [`data/voices/Felipe/FelipeGildedParalinguistics.32000.LTTS7Codec210.44000.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Felipe/FelipeGildedParalinguistics.32000.LTTS7Codec210.44000.10.bin) | 1.0 MB | `7e288de6c41eb2faa364e8f089950750fec04535c7890ab43bb2069e83847d37` |
| [`data/voices/Felipe/FelipeGildedParalinguistics.sde`](https://quoquendo.depau.eu/v1/data/voices/Felipe/FelipeGildedParalinguistics.sde) | 6 kB | `b8dda085bffb88cdb15e2ab3ab61ad834a228a27b22a237e44e376841dad89e4` |
| [`data/voices/Felipe/FelipeGildedPhrases.32000.LTTS7Codec210.44000.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Felipe/FelipeGildedPhrases.32000.LTTS7Codec210.44000.10.bin) | 1.2 MB | `a6da28af871966de0b51d75c55d9bbb270cd99d00afa88d6854bb48664968523` |
| [`data/voices/Felipe/FelipeGildedPhrases.sde`](https://quoquendo.depau.eu/v1/data/voices/Felipe/FelipeGildedPhrases.sde) | 60 kB | `307dacc5705e56bad2d1c54a93b985020a36c29a364bdb17d0fb6f4987a37626` |

### Fernanda

| File | Size | SHA-256 |
|---|---:|---|
| [`bin/LTTS7Fernanda.dll`](https://quoquendo.depau.eu/v1/bin/LTTS7Fernanda.dll) | 56 kB | `fb0a5eccf3591f84d55c64ed402f3472ae7bf2ec283c55920154b443a7f2da6f` |
| [`data/Fernanda.vcf`](https://quoquendo.depau.eu/v1/data/Fernanda.vcf) | 414 B | `c3568bd262e477abf319205fe65bf8208bd5c64996a14280cc4f7243c9061d77` |
| [`data/voices/Fernanda/Fernanda.32000.LTTS7Codec210.44000.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Fernanda/Fernanda.32000.LTTS7Codec210.44000.10.bin) | 47.3 MB | `520a4690087c67a358407a40370656e30a4ca7aba7c9e8b2fb5be03926f45f9e` |
| [`data/voices/Fernanda/Fernanda.sde`](https://quoquendo.depau.eu/v1/data/voices/Fernanda/Fernanda.sde) | 2.1 MB | `262a2de9959a34cc78dd63f5190b02ed7327beed444ef1fed90af1065097eb08` |
| [`data/voices/Fernanda/Fernanda.vde`](https://quoquendo.depau.eu/v1/data/voices/Fernanda/Fernanda.vde) | 508 B | `84727f009bd978f2a57b9c1160deab33329bbf078d0b4dec863d6bdc4f6c7337` |
| [`data/voices/Fernanda/FernandaGilded.gde`](https://quoquendo.depau.eu/v1/data/voices/Fernanda/FernandaGilded.gde) | 6 kB | `4ac3c9552f1d52e57f111b83859528f2b8fae3a100ebb14d0ed78d31b768a78f` |
| [`data/voices/Fernanda/FernandaGildedParalinguistics.32000.LTTS7Codec210.44000.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Fernanda/FernandaGildedParalinguistics.32000.LTTS7Codec210.44000.10.bin) | 946 kB | `10642f0952792c79dd74afda05dc970fc442a422080ed4eb617403e35913693f` |
| [`data/voices/Fernanda/FernandaGildedParalinguistics.sde`](https://quoquendo.depau.eu/v1/data/voices/Fernanda/FernandaGildedParalinguistics.sde) | 6 kB | `d0d295d415df09d9144030cca40dcd7bc332ef5fdb4dcdb279862c780890034f` |
| [`data/voices/Fernanda/FernandaGildedPhrases.32000.LTTS7Codec210.44000.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Fernanda/FernandaGildedPhrases.32000.LTTS7Codec210.44000.10.bin) | 1.3 MB | `ff321064da03443a6b7832c3822acdcb159714dc6091bd0fc119de06d5aa9cea` |
| [`data/voices/Fernanda/FernandaGildedPhrases.sde`](https://quoquendo.depau.eu/v1/data/voices/Fernanda/FernandaGildedPhrases.sde) | 61 kB | `dc1557a6318f444133eeea7868e9c40bcbae7a1b7b4280a9052e946243905538` |

### Florence

| File | Size | SHA-256 |
|---|---:|---|
| [`bin/LTTS7Florence.dll`](https://quoquendo.depau.eu/v1/bin/LTTS7Florence.dll) | 48 kB | `fa428005f06dc2d0d34a5c5dabe672bdbe690dfeb1404e343fba13a719f32fbc` |
| [`data/Florence.vcf`](https://quoquendo.depau.eu/v1/data/Florence.vcf) | 282 B | `d3e52846a1fd0ed07f4b4bfcd0827cfd8c5e6b553b73e4c46e1e070ac044c865` |
| [`data/voices/Florence/Florence.44000.LTTS7CodecMsx.58300.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Florence/Florence.44000.LTTS7CodecMsx.58300.10.bin) | 44.8 MB | `6a870ceab96276f4a1fd59c93574788c8a8d6e9c244b3df81c80d1a7006ba995` |
| [`data/voices/Florence/Florence.sde`](https://quoquendo.depau.eu/v1/data/voices/Florence/Florence.sde) | 1.4 MB | `3204894198bacce0813ef40c26f22afcbee0117b457ab1bbb76bc5f36a374de2` |
| [`data/voices/Florence/Florence.vde`](https://quoquendo.depau.eu/v1/data/voices/Florence/Florence.vde) | 444 B | `c0a0803adb419bd37b18800a14b45131065b8b49aebac8ea8eb3da1326b01451` |
| [`data/voices/Florence/FlorenceGildedParalinguistics.44000.LTTS7CodecMsx.58300.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Florence/FlorenceGildedParalinguistics.44000.LTTS7CodecMsx.58300.10.bin) | 1.4 MB | `e56dec944dd5058769ddf2e223317422b86ba4dec5935dc61ae03bb83436b6aa` |
| [`data/voices/Florence/FlorenceGildedParalinguistics.sde`](https://quoquendo.depau.eu/v1/data/voices/Florence/FlorenceGildedParalinguistics.sde) | 6 kB | `34052ac89dd59161800531f3eefb4cbc7d8bdfb1a8a234ff7259509b8099803d` |
| [`data/voices/Florence/FlorenceGildedPhrases.44000.LTTS7CodecMsx.58300.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Florence/FlorenceGildedPhrases.44000.LTTS7CodecMsx.58300.10.bin) | 2.9 MB | `6aa48bf9a556ca75ce449566c383a8dfc3396080c4a64c44463b174dee43af53` |
| [`data/voices/Florence/FlorenceGildedPhrases.sde`](https://quoquendo.depau.eu/v1/data/voices/Florence/FlorenceGildedPhrases.sde) | 101 kB | `42cfc91d71e326f9dfefb8736bf96a62241f8b959da165d3ee34e7dbb14c14d5` |

### Francisca

| File | Size | SHA-256 |
|---|---:|---|
| [`bin/LTTS7Francisca.dll`](https://quoquendo.depau.eu/v1/bin/LTTS7Francisca.dll) | 56 kB | `d269516b225b01fc45c7e126219526d5d158fcc11bfb2f22441e9fb242155e04` |
| [`data/Francisca.vcf`](https://quoquendo.depau.eu/v1/data/Francisca.vcf) | 334 B | `940ed4ac96591d59ef2a0de945755ad5fdf865fdd336514db9ff595ade767623` |
| [`data/voices/Francisca/Francisca.32000.LTTS7Codec210.44000.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Francisca/Francisca.32000.LTTS7Codec210.44000.10.bin) | 29.1 MB | `928da98b0b4612c66520144721d6c22ecdbfa42e1fa96a64431c752fdc69b07e` |
| [`data/voices/Francisca/Francisca.sde`](https://quoquendo.depau.eu/v1/data/voices/Francisca/Francisca.sde) | 1.5 MB | `2d754654db9a9ef08aa66f8f29984d6c031c25ece0150ce89429dd044f373a4b` |
| [`data/voices/Francisca/Francisca.vde`](https://quoquendo.depau.eu/v1/data/voices/Francisca/Francisca.vde) | 426 B | `5f5186562df6a60f8289a1c8d34be64eae7e6ac3ebdd4845d3e5a2ab25e70967` |

### Frida

| File | Size | SHA-256 |
|---|---:|---|
| [`bin/LTTS7Frida.dll`](https://quoquendo.depau.eu/v1/bin/LTTS7Frida.dll) | 60 kB | `361bbf672fbb591d281982ffafc1e8de07d71ad8256c8fdc5989ed20e5859ab6` |
| [`data/Frida.vcf`](https://quoquendo.depau.eu/v1/data/Frida.vcf) | 407 B | `cc0efc7b21424364624120fe82357ef0ea5719c4e2597f0261df4d92bd78f66c` |
| [`data/voices/Frida/Frida.32000.LTTS7Codec210.44000.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Frida/Frida.32000.LTTS7Codec210.44000.10.bin) | 33.7 MB | `a32a0f88313f09422e92818ada783dac7ed7199c544b84a51cf67883a9ef1c7d` |
| [`data/voices/Frida/Frida.sde`](https://quoquendo.depau.eu/v1/data/voices/Frida/Frida.sde) | 2.0 MB | `f5457832c2d4e65cdb6ece0cd48037afdc97fd7bb3d6db1bc1886bd4b3c43568` |
| [`data/voices/Frida/Frida.vde`](https://quoquendo.depau.eu/v1/data/voices/Frida/Frida.vde) | 553 B | `c25bd1326df2d71137ae8ad20223c29ee26c75b7f2b382354670e65947b93fb0` |
| [`data/voices/Frida/FridaGilded.gde`](https://quoquendo.depau.eu/v1/data/voices/Frida/FridaGilded.gde) | 11 kB | `9a07b22c2aaa18a3fd076a805cd1127f0b2a7444b5145267cc73a3ec5bb00b61` |
| [`data/voices/Frida/FridaGildedParalinguistics.32000.LTTS7Codec210.44000.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Frida/FridaGildedParalinguistics.32000.LTTS7Codec210.44000.10.bin) | 1.4 MB | `bdebcf95dcf483b98e2457af1591968d3554dfd94818e63937c78572f09d0f9d` |
| [`data/voices/Frida/FridaGildedParalinguistics.sde`](https://quoquendo.depau.eu/v1/data/voices/Frida/FridaGildedParalinguistics.sde) | 13 kB | `a685f4ae70f3a0a25c4139d4ce17aa4236c728dbbc89449e9b9ef285d5fab0f2` |
| [`data/voices/Frida/FridaGildedPhrases.32000.LTTS7Codec210.44000.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Frida/FridaGildedPhrases.32000.LTTS7Codec210.44000.10.bin) | 1.7 MB | `7bb7812e402bc320a478f359b20310a1e0806a1548c93137e93d8bd131f2d057` |
| [`data/voices/Frida/FridaGildedPhrases.sde`](https://quoquendo.depau.eu/v1/data/voices/Frida/FridaGildedPhrases.sde) | 99 kB | `c27c593d933919c8b0cfd2590d211a9154ca99da299487131bf4a5e83903a553` |

### Gabriela

| File | Size | SHA-256 |
|---|---:|---|
| [`bin/LTTS7Gabriela.dll`](https://quoquendo.depau.eu/v1/bin/LTTS7Gabriela.dll) | 56 kB | `498e861937edf9dd44c34f343ebe3b373560ad2ce63e00fab58082c5ce21f000` |
| [`data/Gabriela.vcf`](https://quoquendo.depau.eu/v1/data/Gabriela.vcf) | 351 B | `7bc3e533b686fbc13612a04cae48c1c4b1de7ed61df6dce9edafa8c1f943f1b6` |
| [`data/voices/Gabriela/Gabriela.32000.LTTS7Codec210.44000.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Gabriela/Gabriela.32000.LTTS7Codec210.44000.10.bin) | 43.1 MB | `029790e194817e948bdb6d13c152a8e6007d737ff72105f5318b70c2220d3e5e` |
| [`data/voices/Gabriela/Gabriela.sde`](https://quoquendo.depau.eu/v1/data/voices/Gabriela/Gabriela.sde) | 1.9 MB | `ad73fe6c86a88165e9fb4f55afa53136d8ffb6d3b6b26b869a76ba70fdaf766e` |
| [`data/voices/Gabriela/Gabriela.vde`](https://quoquendo.depau.eu/v1/data/voices/Gabriela/Gabriela.vde) | 508 B | `0036e5e14a07a544ffcb23b13c104a50d992b1ae918b46f2848559669460e904` |
| [`data/voices/Gabriela/GabrielaGilded.gde`](https://quoquendo.depau.eu/v1/data/voices/Gabriela/GabrielaGilded.gde) | 5 kB | `7348a7e19bb7f8a30fcb2255d2744406295358585d3d1b1f49928dded80dcc1e` |
| [`data/voices/Gabriela/GabrielaGildedParalinguistics.32000.LTTS7Codec210.44000.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Gabriela/GabrielaGildedParalinguistics.32000.LTTS7Codec210.44000.10.bin) | 624 kB | `09f59f997214e2a1286840f0cab258aa105186267d16fae4855e4596812604b4` |
| [`data/voices/Gabriela/GabrielaGildedParalinguistics.sde`](https://quoquendo.depau.eu/v1/data/voices/Gabriela/GabrielaGildedParalinguistics.sde) | 4 kB | `c33f67de126c4d21303e94d04ecfbf1f8c785813d05a8bb216d118c4a6482c88` |
| [`data/voices/Gabriela/GabrielaGildedPhrases.32000.LTTS7Codec210.44000.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Gabriela/GabrielaGildedPhrases.32000.LTTS7Codec210.44000.10.bin) | 1.3 MB | `26557483c013169472db57f930d659989ae7f756c42f5e8bcf689351eb7dc7b8` |
| [`data/voices/Gabriela/GabrielaGildedPhrases.sde`](https://quoquendo.depau.eu/v1/data/voices/Gabriela/GabrielaGildedPhrases.sde) | 58 kB | `5ae1f3aa4e2648b99de5b945fa40be421ef23207f1b1e249d562f894c8cbebe9` |

### Giulia

| File | Size | SHA-256 |
|---|---:|---|
| [`bin/LTTS7Giulia.dll`](https://quoquendo.depau.eu/v1/bin/LTTS7Giulia.dll) | 52 kB | `26725bfcdd4bd4b56650b1df3ea4cf8e9e22ba08fa1985778c9a47b532ece7b4` |
| [`data/Giulia.vcf`](https://quoquendo.depau.eu/v1/data/Giulia.vcf) | 413 B | `8c206b4830c1c272655f834dc8102ac4deaf4e79b075eee8350d1465621cbbe7` |
| [`data/voices/Giulia/Giulia.32000.LTTS7Codec210.44000.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Giulia/Giulia.32000.LTTS7Codec210.44000.10.bin) | 37.8 MB | `85003b7f8e1c28b855fbd8263ecf8df9e3f8f4bcc30d075ea00848cf5ffc4c37` |
| [`data/voices/Giulia/Giulia.sde`](https://quoquendo.depau.eu/v1/data/voices/Giulia/Giulia.sde) | 1.9 MB | `aede29afd55da13d800340a1ac00ec2def496ba29646d221946061e507821283` |
| [`data/voices/Giulia/Giulia.vde`](https://quoquendo.depau.eu/v1/data/voices/Giulia/Giulia.vde) | 467 B | `25b9a87673f3a20e19763e33640d7beff11ebe9509307377d601a377ea969f51` |
| [`data/voices/Giulia/GiuliaGilded.gde`](https://quoquendo.depau.eu/v1/data/voices/Giulia/GiuliaGilded.gde) | 12 kB | `c07dfcd72b6fa58542b4ecc7f73c536b7884a399cdfd46a100dc2de8d265ffc4` |
| [`data/voices/Giulia/GiuliaGildedParalinguistics.32000.LTTS7Codec210.44000.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Giulia/GiuliaGildedParalinguistics.32000.LTTS7Codec210.44000.10.bin) | 1.1 MB | `138aa18f42ba35bc158f024495786ac831735637fa488a3a79bf1e85d5238297` |
| [`data/voices/Giulia/GiuliaGildedParalinguistics.sde`](https://quoquendo.depau.eu/v1/data/voices/Giulia/GiuliaGildedParalinguistics.sde) | 9 kB | `df603746fe774bb444a4ada0868289f2da97cb2045ae350dcf8fd2a511ef06ce` |
| [`data/voices/Giulia/GiuliaGildedPhrases.32000.LTTS7Codec210.44000.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Giulia/GiuliaGildedPhrases.32000.LTTS7Codec210.44000.10.bin) | 2.8 MB | `86150924332d9cf6024cf651d8cf468358431dee9ff2225da45aaeec213f0e13` |
| [`data/voices/Giulia/GiuliaGildedPhrases.sde`](https://quoquendo.depau.eu/v1/data/voices/Giulia/GiuliaGildedPhrases.sde) | 159 kB | `4c6c4f4dcad21275d972dc18aca54405e50caf8d04e607b3ca540b695516e58c` |

### Grace

| File | Size | SHA-256 |
|---|---:|---|
| [`bin/LTTS7Grace.dll`](https://quoquendo.depau.eu/v1/bin/LTTS7Grace.dll) | 58 kB | `f61405c92e3bf798df61d84142b226b862806132aaeb799104c1ded1155b3034` |
| [`data/Grace.vcf`](https://quoquendo.depau.eu/v1/data/Grace.vcf) | 279 B | `7563d7930cdce7f85389a6a7b734babed47f071e150c5c3c449994122817fa3e` |
| [`data/voices/Grace/Grace.44000.LTTS7CodecMsx.58300.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Grace/Grace.44000.LTTS7CodecMsx.58300.10.bin) | 57.2 MB | `99c23e72865d7002ff1013402c8c10ff6528a21d0f99a939f83bed98c62b287c` |
| [`data/voices/Grace/Grace.sde`](https://quoquendo.depau.eu/v1/data/voices/Grace/Grace.sde) | 1.9 MB | `055c3a88407c3290c6089b0d6f7dddf12788377e29c43e5d1823f05dd4b564c0` |
| [`data/voices/Grace/Grace.vde`](https://quoquendo.depau.eu/v1/data/voices/Grace/Grace.vde) | 474 B | `aed826cdaaf0b0c2791589b0d9c9f23674816fc6c9b4cf33e8f56857a2f38d8f` |
| [`data/voices/Grace/GraceGildedParalinguistics.44000.LTTS7CodecMsx.58300.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Grace/GraceGildedParalinguistics.44000.LTTS7CodecMsx.58300.10.bin) | 2.0 MB | `475ccf7809c2e5a4ab2afbb55d0c32dad7206be89c32e1b77b97698ef39ff72c` |
| [`data/voices/Grace/GraceGildedParalinguistics.sde`](https://quoquendo.depau.eu/v1/data/voices/Grace/GraceGildedParalinguistics.sde) | 9 kB | `66119f9d2ecdaafcb3ab04c1ccb1aa03155e44f7845f1851662a5ff83051ded0` |
| [`data/voices/Grace/GraceGildedPhrases.44000.LTTS7CodecMsx.58300.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Grace/GraceGildedPhrases.44000.LTTS7CodecMsx.58300.10.bin) | 3.0 MB | `79713479baff8febc43f7c3236c8ac8d5cd3ac392a56931d6b7a182ae971f641` |
| [`data/voices/Grace/GraceGildedPhrases.sde`](https://quoquendo.depau.eu/v1/data/voices/Grace/GraceGildedPhrases.sde) | 103 kB | `42e367fec4c5ca60c6a60a58b84d71f9a31ae9df0cd22197826ecf7258bc131d` |

### Jordi

| File | Size | SHA-256 |
|---|---:|---|
| [`bin/LTTS7Jordi.dll`](https://quoquendo.depau.eu/v1/bin/LTTS7Jordi.dll) | 44 kB | `360cb606a6b01d6b3d00ced54dc43197d7119bcb6645258d0fc7b0b2a855919d` |
| [`data/Jordi.vcf`](https://quoquendo.depau.eu/v1/data/Jordi.vcf) | 266 B | `61bc774c004afdf21a476bf7b7ac65b640d949a55c4e14abf4d616df573e3a1d` |
| [`data/voices/Jordi/Jordi.44000.LTTS7CodecMsx.58300.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Jordi/Jordi.44000.LTTS7CodecMsx.58300.10.bin) | 44.0 MB | `9de6d7cb34878adfb742949efbc3cbc1fdde350b92e164ada86df45475fe9936` |
| [`data/voices/Jordi/Jordi.sde`](https://quoquendo.depau.eu/v1/data/voices/Jordi/Jordi.sde) | 1.6 MB | `3569d392d610294336a3a0d39543249aed7e39beaef0f973c734ec8e8d44e8d8` |
| [`data/voices/Jordi/Jordi.vde`](https://quoquendo.depau.eu/v1/data/voices/Jordi/Jordi.vde) | 437 B | `2b5fbc89f61a45a8260210db9151eb341cbdb72a022677fdce205dfa9741d6fe` |
| [`data/voices/Jordi/JordiGildedParalinguistics.44000.LTTS7CodecMsx.58300.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Jordi/JordiGildedParalinguistics.44000.LTTS7CodecMsx.58300.10.bin) | 1.7 MB | `1838eb89f2cdd33eb6c0cdac68619379242d2662930ff8bc47c06fcd28911a74` |
| [`data/voices/Jordi/JordiGildedParalinguistics.sde`](https://quoquendo.depau.eu/v1/data/voices/Jordi/JordiGildedParalinguistics.sde) | 7 kB | `5e7c4d692bd22c31848830b08b8f259a117ce80c13e17fe0af4f8966e262bb9e` |
| [`data/voices/Jordi/JordiGildedPhrases.44000.LTTS7CodecMsx.58300.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Jordi/JordiGildedPhrases.44000.LTTS7CodecMsx.58300.10.bin) | 1.5 MB | `8fc4f87f5ec3d6b5a363ea25e45206881abffbdc4f22b451cfdc24caedccdce1` |
| [`data/voices/Jordi/JordiGildedPhrases.sde`](https://quoquendo.depau.eu/v1/data/voices/Jordi/JordiGildedPhrases.sde) | 61 kB | `20980e2e61698b82b31a41ca34b62b5d003b5868cdf75ae9e2d863b366e8845a` |

### Jorge

| File | Size | SHA-256 |
|---|---:|---|
| [`bin/LTTS7Jorge.dll`](https://quoquendo.depau.eu/v1/bin/LTTS7Jorge.dll) | 56 kB | `25a724844fc88c6b49803d9706246a045d7c405b8235680b8d34b6daa506bc93` |
| [`data/Jorge.vcf`](https://quoquendo.depau.eu/v1/data/Jorge.vcf) | 415 B | `b06ec4b02f9b4fa6b5af2f52f8875a1d3a4a72d24c30e78af8b17c68b4d8b05e` |
| [`data/voices/Jorge/Jorge.32000.LTTS7Codec210.44000.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Jorge/Jorge.32000.LTTS7Codec210.44000.10.bin) | 27.8 MB | `e8b355b0fecc600de382b1a2f2345c9b1b134b0ab328e06e952ba00f54387d95` |
| [`data/voices/Jorge/Jorge.sde`](https://quoquendo.depau.eu/v1/data/voices/Jorge/Jorge.sde) | 1.4 MB | `5e60f973f1bcce45bad30a7eb87e49239daeaa555fc83615b6d3b69e33ca5693` |
| [`data/voices/Jorge/Jorge.vde`](https://quoquendo.depau.eu/v1/data/voices/Jorge/Jorge.vde) | 460 B | `c42ada4c8019c40c9702c4bc6754cc06d76d1d83fd89f89e4aa0d7ecdbde653e` |
| [`data/voices/Jorge/JorgeGilded.gde`](https://quoquendo.depau.eu/v1/data/voices/Jorge/JorgeGilded.gde) | 6 kB | `33b4f2d3993a13bd11602525f608cbf9df03774a1749b7c272fad7a1e16d1239` |
| [`data/voices/Jorge/JorgeGildedParalinguistics.32000.LTTS7Codec210.44000.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Jorge/JorgeGildedParalinguistics.32000.LTTS7Codec210.44000.10.bin) | 1.0 MB | `0ecbfbfe029c7d4ee8f81ebed54bf8e33dab19bd6a6654c4fd76f6051bf95566` |
| [`data/voices/Jorge/JorgeGildedParalinguistics.sde`](https://quoquendo.depau.eu/v1/data/voices/Jorge/JorgeGildedParalinguistics.sde) | 4 kB | `3c482db88a1e06bb2a76105d13cdb2af61988629a3c5fe18c401973f5946fd72` |
| [`data/voices/Jorge/JorgeGildedPhrases.32000.LTTS7Codec210.44000.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Jorge/JorgeGildedPhrases.32000.LTTS7Codec210.44000.10.bin) | 955 kB | `02c98383c4bd46e95b28bdd03c1db8919591df5fbe2b87ea2892a36a8d01e888` |
| [`data/voices/Jorge/JorgeGildedPhrases.sde`](https://quoquendo.depau.eu/v1/data/voices/Jorge/JorgeGildedPhrases.sde) | 52 kB | `5a4bc32d4d1796f3a35b8c4f7627e5a604d09d99adbaa6d144b8d2549e47977f` |

### Juliette

| File | Size | SHA-256 |
|---|---:|---|
| [`bin/LTTS7Juliette.dll`](https://quoquendo.depau.eu/v1/bin/LTTS7Juliette.dll) | 56 kB | `9a6329276660582e13878737a45f38b5f869cdbfa681f767fe90d4974a46b408` |
| [`data/Juliette.vcf`](https://quoquendo.depau.eu/v1/data/Juliette.vcf) | 348 B | `dc29e6df03a5890009d798c348c0604b7405e106385b025260f5bba3ed83d086` |
| [`data/voices/Juliette/Juliette.32000.LTTS7Codec210.44000.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Juliette/Juliette.32000.LTTS7Codec210.44000.10.bin) | 34.6 MB | `eeded7c2994cf9f20380c6fa972661d1427483aa8419941e66c5f258b37b4af2` |
| [`data/voices/Juliette/Juliette.sde`](https://quoquendo.depau.eu/v1/data/voices/Juliette/Juliette.sde) | 1.4 MB | `8ac5be20175b922506b31ef3919a0c8066a67d0fdc859200843a11dd8be18483` |
| [`data/voices/Juliette/Juliette.vde`](https://quoquendo.depau.eu/v1/data/voices/Juliette/Juliette.vde) | 491 B | `6b68d038d2d26c156bda88d30547858970d9922d740cb4192d5e41724024d27a` |
| [`data/voices/Juliette/JulietteGilded.gde`](https://quoquendo.depau.eu/v1/data/voices/Juliette/JulietteGilded.gde) | 8 kB | `fd5b10afb7217fa3b96665dfab4417a8b8c579b330f7d74c51dba64dea57f2cc` |
| [`data/voices/Juliette/JulietteGildedParalinguistics.32000.LTTS7Codec210.44000.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Juliette/JulietteGildedParalinguistics.32000.LTTS7Codec210.44000.10.bin) | 764 kB | `fb760df57d2cd761b1c45395456ef1437673f071a8acb4dfdd163ad5053594e7` |
| [`data/voices/Juliette/JulietteGildedParalinguistics.sde`](https://quoquendo.depau.eu/v1/data/voices/Juliette/JulietteGildedParalinguistics.sde) | 2 kB | `43adf64f7d0bb523e4d692971ea21eabedae161020aa7778c098291537aa6091` |
| [`data/voices/Juliette/JulietteGildedPhrases.32000.LTTS7Codec210.44000.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Juliette/JulietteGildedPhrases.32000.LTTS7Codec210.44000.10.bin) | 1.7 MB | `2575271b1473dadd4609b7baf6731fdf42258f81bd574aced6b30e20c58f5694` |
| [`data/voices/Juliette/JulietteGildedPhrases.sde`](https://quoquendo.depau.eu/v1/data/voices/Juliette/JulietteGildedPhrases.sde) | 70 kB | `1bd3541dd96cd3ac1250e62246f59a0498f10d61966955047a884e96dda43437` |

### Kate

| File | Size | SHA-256 |
|---|---:|---|
| [`bin/LTTS7Kate.dll`](https://quoquendo.depau.eu/v1/bin/LTTS7Kate.dll) | 68 kB | `64f77f0b38e0384b8ea56c913cacdb5d86562a7a751d38072df9bb195a84ec1e` |
| [`data/Kate.vcf`](https://quoquendo.depau.eu/v1/data/Kate.vcf) | 411 B | `cd9d0706cc29819136580393cc12d9a34d23d841103d6fed3b90ad1b06338fab` |
| [`data/voices/Kate/Kate.32000.LTTS7Codec210.44000.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Kate/Kate.32000.LTTS7Codec210.44000.10.bin) | 47.4 MB | `f30330f1fb981b5b4ae57d7035286ed4a92e71183a3c2fec36eec8dc63cb18f0` |
| [`data/voices/Kate/Kate.sde`](https://quoquendo.depau.eu/v1/data/voices/Kate/Kate.sde) | 2.2 MB | `19682b791b851278fb5b23f9ddb282edb37b94f4fa0e9cc881d89cc27e770fc2` |
| [`data/voices/Kate/Kate.vde`](https://quoquendo.depau.eu/v1/data/voices/Kate/Kate.vde) | 509 B | `6f53b065f4d55527e9cd70f86bdacaef49ec5e717dbd75c112f050a9b01f56b1` |
| [`data/voices/Kate/KateGilded.gde`](https://quoquendo.depau.eu/v1/data/voices/Kate/KateGilded.gde) | 12 kB | `b009a277f9a7242047de639a2eedfc2af7ce20ba190c1564091a6eb466ac6457` |
| [`data/voices/Kate/KateGildedParalinguistics.32000.LTTS7Codec210.44000.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Kate/KateGildedParalinguistics.32000.LTTS7Codec210.44000.10.bin) | 625 kB | `9fcd062ee094b27d60cd182b2949567a8d0b87ee46aa712d46e58ba893969391` |
| [`data/voices/Kate/KateGildedParalinguistics.sde`](https://quoquendo.depau.eu/v1/data/voices/Kate/KateGildedParalinguistics.sde) | 4 kB | `a92e1ad86edbf2d8dfa402d462c5ff90ac4c17b4fa8b19d070ffce1812a9e961` |
| [`data/voices/Kate/KateGildedPhrases.32000.LTTS7Codec210.44000.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Kate/KateGildedPhrases.32000.LTTS7Codec210.44000.10.bin) | 3.1 MB | `bcdcdf30c4fa52968dfaa082862339dd8c557c2fc24f38fda31ec24a3ad825b3` |
| [`data/voices/Kate/KateGildedPhrases.sde`](https://quoquendo.depau.eu/v1/data/voices/Kate/KateGildedPhrases.sde) | 146 kB | `8484cb05c275f5fdc6375e6acc314d4018f2e56a6082f34c849508ead5f8c60a` |

### Katrin

| File | Size | SHA-256 |
|---|---:|---|
| [`bin/LTTS7Katrin.dll`](https://quoquendo.depau.eu/v1/bin/LTTS7Katrin.dll) | 64 kB | `61b579cd6c480aa48b69049c4533b00596b2ed1246fcc17c2a16518b000bbf76` |
| [`data/Katrin.vcf`](https://quoquendo.depau.eu/v1/data/Katrin.vcf) | 412 B | `7d3f557d2e69ed9574c934f163463b284d07810b8d84ad9efb597b9709d7dcac` |
| [`data/voices/Katrin/Katrin.32000.LTTS7Codec210.44000.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Katrin/Katrin.32000.LTTS7Codec210.44000.10.bin) | 47.7 MB | `f481442e0964f74854989673990b237c277fbec3b8180df824afaeda6764ec8b` |
| [`data/voices/Katrin/Katrin.sde`](https://quoquendo.depau.eu/v1/data/voices/Katrin/Katrin.sde) | 2.4 MB | `4cf8ee1269607481e91a504ce689cb51ba819364afd1b1795feaf021317fa00a` |
| [`data/voices/Katrin/Katrin.vde`](https://quoquendo.depau.eu/v1/data/voices/Katrin/Katrin.vde) | 456 B | `26fa2d3810ecb87f5d7e1fcbabf2d8831688eb2a3523b7fc2c98a2e0e55c3b62` |
| [`data/voices/Katrin/KatrinGilded.gde`](https://quoquendo.depau.eu/v1/data/voices/Katrin/KatrinGilded.gde) | 8 kB | `5c516427a61fa4e99efcb072c38bd3406807cad17b1b867adad5693c0dcf7607` |
| [`data/voices/Katrin/KatrinGildedParalinguistics.32000.LTTS7Codec210.44000.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Katrin/KatrinGildedParalinguistics.32000.LTTS7Codec210.44000.10.bin) | 354 kB | `9abe6ce09080d844cb19c19240d2a8168d2a251df7aabb96791ad5c912cb4b8b` |
| [`data/voices/Katrin/KatrinGildedParalinguistics.sde`](https://quoquendo.depau.eu/v1/data/voices/Katrin/KatrinGildedParalinguistics.sde) | 2 kB | `8171918dbf6180f3cda973aa70ae346da4448601380b6d5a71d811bf65e23f4f` |
| [`data/voices/Katrin/KatrinGildedPhrases.32000.LTTS7Codec210.44000.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Katrin/KatrinGildedPhrases.32000.LTTS7Codec210.44000.10.bin) | 1.9 MB | `7c535511b8f242c2b51c93f75a5ad4e861a1f14ac0876cfa85b7aff45e9779d8` |
| [`data/voices/Katrin/KatrinGildedPhrases.sde`](https://quoquendo.depau.eu/v1/data/voices/Katrin/KatrinGildedPhrases.sde) | 85 kB | `5c47d8d045800ae9823c4bc95890b0e55fbca9ad240ea3d505f75a6f528e3854` |

### Krzysztof

| File | Size | SHA-256 |
|---|---:|---|
| [`bin/LTTS7Krzysztof.dll`](https://quoquendo.depau.eu/v1/bin/LTTS7Krzysztof.dll) | 56 kB | `5ee1f0f4d1f08080f168baea93f6b8cd1a83c30c5db7dc64c9366643269eb0a4` |
| [`data/Krzysztof.vcf`](https://quoquendo.depau.eu/v1/data/Krzysztof.vcf) | 351 B | `6060715e65f461d6c8a10f833d4cb8a2baca98ac41be5af34bd11942cc82d292` |
| [`data/voices/Krzysztof/Krzysztof.32000.LTTS7Codec210.44000.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Krzysztof/Krzysztof.32000.LTTS7Codec210.44000.10.bin) | 36.9 MB | `7776d752025734c0650bcc1562a7efb0f8f2090f40e47e7bb4e4f2b1865aa7f4` |
| [`data/voices/Krzysztof/Krzysztof.sde`](https://quoquendo.depau.eu/v1/data/voices/Krzysztof/Krzysztof.sde) | 1.8 MB | `2dd09b3e29922fe6e5548f420f462a221ab67398cd2bd15f66e21486276f3dac` |
| [`data/voices/Krzysztof/Krzysztof.vde`](https://quoquendo.depau.eu/v1/data/voices/Krzysztof/Krzysztof.vde) | 453 B | `442a7e99c43bb361b8fe7ec0cfbfcf83949e1d7c80ae9fcef172ffa2e2d5b87d` |
| [`data/voices/Krzysztof/KrzysztofGilded.gde`](https://quoquendo.depau.eu/v1/data/voices/Krzysztof/KrzysztofGilded.gde) | 8 kB | `4423193c275549eddf579a705909d2d59b46ce00d3e3ac7aef6045d7e11c316e` |
| [`data/voices/Krzysztof/KrzysztofGildedParalinguistics.32000.LTTS7Codec210.44000.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Krzysztof/KrzysztofGildedParalinguistics.32000.LTTS7Codec210.44000.10.bin) | 773 kB | `b35c735bae346f37cc40ffe960f28fb614384b613992b4c502c01f48ec2604d9` |
| [`data/voices/Krzysztof/KrzysztofGildedParalinguistics.sde`](https://quoquendo.depau.eu/v1/data/voices/Krzysztof/KrzysztofGildedParalinguistics.sde) | 6 kB | `fdd73f23d9941f6dc4a31c70e19273f6016dac4f91d8159f31284f001da3f1eb` |
| [`data/voices/Krzysztof/KrzysztofGildedPhrases.32000.LTTS7Codec210.44000.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Krzysztof/KrzysztofGildedPhrases.32000.LTTS7Codec210.44000.10.bin) | 1.5 MB | `a5d01d249cb9809379ba073cbb398810ae1804c20e864dfb5ffd8ec7f969b3af` |
| [`data/voices/Krzysztof/KrzysztofGildedPhrases.sde`](https://quoquendo.depau.eu/v1/data/voices/Krzysztof/KrzysztofGildedPhrases.sde) | 85 kB | `6e2bf9b1fa2dc862ebb3ecc0b646143e9865911813b03d81389ed75c156591cd` |

### Leonor

| File | Size | SHA-256 |
|---|---:|---|
| [`bin/LTTS7Leonor.dll`](https://quoquendo.depau.eu/v1/bin/LTTS7Leonor.dll) | 56 kB | `f65e725f68ff05b1fb52acfe5c16b4d836596b1ec894e7b9d752ae8168834933` |
| [`data/Leonor.vcf`](https://quoquendo.depau.eu/v1/data/Leonor.vcf) | 349 B | `14beae640337d74759eb08fc6d362cf90d137612f07d3854bdc533a84e1eecbf` |
| [`data/voices/Leonor/Leonor.32000.LTTS7Codec210.44000.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Leonor/Leonor.32000.LTTS7Codec210.44000.10.bin) | 36.7 MB | `9728a8c7db60a42063865a339d74e4e1cc35b742a7b7bb5c587196934746b3f1` |
| [`data/voices/Leonor/Leonor.sde`](https://quoquendo.depau.eu/v1/data/voices/Leonor/Leonor.sde) | 1.7 MB | `1d3abc961098ba0e6ae92e51ba22a50072d9179434e8fa1c9889c6653cc5924d` |
| [`data/voices/Leonor/Leonor.vde`](https://quoquendo.depau.eu/v1/data/voices/Leonor/Leonor.vde) | 480 B | `0befc2b9fa1ec8c54f0798a9cc185a4567a2024e21c014fc36c181ad225c0158` |
| [`data/voices/Leonor/LeonorGilded.gde`](https://quoquendo.depau.eu/v1/data/voices/Leonor/LeonorGilded.gde) | 6 kB | `9f353fcd7ce2c650f95b01834a4d0cc758a11b56feda1150f531d1d328afe9ac` |
| [`data/voices/Leonor/LeonorGildedParalinguistics.32000.LTTS7Codec210.44000.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Leonor/LeonorGildedParalinguistics.32000.LTTS7Codec210.44000.10.bin) | 928 kB | `f3bd7c089ad2ed14c336c95afef37be330dc650818ec270fc128cf64ea8f2c6b` |
| [`data/voices/Leonor/LeonorGildedParalinguistics.sde`](https://quoquendo.depau.eu/v1/data/voices/Leonor/LeonorGildedParalinguistics.sde) | 3 kB | `971a40482bae12af6717c374b9d66410f03daa307e05ab7676e58586d79188f2` |
| [`data/voices/Leonor/LeonorGildedPhrases.32000.LTTS7Codec210.44000.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Leonor/LeonorGildedPhrases.32000.LTTS7Codec210.44000.10.bin) | 1.2 MB | `fba248deb06e2d22165d069649503d79a438e54b2341f99fa9f9e251f108dab5` |
| [`data/voices/Leonor/LeonorGildedPhrases.sde`](https://quoquendo.depau.eu/v1/data/voices/Leonor/LeonorGildedPhrases.sde) | 60 kB | `bc7f21522e7efb0d60a6e0023fe79bd6fd919fea74bfa60a842c2f040594259c` |

### Linlin

| File | Size | SHA-256 |
|---|---:|---|
| [`bin/LTTS7Linlin.dll`](https://quoquendo.depau.eu/v1/bin/LTTS7Linlin.dll) | 56 kB | `2a8fe4a972a344af93b5b2aa46fe005379a088d5fc932971b00c6eb39e492574` |
| [`data/Linlin.vcf`](https://quoquendo.depau.eu/v1/data/Linlin.vcf) | 317 B | `543beee4df89493109ed48be32f1b859459434cd07616281a1f3c023374eb4f6` |
| [`data/voices/Linlin/Linlin.32000.LTTS7Codec210.44000.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Linlin/Linlin.32000.LTTS7Codec210.44000.10.bin) | 29.1 MB | `f343359eb79fa7e42e498a3c0ed03b2a0c6ce4fe987874175b98bb701af0e098` |
| [`data/voices/Linlin/Linlin.sde`](https://quoquendo.depau.eu/v1/data/voices/Linlin/Linlin.sde) | 1.3 MB | `6dcdf882cc2d362c660c1dd2ee3a8dda6f3ad9ba1d0afa5244cf2bcb1f772cd7` |
| [`data/voices/Linlin/Linlin.vde`](https://quoquendo.depau.eu/v1/data/voices/Linlin/Linlin.vde) | 423 B | `57cc1a599dfcdef40a443958200888e7ffec6a76ae676bb4c1019959531d3fb8` |

### Luca

| File | Size | SHA-256 |
|---|---:|---|
| [`bin/LTTS7Luca.dll`](https://quoquendo.depau.eu/v1/bin/LTTS7Luca.dll) | 52 kB | `ed564871226d37750e58114d59fed1c12c804c9676c7292bc28d31367dea5283` |
| [`data/Luca.vcf`](https://quoquendo.depau.eu/v1/data/Luca.vcf) | 327 B | `b520a7c4b53c93133cef18e543bb49f7f2677b7a2d2a510d536d5c517641bf79` |
| [`data/voices/Luca/Luca.32000.LTTS7Codec210.44000.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Luca/Luca.32000.LTTS7Codec210.44000.10.bin) | 35.6 MB | `bb03a2c3ff94cbb9a514d02b5c01f7b2792b0246a899e4f606ea3e8bfe391c8d` |
| [`data/voices/Luca/Luca.sde`](https://quoquendo.depau.eu/v1/data/voices/Luca/Luca.sde) | 1.9 MB | `caed8b6af0a510546f2b73a39c6f1401538a74be4903170d2dcf1bd687f5d106` |
| [`data/voices/Luca/Luca.vde`](https://quoquendo.depau.eu/v1/data/voices/Luca/Luca.vde) | 454 B | `0b18d22835c2da1efbbdb875c2382032c1a0704eb861b4da2ee293c232f1cb1c` |
| [`data/voices/Luca/LucaGilded.gde`](https://quoquendo.depau.eu/v1/data/voices/Luca/LucaGilded.gde) | 14 kB | `6cf06d4a4b51cdaff8834a6ebb342630e053c00052195f331261eac57a4780c5` |
| [`data/voices/Luca/LucaGildedParalinguistics.32000.LTTS7Codec210.44000.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Luca/LucaGildedParalinguistics.32000.LTTS7Codec210.44000.10.bin) | 2.1 MB | `8ca80958e12001ede758b4e6af5e32687f22b4b168b170cf095c50daba1f81a0` |
| [`data/voices/Luca/LucaGildedParalinguistics.sde`](https://quoquendo.depau.eu/v1/data/voices/Luca/LucaGildedParalinguistics.sde) | 19 kB | `e5f99cf7a74077222c6c721b2d7047cc7a764da6a8ac161c6a4aa7057373a052` |
| [`data/voices/Luca/LucaGildedPhrases.32000.LTTS7Codec210.44000.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Luca/LucaGildedPhrases.32000.LTTS7Codec210.44000.10.bin) | 2.6 MB | `f21368efa75f10138df46bd3327a374514c9aa84d7d4382e273f2ddc005efce4` |
| [`data/voices/Luca/LucaGildedPhrases.sde`](https://quoquendo.depau.eu/v1/data/voices/Luca/LucaGildedPhrases.sde) | 158 kB | `98c3f64c94522f0a609f73141a93f6d0d273f07fb72dac38ec5991ed1e235615` |

### Marcello

| File | Size | SHA-256 |
|---|---:|---|
| [`bin/LTTS7Marcello.dll`](https://quoquendo.depau.eu/v1/bin/LTTS7Marcello.dll) | 44 kB | `e00e2182798c2e70d9bb8c2308c80a79bbae3828e43e4848d4be92f578363696` |
| [`data/Marcello.vcf`](https://quoquendo.depau.eu/v1/data/Marcello.vcf) | 187 B | `8a12b73fed6b8d11f5fc9e07a6afd2d1dc29d407cf8062389e1be9a0b7d077c4` |
| [`data/voices/Marcello/Marcello.44000.LTTS7CodecMsx.58300.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Marcello/Marcello.44000.LTTS7CodecMsx.58300.10.bin) | 46.2 MB | `49b0f771813890ebfc8cf0bd429b0e2a683560a6d84fb44b5e9cae999f0d93c9` |
| [`data/voices/Marcello/Marcello.sde`](https://quoquendo.depau.eu/v1/data/voices/Marcello/Marcello.sde) | 1.4 MB | `3c1806dff4328853044aa540e74835c5e8ce4feec9b876ad46f88dccdf763733` |
| [`data/voices/Marcello/Marcello.vde`](https://quoquendo.depau.eu/v1/data/voices/Marcello/Marcello.vde) | 412 B | `8c2ce4eb155432b5f79b74b2ddced32c741d7295d5825b98f5f5abe083cfa28f` |

### Matteo

| File | Size | SHA-256 |
|---|---:|---|
| [`bin/LTTS7Matteo.dll`](https://quoquendo.depau.eu/v1/bin/LTTS7Matteo.dll) | 52 kB | `c6a05cfe09385bb476e979118cf5c8739bc9dff306792ca9bf09dc2095738549` |
| [`data/Matteo.vcf`](https://quoquendo.depau.eu/v1/data/Matteo.vcf) | 411 B | `c81ec272ba7379eddf30d256abcf470800059b68cf1ca19bb950ebd4fda0729d` |
| [`data/voices/Matteo/Matteo.32000.LTTS7Codec210.44000.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Matteo/Matteo.32000.LTTS7Codec210.44000.10.bin) | 35.9 MB | `5a339ee3ec9876d305325057290278020ab04e00223fc96c96ff6ab773508d22` |
| [`data/voices/Matteo/Matteo.sde`](https://quoquendo.depau.eu/v1/data/voices/Matteo/Matteo.sde) | 1.9 MB | `f9cdb87876e64a1fb458e5fb023502657d5a336415f737591371054945648b7d` |
| [`data/voices/Matteo/Matteo.vde`](https://quoquendo.depau.eu/v1/data/voices/Matteo/Matteo.vde) | 461 B | `795ab6c75de524af806db00a32cd5ef7493435fd88b895eca3a8912f35a4665a` |
| [`data/voices/Matteo/MatteoGilded.gde`](https://quoquendo.depau.eu/v1/data/voices/Matteo/MatteoGilded.gde) | 12 kB | `621b2b2bb00d70a7ffbf1c04d167e12aebf65859308eb52bbc7a563a4c2b3551` |
| [`data/voices/Matteo/MatteoGildedParalinguistics.32000.LTTS7Codec210.44000.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Matteo/MatteoGildedParalinguistics.32000.LTTS7Codec210.44000.10.bin) | 839 kB | `d704306c43816230d8b5d9c4cb2d6dca83ea27cb9ba83f51d32928add17b144b` |
| [`data/voices/Matteo/MatteoGildedParalinguistics.sde`](https://quoquendo.depau.eu/v1/data/voices/Matteo/MatteoGildedParalinguistics.sde) | 8 kB | `c3f2e8c7561feae337d2b8aafe169fde1d3bb63e116bf0f0dfab0ab334081db7` |
| [`data/voices/Matteo/MatteoGildedPhrases.32000.LTTS7Codec210.44000.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Matteo/MatteoGildedPhrases.32000.LTTS7Codec210.44000.10.bin) | 2.5 MB | `f5903855df572215cedfeefeeebcbc4cc8bdfda71d1f463ff63ec38b3378e47b` |
| [`data/voices/Matteo/MatteoGildedPhrases.sde`](https://quoquendo.depau.eu/v1/data/voices/Matteo/MatteoGildedPhrases.sde) | 162 kB | `21d31dd913a4c3ba8b00d75998d4498750cc1be508555e11d054ec0a41faa9d0` |

### Olga

| File | Size | SHA-256 |
|---|---:|---|
| [`bin/LTTS7Olga.dll`](https://quoquendo.depau.eu/v1/bin/LTTS7Olga.dll) | 46 kB | `dc5938c1d474c0e47df22cf2537159a0a596fe62d331e0b49b9324c3041549ee` |
| [`data/Olga.vcf`](https://quoquendo.depau.eu/v1/data/Olga.vcf) | 263 B | `380015faafb9ba9dd60904f0510a1b9b271fcb9bada02da9ce12330151304154` |
| [`data/voices/Olga/Olga.44000.LTTS7CodecMsx.58300.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Olga/Olga.44000.LTTS7CodecMsx.58300.10.bin) | 42.5 MB | `77f2c451b8897a729459805b96c237bd9b1d862febf7d3fedfcdc6ed8d2ae0a2` |
| [`data/voices/Olga/Olga.sde`](https://quoquendo.depau.eu/v1/data/voices/Olga/Olga.sde) | 1.7 MB | `dea38f83f743c21783cb1c94b90b52eefd8720fd2840561688aa2ae33b9f3f76` |
| [`data/voices/Olga/Olga.vde`](https://quoquendo.depau.eu/v1/data/voices/Olga/Olga.vde) | 526 B | `0905a9e0e21b3b37876950de48e30a8e5f9ed631cc422e1ecbc93e38e75cbe00` |
| [`data/voices/Olga/OlgaGildedParalinguistics.44000.LTTS7CodecMsx.58300.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Olga/OlgaGildedParalinguistics.44000.LTTS7CodecMsx.58300.10.bin) | 2.0 MB | `3823737d636e3d533786af5b0552521403cf3611a03a98994a91a0084bc4cf12` |
| [`data/voices/Olga/OlgaGildedParalinguistics.sde`](https://quoquendo.depau.eu/v1/data/voices/Olga/OlgaGildedParalinguistics.sde) | 9 kB | `cb5794f3583aae6578db949490fa90128fae53b613d694d3543a344789ba88ee` |
| [`data/voices/Olga/OlgaGildedPhrases.44000.LTTS7CodecMsx.58300.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Olga/OlgaGildedPhrases.44000.LTTS7CodecMsx.58300.10.bin) | 2.7 MB | `09a19664d5501b52b0b43818882043990dc15504119499efecb690f7c58ce47a` |
| [`data/voices/Olga/OlgaGildedPhrases.sde`](https://quoquendo.depau.eu/v1/data/voices/Olga/OlgaGildedPhrases.sde) | 116 kB | `297e067f8ef33e3264e4958339a03d45bee20cdaab5fba41f8f825f8591e69e5` |

### Paola

| File | Size | SHA-256 |
|---|---:|---|
| [`bin/LTTS7Paola.dll`](https://quoquendo.depau.eu/v1/bin/LTTS7Paola.dll) | 52 kB | `5e3317e4e488c238634f0a9ebd5a78b93ff77d7ec4172316588e0e11d6803c23` |
| [`data/Paola.vcf`](https://quoquendo.depau.eu/v1/data/Paola.vcf) | 408 B | `59ced2a098f6beffaf1a598adf602f083db1e1540613acc8ba49776f0fe17712` |
| [`data/voices/Paola/Paola.32000.LTTS7Codec210.44000.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Paola/Paola.32000.LTTS7Codec210.44000.10.bin) | 38.6 MB | `8900303eb320c0edb6732ef6a5614b96e9c373789c5ba671ed5effe5e756d33d` |
| [`data/voices/Paola/Paola.sde`](https://quoquendo.depau.eu/v1/data/voices/Paola/Paola.sde) | 1.9 MB | `8650bf1cb39d81a7afca96d106a5945badcedbde38c8b167036cbd67bea97fb5` |
| [`data/voices/Paola/Paola.vde`](https://quoquendo.depau.eu/v1/data/voices/Paola/Paola.vde) | 463 B | `0acce66b400876ed470e7c703f28c0c8d41bcfbdad2f7d1d19f096442fee370a` |
| [`data/voices/Paola/PaolaGilded.gde`](https://quoquendo.depau.eu/v1/data/voices/Paola/PaolaGilded.gde) | 12 kB | `59c0616909363799f4328e308f8a1058b981d3a32b784884f9c3bcbc76d5521b` |
| [`data/voices/Paola/PaolaGildedParalinguistics.32000.LTTS7Codec210.44000.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Paola/PaolaGildedParalinguistics.32000.LTTS7Codec210.44000.10.bin) | 1.0 MB | `7768bfa4b42893c354c37ee4c75b14511a7ab5961a189a0641e9ed68eaeeda0c` |
| [`data/voices/Paola/PaolaGildedParalinguistics.sde`](https://quoquendo.depau.eu/v1/data/voices/Paola/PaolaGildedParalinguistics.sde) | 7 kB | `b85374f64846b5a0f85bcc95c22e49290ffe8836dc7bdb34c4dcffab006e86d0` |
| [`data/voices/Paola/PaolaGildedPhrases.32000.LTTS7Codec210.44000.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Paola/PaolaGildedPhrases.32000.LTTS7Codec210.44000.10.bin) | 2.8 MB | `d4f5a37acbc0fb1f6d11de57115f16eab12ec9b8733f13bf556169be2822dbf9` |
| [`data/voices/Paola/PaolaGildedPhrases.sde`](https://quoquendo.depau.eu/v1/data/voices/Paola/PaolaGildedPhrases.sde) | 159 kB | `4602ca5c9fdd69e2e1aa0d714cbe0cdd4599e481d99c7e771ce968a84b73ca7d` |

### Roberto

| File | Size | SHA-256 |
|---|---:|---|
| [`bin/LTTS7Roberto.dll`](https://quoquendo.depau.eu/v1/bin/LTTS7Roberto.dll) | 56 kB | `2ff4c970a96cdff4128850cbfc44607826a730c12d89b4d9f80623528c5ae69f` |
| [`data/Roberto.vcf`](https://quoquendo.depau.eu/v1/data/Roberto.vcf) | 318 B | `174ced0bc0ab302754a7a0b4fc0e62b8d19de3c429640fdfb165e3b323e441ab` |
| [`data/voices/Roberto/Roberto.32000.LTTS7Codec210.44000.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Roberto/Roberto.32000.LTTS7Codec210.44000.10.bin) | 35.2 MB | `8ae06055ef1dc9949fdce92d30d81696142e4e9d0df177bd0c7c347576eddd45` |
| [`data/voices/Roberto/Roberto.sde`](https://quoquendo.depau.eu/v1/data/voices/Roberto/Roberto.sde) | 1.6 MB | `f667be866cf5a46fe7a2e5ccf036e8199bcecb9c68c8739126a0dd4b2c40147e` |
| [`data/voices/Roberto/Roberto.vde`](https://quoquendo.depau.eu/v1/data/voices/Roberto/Roberto.vde) | 379 B | `c74f789c99c97e640c435dce3e15c17f459cffac1fb9735dafaf19b78ea51c66` |

### Simon

| File | Size | SHA-256 |
|---|---:|---|
| [`bin/LTTS7Simon.dll`](https://quoquendo.depau.eu/v1/bin/LTTS7Simon.dll) | 68 kB | `9f8bfc012b01de7bdac349070e8dcb35f11200a77c23ee6505c0066f88683a24` |
| [`data/Simon.vcf`](https://quoquendo.depau.eu/v1/data/Simon.vcf) | 414 B | `0e0e7eae9e13fbf0e5d969667dc16227195e35e698088eaf2bc51f3774d67960` |
| [`data/voices/Simon/Simon.32000.LTTS7Codec210.44000.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Simon/Simon.32000.LTTS7Codec210.44000.10.bin) | 44.7 MB | `c02a2bad9c0a014c8d72128b5522c9975cbf7def98ee76e34de6522342662bee` |
| [`data/voices/Simon/Simon.sde`](https://quoquendo.depau.eu/v1/data/voices/Simon/Simon.sde) | 2.1 MB | `7c98a7f66bc9bd05417850d74648ee435a6b8ebf8999863bcd1b83c42b6a54d7` |
| [`data/voices/Simon/Simon.vde`](https://quoquendo.depau.eu/v1/data/voices/Simon/Simon.vde) | 506 B | `13a3d03971a3d0cc13074fd6dd1ea1b8f2e42b2db9996334844488d810637e88` |
| [`data/voices/Simon/SimonGilded.gde`](https://quoquendo.depau.eu/v1/data/voices/Simon/SimonGilded.gde) | 12 kB | `040216883d6bb7b7687eb0ed5988c4a166268e216c73463b73631f1d10ad9918` |
| [`data/voices/Simon/SimonGildedParalinguistics.32000.LTTS7Codec210.44000.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Simon/SimonGildedParalinguistics.32000.LTTS7Codec210.44000.10.bin) | 1000 kB | `9b4e210a73b7656dd82a4d40aba7a04d73748fbb4e9eb2e6bd18a9256bf29ce7` |
| [`data/voices/Simon/SimonGildedParalinguistics.sde`](https://quoquendo.depau.eu/v1/data/voices/Simon/SimonGildedParalinguistics.sde) | 4 kB | `0d78943e45ee38e476c18a8631f45fb5d0fb434aa15f0566cb3c5d364bc8d968` |
| [`data/voices/Simon/SimonGildedPhrases.32000.LTTS7Codec210.44000.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Simon/SimonGildedPhrases.32000.LTTS7Codec210.44000.10.bin) | 2.8 MB | `e503bdecf8bcde10612b972973698a962e4f40b375df4e3a2b779ecd87c95102` |
| [`data/voices/Simon/SimonGildedPhrases.sde`](https://quoquendo.depau.eu/v1/data/voices/Simon/SimonGildedPhrases.sde) | 144 kB | `27ec5a1424d312f1114fc679bfbdc620543c3133259456c16141d7c690b9dfb4` |

### Soledad

| File | Size | SHA-256 |
|---|---:|---|
| [`bin/LTTS7Soledad.dll`](https://quoquendo.depau.eu/v1/bin/LTTS7Soledad.dll) | 56 kB | `a4a96b4bbde3f5b6579b22daa04fc1f9f2114989594c1df3ade8ffc162d59193` |
| [`data/Soledad.vcf`](https://quoquendo.depau.eu/v1/data/Soledad.vcf) | 353 B | `85e32971f74d651eb70a28c849d2896a11780a71ff5afea3c658e5e43079dd48` |
| [`data/voices/Soledad/Soledad.32000.LTTS7Codec210.44000.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Soledad/Soledad.32000.LTTS7Codec210.44000.10.bin) | 31.6 MB | `4af4841d27e6f5838b463333dbf563e7d5b8e4fc927e9745c7e5b68b8b1f7ef2` |
| [`data/voices/Soledad/Soledad.sde`](https://quoquendo.depau.eu/v1/data/voices/Soledad/Soledad.sde) | 1.7 MB | `f874c608e4eeefb87c533738b0b2a5f59f2aa0bd28058022b378696de5944752` |
| [`data/voices/Soledad/Soledad.vde`](https://quoquendo.depau.eu/v1/data/voices/Soledad/Soledad.vde) | 531 B | `3ce065f4f5ee501850058e5b750815fb68294f832c69f7fa5c3e8aeb7db06a0c` |
| [`data/voices/Soledad/SoledadGilded.gde`](https://quoquendo.depau.eu/v1/data/voices/Soledad/SoledadGilded.gde) | 9 kB | `d6800c51ed71ef85d887dbaf466716cfae709e12d29ae1a5a46ca0a002ac2024` |
| [`data/voices/Soledad/SoledadGildedParalinguistics.32000.LTTS7Codec210.44000.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Soledad/SoledadGildedParalinguistics.32000.LTTS7Codec210.44000.10.bin) | 862 kB | `10a4365e81511ef0e15e158345f1ec61d3f286cf4735f89ab9b2b1661b51b62c` |
| [`data/voices/Soledad/SoledadGildedParalinguistics.sde`](https://quoquendo.depau.eu/v1/data/voices/Soledad/SoledadGildedParalinguistics.sde) | 5 kB | `5218c22cce6178fb38e2132552f793d2184d09dc1ab3810108593acd520dc1de` |
| [`data/voices/Soledad/SoledadGildedPhrases.32000.LTTS7Codec210.44000.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Soledad/SoledadGildedPhrases.32000.LTTS7Codec210.44000.10.bin) | 2.2 MB | `74a0d9e0cf6cfa66f73bea7fe46fa09ac9322120eae3eaf1f31e2bfb74ce76b1` |
| [`data/voices/Soledad/SoledadGildedPhrases.sde`](https://quoquendo.depau.eu/v1/data/voices/Soledad/SoledadGildedPhrases.sde) | 127 kB | `18d62bf7238c6e29af5445527b39773d3a18e8f7fd1de8b138c370a330b1c368` |

### Stefan

| File | Size | SHA-256 |
|---|---:|---|
| [`bin/LTTS7Stefan.dll`](https://quoquendo.depau.eu/v1/bin/LTTS7Stefan.dll) | 64 kB | `f4394e230cd7ab8c3e2bd2ef710b7313c270b30f23dd02635c7fd4e7851cb851` |
| [`data/Stefan.vcf`](https://quoquendo.depau.eu/v1/data/Stefan.vcf) | 410 B | `0f666a08224f34a8d48eb2c359a316495ed7e39403d94b846d86b5b881b741f8` |
| [`data/voices/Stefan/Stefan.32000.LTTS7Codec210.44000.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Stefan/Stefan.32000.LTTS7Codec210.44000.10.bin) | 44.0 MB | `52e660eba40658b2f637fd4d1eba40f1afb513488518ca33a5184216933bd8de` |
| [`data/voices/Stefan/Stefan.sde`](https://quoquendo.depau.eu/v1/data/voices/Stefan/Stefan.sde) | 1.9 MB | `93a986e8560cb0f5fee9474d43ce64a1551f32a828aeff58c88b011ef93ff3d6` |
| [`data/voices/Stefan/Stefan.vde`](https://quoquendo.depau.eu/v1/data/voices/Stefan/Stefan.vde) | 455 B | `84ac75ceb0245398fb12e2af086d3d1f1d5759306610a3ab18a43d722d8c20e1` |
| [`data/voices/Stefan/StefanGilded.gde`](https://quoquendo.depau.eu/v1/data/voices/Stefan/StefanGilded.gde) | 8 kB | `54b2981aef66252574bbbce70826ee59f5684d8667556c4e5d8bb7142f18d381` |
| [`data/voices/Stefan/StefanGildedParalinguistics.32000.LTTS7Codec210.44000.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Stefan/StefanGildedParalinguistics.32000.LTTS7Codec210.44000.10.bin) | 855 kB | `f1c2afad938549aeed760e8211dd24e8d5cb145b62f5fb73735d3ecea2ad03b0` |
| [`data/voices/Stefan/StefanGildedParalinguistics.sde`](https://quoquendo.depau.eu/v1/data/voices/Stefan/StefanGildedParalinguistics.sde) | 3 kB | `d09837b10a36ab01529869b3693d48ae9602a2e544b2340a5fa754727ac6e2a5` |
| [`data/voices/Stefan/StefanGildedPhrases.32000.LTTS7Codec210.44000.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Stefan/StefanGildedPhrases.32000.LTTS7Codec210.44000.10.bin) | 2.0 MB | `b0e9744f4ff207e2984ca9469e57d1ca0b40b9482fc627fc096fd20e0be83608` |
| [`data/voices/Stefan/StefanGildedPhrases.sde`](https://quoquendo.depau.eu/v1/data/voices/Stefan/StefanGildedPhrases.sde) | 81 kB | `a4a0052a39616160f187786c29844049c42504403da248c9730da55f69398feb` |

### Steven

| File | Size | SHA-256 |
|---|---:|---|
| [`bin/LTTS7Steven.dll`](https://quoquendo.depau.eu/v1/bin/LTTS7Steven.dll) | 68 kB | `7207ff006731a9268b588652126227a6ad12d327eb9a9f748cfc76f4b170d68e` |
| [`data/Steven.vcf`](https://quoquendo.depau.eu/v1/data/Steven.vcf) | 409 B | `0cd7b3729f6369b7c4f5ca3a627f3a74077252b22c13633dd648e9d121bddd85` |
| [`data/voices/Steven/Steven.32000.LTTS7Codec210.44000.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Steven/Steven.32000.LTTS7Codec210.44000.10.bin) | 33.7 MB | `fe9705547b31e6238d8e791ad20e59d80271e440daa35345358b1551d90f2874` |
| [`data/voices/Steven/Steven.sde`](https://quoquendo.depau.eu/v1/data/voices/Steven/Steven.sde) | 1.7 MB | `a44f90d4c405d3437a3bfbed437fe890e022dc6b59acac291785dfaf2b13bbfb` |
| [`data/voices/Steven/Steven.vde`](https://quoquendo.depau.eu/v1/data/voices/Steven/Steven.vde) | 499 B | `57cac940624dbd2b71f403634c7053d7c9ff636cf756ba34ca3d3175826610b3` |
| [`data/voices/Steven/StevenGilded.gde`](https://quoquendo.depau.eu/v1/data/voices/Steven/StevenGilded.gde) | 12 kB | `121bd8e574ba6af090c6249ad1a6fb29fdc311ccc0ce7acc8cd04e7491710855` |
| [`data/voices/Steven/StevenGildedParalinguistics.32000.LTTS7Codec210.44000.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Steven/StevenGildedParalinguistics.32000.LTTS7Codec210.44000.10.bin) | 1020 kB | `4fefd02d17e4b7a913e41a84ddae82b8d4af70d781e1ae8cf20c011229ade612` |
| [`data/voices/Steven/StevenGildedParalinguistics.sde`](https://quoquendo.depau.eu/v1/data/voices/Steven/StevenGildedParalinguistics.sde) | 8 kB | `5395e96407d4ee5994baf2551e2f1d045bb0acfd72484114085203fd0d15a437` |
| [`data/voices/Steven/StevenGildedPhrases.32000.LTTS7Codec210.44000.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Steven/StevenGildedPhrases.32000.LTTS7Codec210.44000.10.bin) | 2.8 MB | `7df3db8a86b14f5c18c67cb20a75ced22aa190af0726ba57aa384b7d3d8b9cfe` |
| [`data/voices/Steven/StevenGildedPhrases.sde`](https://quoquendo.depau.eu/v1/data/voices/Steven/StevenGildedPhrases.sde) | 133 kB | `84716bfee660d08073d19483ec67c3f029a56550b6df5fae1945009028c9c8e6` |

### Susan

| File | Size | SHA-256 |
|---|---:|---|
| [`bin/LTTS7Susan.dll`](https://quoquendo.depau.eu/v1/bin/LTTS7Susan.dll) | 68 kB | `c0a96a412287747eded6cc284c5d93e0e498b461e4051ed9bb162cc51139c10c` |
| [`data/Susan.vcf`](https://quoquendo.depau.eu/v1/data/Susan.vcf) | 416 B | `82afaa0bb694e71d9694519d1c85a001fdf4195a8d202300d0ef48d9fd084322` |
| [`data/voices/Susan/Susan.32000.LTTS7Codec210.44000.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Susan/Susan.32000.LTTS7Codec210.44000.10.bin) | 46.2 MB | `b2b063cf0a551cd9554b88f97a90946093576626271961186e440f1e3f84988b` |
| [`data/voices/Susan/Susan.sde`](https://quoquendo.depau.eu/v1/data/voices/Susan/Susan.sde) | 2.3 MB | `cdf8d446d0cdb5aff53b9e7b89437aa30befcd0b702b9c9b0da299434ed1caee` |
| [`data/voices/Susan/Susan.vde`](https://quoquendo.depau.eu/v1/data/voices/Susan/Susan.vde) | 543 B | `a3cb99a174425103573ac782fbad559a8012deaff2eee5253aac7e3beb9b70bb` |
| [`data/voices/Susan/SusanGilded.gde`](https://quoquendo.depau.eu/v1/data/voices/Susan/SusanGilded.gde) | 10 kB | `f136fc07ca0ec1ddf93b54050ba764ccd781a403909df421a463b68b4fb74917` |
| [`data/voices/Susan/SusanGildedParalinguistics.32000.LTTS7Codec210.44000.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Susan/SusanGildedParalinguistics.32000.LTTS7Codec210.44000.10.bin) | 440 kB | `c52779888a0ab47c5548adaa4782a4f0ddae79e01d567759abceed6c03056dd1` |
| [`data/voices/Susan/SusanGildedParalinguistics.sde`](https://quoquendo.depau.eu/v1/data/voices/Susan/SusanGildedParalinguistics.sde) | 4 kB | `37283fafa9bd653d3f53bc8e20e03aa2c1722d91d1cdbe3a6fde67af2ca20523` |
| [`data/voices/Susan/SusanGildedPhrases.32000.LTTS7Codec210.44000.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Susan/SusanGildedPhrases.32000.LTTS7Codec210.44000.10.bin) | 2.7 MB | `16ebb09395d7a757bc289c11f8027adaeee4aca7abd260728042f0eeab13ef16` |
| [`data/voices/Susan/SusanGildedPhrases.sde`](https://quoquendo.depau.eu/v1/data/voices/Susan/SusanGildedPhrases.sde) | 131 kB | `fe6496fa7d4e6e24b886de8ea86a8fc24ed77c7868b49d80d49aa18e4aecb265` |

### Sven

| File | Size | SHA-256 |
|---|---:|---|
| [`bin/LTTS7Sven.dll`](https://quoquendo.depau.eu/v1/bin/LTTS7Sven.dll) | 64 kB | `644fa6de9829f8dc7f42759f592ef98f8ac928d7ae080fffdd86135baf83ce49` |
| [`data/Sven.vcf`](https://quoquendo.depau.eu/v1/data/Sven.vcf) | 390 B | `edf6c323f3e1d49fcaad386bd9a569b765e4815dc1739e064ef8a47b4d2baab2` |
| [`data/voices/Sven/Sven.32000.LTTS7Codec210.44000.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Sven/Sven.32000.LTTS7Codec210.44000.10.bin) | 50.9 MB | `1623abef391aefa52b04ae2ab8ab0e1f171056da6f870239bd33d1781b6460ac` |
| [`data/voices/Sven/Sven.sde`](https://quoquendo.depau.eu/v1/data/voices/Sven/Sven.sde) | 2.2 MB | `110f63ba3bfd5b4198d351de411028fa52c6e7050c2000644a2a808abc702df4` |
| [`data/voices/Sven/Sven.vde`](https://quoquendo.depau.eu/v1/data/voices/Sven/Sven.vde) | 459 B | `4d3994d62503e5728324b5b1e596b6ddd85eff427ce1dd63a6601756d3e38434` |
| [`data/voices/Sven/SvenGilded.gde`](https://quoquendo.depau.eu/v1/data/voices/Sven/SvenGilded.gde) | 8 kB | `6f150813fefa2cb4420e0c13542dfc0cf367de7898d7fb12f98725c0a536dded` |
| [`data/voices/Sven/SvenGildedParalinguistics.32000.LTTS7Codec210.44000.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Sven/SvenGildedParalinguistics.32000.LTTS7Codec210.44000.10.bin) | 816 kB | `9df4cc8ad1f5bd6bf83888346ab76fb0cfb774279c139f209b96c447f6c2ae93` |
| [`data/voices/Sven/SvenGildedParalinguistics.sde`](https://quoquendo.depau.eu/v1/data/voices/Sven/SvenGildedParalinguistics.sde) | 6 kB | `50407f6b666f273227e9e3039ee3a11f36c8bd48746b98e2d327de1aa2e023e7` |
| [`data/voices/Sven/SvenGildedPhrases.32000.LTTS7Codec210.44000.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Sven/SvenGildedPhrases.32000.LTTS7Codec210.44000.10.bin) | 1.6 MB | `5f637950af49f708cebbc7bb41295947dff950bf68156d6ef16273b86438b312` |
| [`data/voices/Sven/SvenGildedPhrases.sde`](https://quoquendo.depau.eu/v1/data/voices/Sven/SvenGildedPhrases.sde) | 84 kB | `f7aeefaeff28d35cb85b044131865c72391fb974a66c4da0002dbf2f572c79cb` |

### Willem

| File | Size | SHA-256 |
|---|---:|---|
| [`bin/LTTS7Willem.dll`](https://quoquendo.depau.eu/v1/bin/LTTS7Willem.dll) | 50 kB | `f6d037d1e568ae0698af36f16a86188859c3919a3b596bb09791117d74524bda` |
| [`data/Willem.vcf`](https://quoquendo.depau.eu/v1/data/Willem.vcf) | 269 B | `3b1dab089ba5b713b07507a1b7294944ef71172a5e87f386ea1eabc5345d6a35` |
| [`data/voices/Willem/Willem.44000.LTTS7CodecMsx.58300.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Willem/Willem.44000.LTTS7CodecMsx.58300.10.bin) | 59.9 MB | `0ac250ddf724fd2b62910b9be26fc3f952c76ab83a46b6d1634dd35119ef2dd9` |
| [`data/voices/Willem/Willem.sde`](https://quoquendo.depau.eu/v1/data/voices/Willem/Willem.sde) | 2.2 MB | `08b588c72f9c9952a8760e483df883f37c6aa01c1b0ce5725f1a6d55e24d2390` |
| [`data/voices/Willem/Willem.vde`](https://quoquendo.depau.eu/v1/data/voices/Willem/Willem.vde) | 436 B | `e09de0a111899b97e7455bfe6329b43fc0f629689beb6ee0534c90482f7f3bf2` |
| [`data/voices/Willem/WillemGildedParalinguistics.44000.LTTS7CodecMsx.58300.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Willem/WillemGildedParalinguistics.44000.LTTS7CodecMsx.58300.10.bin) | 961 kB | `402bd1d8d9b2176ffb8d39f77659d8720ca1dc592308cb88712cc3d2f4e3cc18` |
| [`data/voices/Willem/WillemGildedParalinguistics.sde`](https://quoquendo.depau.eu/v1/data/voices/Willem/WillemGildedParalinguistics.sde) | 3 kB | `a6258b09b7cc62d03267c4a5f5c774d178d84132696727302c60628185844902` |
| [`data/voices/Willem/WillemGildedPhrases.44000.LTTS7CodecMsx.58300.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Willem/WillemGildedPhrases.44000.LTTS7CodecMsx.58300.10.bin) | 3.5 MB | `9c2a8812a1f24256e65b03a2d97a89bd24fbc94160709e9bf431b364c2a499d5` |
| [`data/voices/Willem/WillemGildedPhrases.sde`](https://quoquendo.depau.eu/v1/data/voices/Willem/WillemGildedPhrases.sde) | 142 kB | `4e0e914899edd69cdd7281b5218c5db1f2ec5a8b0c225aa6462e97dc4ce54490` |

### Ximena

| File | Size | SHA-256 |
|---|---:|---|
| [`bin/LTTS7Ximena.dll`](https://quoquendo.depau.eu/v1/bin/LTTS7Ximena.dll) | 46 kB | `8bb9b1702b7b791233e2d7b1f23c34779954f3b7f109ce3aa441a8fca917e695` |
| [`data/Ximena.vcf`](https://quoquendo.depau.eu/v1/data/Ximena.vcf) | 282 B | `f08d8ce23054e9259bdb7b2f1c816bebb0bf315c213e60b5a1c3643ce9d0cdc7` |
| [`data/voices/Ximena/Ximena.44000.LTTS7CodecMsx.58300.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Ximena/Ximena.44000.LTTS7CodecMsx.58300.10.bin) | 45.4 MB | `9d594d55b7702861605bedb613e314c2877d3d3bf1a5f208b881c51df6eb462b` |
| [`data/voices/Ximena/Ximena.sde`](https://quoquendo.depau.eu/v1/data/voices/Ximena/Ximena.sde) | 1.6 MB | `7a4cdcbbfdf4d91f738e9b1863f5f71af7eecc267cdc3768e99f2ff36b7d2db9` |
| [`data/voices/Ximena/Ximena.vde`](https://quoquendo.depau.eu/v1/data/voices/Ximena/Ximena.vde) | 473 B | `9d2973e083437f3d9bbfcc80345adf922488d64fcb037634ae17803abf4fe548` |
| [`data/voices/Ximena/XimenaGildedParalinguistics.44000.LTTS7CodecMsx.58300.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Ximena/XimenaGildedParalinguistics.44000.LTTS7CodecMsx.58300.10.bin) | 1.5 MB | `4ef1e8bc9ef33b8ed57b3215e4b028d9001fe184bc2aedf87463079c88c6dd31` |
| [`data/voices/Ximena/XimenaGildedParalinguistics.sde`](https://quoquendo.depau.eu/v1/data/voices/Ximena/XimenaGildedParalinguistics.sde) | 7 kB | `e603a156b49ba43b85677564750323303bd2c4191090e0d26582baf0c6b25a11` |
| [`data/voices/Ximena/XimenaGildedPhrases.44000.LTTS7CodecMsx.58300.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Ximena/XimenaGildedPhrases.44000.LTTS7CodecMsx.58300.10.bin) | 3.5 MB | `d601120449ad89cb3d3ea372ccf12d91351a16af1f573dbd675239c466b813e8` |
| [`data/voices/Ximena/XimenaGildedPhrases.sde`](https://quoquendo.depau.eu/v1/data/voices/Ximena/XimenaGildedPhrases.sde) | 129 kB | `b7cad6dbddce31efb13ddff7e82f2506ef4a27e187919dc771eb4d27e079a217` |

### Zosia

| File | Size | SHA-256 |
|---|---:|---|
| [`bin/LTTS7Zosia.dll`](https://quoquendo.depau.eu/v1/bin/LTTS7Zosia.dll) | 56 kB | `bdcd4b0a7c09b3eaabb069de40147270c0f75bad78e470eac133a7fe59811b52` |
| [`data/Zosia.vcf`](https://quoquendo.depau.eu/v1/data/Zosia.vcf) | 333 B | `442c3cac7d7aa90c5cbeda7bc0f1ede2830143f25fad4ea84247e0812d4e9a8d` |
| [`data/voices/Zosia/Zosia.32000.LTTS7Codec210.44000.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Zosia/Zosia.32000.LTTS7Codec210.44000.10.bin) | 40.0 MB | `48f74d21e51d4438e22aab1649d5376ddfe8f3af86a02441245cd678c5586be2` |
| [`data/voices/Zosia/Zosia.sde`](https://quoquendo.depau.eu/v1/data/voices/Zosia/Zosia.sde) | 1.9 MB | `f89bf8e963d2ee9d3eb8d85194f31341815009c9266d76caeed4efb2451ffe09` |
| [`data/voices/Zosia/Zosia.vde`](https://quoquendo.depau.eu/v1/data/voices/Zosia/Zosia.vde) | 442 B | `faac1fdf62b0d8154e1912f37cb5c2c22bc22e5b38935b37fcb87272c7d0b87f` |
| [`data/voices/Zosia/ZosiaGilded.gde`](https://quoquendo.depau.eu/v1/data/voices/Zosia/ZosiaGilded.gde) | 8 kB | `7078e99060e3d10dd61e2543b10b85a436310899ece50efdb5c8265ea984dd4b` |
| [`data/voices/Zosia/ZosiaGildedParalinguistics.32000.LTTS7Codec210.44000.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Zosia/ZosiaGildedParalinguistics.32000.LTTS7Codec210.44000.10.bin) | 761 kB | `d5a34c4d6de221d470b093d400eac2c6b72aaf0452de6354bbf239e8e359ca10` |
| [`data/voices/Zosia/ZosiaGildedParalinguistics.sde`](https://quoquendo.depau.eu/v1/data/voices/Zosia/ZosiaGildedParalinguistics.sde) | 6 kB | `44cdce873b1c55f258e12dcb764eb5e5c4ae4cf58e859740a577172845d937de` |
| [`data/voices/Zosia/ZosiaGildedPhrases.32000.LTTS7Codec210.44000.10.bin`](https://quoquendo.depau.eu/v1/data/voices/Zosia/ZosiaGildedPhrases.32000.LTTS7Codec210.44000.10.bin) | 1.7 MB | `c41155c7487dbecb93d114ff575dc48ddaceb0e4d829b618b8daf0b78d948403` |
| [`data/voices/Zosia/ZosiaGildedPhrases.sde`](https://quoquendo.depau.eu/v1/data/voices/Zosia/ZosiaGildedPhrases.sde) | 87 kB | `9fb3fc79a04f5a61255f44cd0b1a1b52231817786fb2efa14145134d8bcfd7a7` |

## Languages

### Catalan

| File | Size | SHA-256 |
|---|---:|---|
| [`bin/LTTS7Catalan.dll`](https://quoquendo.depau.eu/v1/bin/LTTS7Catalan.dll) | 670 kB | `78e291692f61e1a114465a58154b1f59dc32032f7ba36699f24aa19dddcd6e17` |
| [`bin/LTTS7Util.dll`](https://quoquendo.depau.eu/v1/bin/LTTS7Util.dll) | 538 kB | `43d6cf2b6d05279146a4cebd8312eaf7c3e6824b3cb7618d76525abbd998597c` |
| [`bin/LoqTTS7.dll`](https://quoquendo.depau.eu/v1/bin/LoqTTS7.dll) | 746 kB | `e8c057e0fbd85c2898a82e99a2fb9680d0ebea4b59badf2ddf654e1a4c4dcb26` |
| [`data/languages/Catalan/Catalan.lcf`](https://quoquendo.depau.eu/v1/data/languages/Catalan/Catalan.lcf) | 193 B | `7ac1dd574c9f24e7c1717f8bd08457d101f8a9827de8eaa1d339309d8f421ab0` |
| [`data/languages/Catalan/Catalan.lex`](https://quoquendo.depau.eu/v1/data/languages/Catalan/Catalan.lex) | 2 kB | `bf67352ebc05291c4dc39f193bc67d59a8da98b1dbd0e43c24b01624d71275f3` |
| [`data/languages/Catalan/CatalanCe.dbl`](https://quoquendo.depau.eu/v1/data/languages/Catalan/CatalanCe.dbl) | 274 kB | `e5e431df12615ee54575eccadc03d9aac6522a358d4cd87e9ba15c66f1a52fdf` |
| [`data/languages/Catalan/CatalanCe.lde`](https://quoquendo.depau.eu/v1/data/languages/Catalan/CatalanCe.lde) | 334 B | `9904e03ea93f48497cdeff970ed3923b6dc709b3e2e1c4c40e03827bc8a89853` |
| [`data/languages/Catalan/CatalanVa.dbl`](https://quoquendo.depau.eu/v1/data/languages/Catalan/CatalanVa.dbl) | 310 kB | `613eef655cb7646a1555a2db63efee9449ba8d0424bf46899d9e9dd763f2ac08` |
| [`data/languages/Catalan/CatalanVa.lde`](https://quoquendo.depau.eu/v1/data/languages/Catalan/CatalanVa.lde) | 362 B | `a6c2148f917933a92d2cacb7ec4c902671ddb61cce293c1439b0782ab87228df` |
| [`data/languages/Catalan/Valencian.lcf`](https://quoquendo.depau.eu/v1/data/languages/Catalan/Valencian.lcf) | 195 B | `54e01c1b827a6a9a139b9e058d3194d5fbec118e2ce03d9f7cc5a9bdcd32eae7` |

### Chinese

| File | Size | SHA-256 |
|---|---:|---|
| [`bin/LTTS7Util.dll`](https://quoquendo.depau.eu/v1/bin/LTTS7Util.dll) | 538 kB | `43d6cf2b6d05279146a4cebd8312eaf7c3e6824b3cb7618d76525abbd998597c` |
| [`bin/LoqTTS7.dll`](https://quoquendo.depau.eu/v1/bin/LoqTTS7.dll) | 746 kB | `e8c057e0fbd85c2898a82e99a2fb9680d0ebea4b59badf2ddf654e1a4c4dcb26` |
| [`data/languages/Chinese/Chinese.chd`](https://quoquendo.depau.eu/v1/data/languages/Chinese/Chinese.chd) | 923 kB | `a75ce67abfb7e1b34dc257cd92d52d09eb75536b7d9609ba9b22ec9e50cab0d4` |
| [`data/languages/Chinese/Chinese.lde`](https://quoquendo.depau.eu/v1/data/languages/Chinese/Chinese.lde) | 188 B | `0a0ca0e8149dbc443394e35f5b7a367da43b923c562d9fef818d940f96adc04b` |

### Danish

| File | Size | SHA-256 |
|---|---:|---|
| [`bin/LTTS7Danish.dll`](https://quoquendo.depau.eu/v1/bin/LTTS7Danish.dll) | 948 kB | `77d5a958aaa55b22c42109d65432e9539c6f99978b14d2cc5f24b432931f958a` |
| [`bin/LTTS7Util.dll`](https://quoquendo.depau.eu/v1/bin/LTTS7Util.dll) | 538 kB | `43d6cf2b6d05279146a4cebd8312eaf7c3e6824b3cb7618d76525abbd998597c` |
| [`bin/LoqTTS7.dll`](https://quoquendo.depau.eu/v1/bin/LoqTTS7.dll) | 746 kB | `e8c057e0fbd85c2898a82e99a2fb9680d0ebea4b59badf2ddf654e1a4c4dcb26` |
| [`data/languages/Danish/Danish.dbl`](https://quoquendo.depau.eu/v1/data/languages/Danish/Danish.dbl) | 11 kB | `41f48229a05827f61a200f04ff0fa1fe06e86f76debe32be7e27fb218c8e2c12` |
| [`data/languages/Danish/Danish.lde`](https://quoquendo.depau.eu/v1/data/languages/Danish/Danish.lde) | 293 B | `9d33be2908867dd8eff0af9e88b15ff668866291d8e4c0429a9b2e117795659b` |
| [`data/languages/Danish/Danish.rex`](https://quoquendo.depau.eu/v1/data/languages/Danish/Danish.rex) | 15 kB | `0cdd1036351fb4193b3e8823981d8b14bd3007fa8e78bd3312e3ac2ac058b85f` |

### Dutch

| File | Size | SHA-256 |
|---|---:|---|
| [`bin/LTTS7Dutch.dll`](https://quoquendo.depau.eu/v1/bin/LTTS7Dutch.dll) | 630 kB | `1e2f4fe013a0bdda435de11f9fcffd8bff4eeca7aaf7e8f57b6968e2f9415bbe` |
| [`bin/LTTS7Util.dll`](https://quoquendo.depau.eu/v1/bin/LTTS7Util.dll) | 538 kB | `43d6cf2b6d05279146a4cebd8312eaf7c3e6824b3cb7618d76525abbd998597c` |
| [`bin/LoqTTS7.dll`](https://quoquendo.depau.eu/v1/bin/LoqTTS7.dll) | 746 kB | `e8c057e0fbd85c2898a82e99a2fb9680d0ebea4b59badf2ddf654e1a4c4dcb26` |
| [`data/languages/Dutch/Dutch.dbl`](https://quoquendo.depau.eu/v1/data/languages/Dutch/Dutch.dbl) | 178 kB | `2c1fa67c553397cddeaccd96fe17965185fef822a6085ecb8b6b9f2fb718ebb4` |
| [`data/languages/Dutch/Dutch.gpr`](https://quoquendo.depau.eu/v1/data/languages/Dutch/Dutch.gpr) | 902 kB | `74e0320d3b8e79f5c2298f0fcee58e49a7153a0a57d3eab0da47f4e13936a1ba` |
| [`data/languages/Dutch/Dutch.lde`](https://quoquendo.depau.eu/v1/data/languages/Dutch/Dutch.lde) | 314 B | `51f2d8c011c1f60fa30a9036911b64f70a1ffda4e9e72bca8bca506e1a1fa50c` |
| [`data/languages/Dutch/Dutch.rex`](https://quoquendo.depau.eu/v1/data/languages/Dutch/Dutch.rex) | 4 kB | `daf7a125d40f74543cb8ff10b55b283808e9f250c4f8b5f2b2fb314cd9ebb002` |
| [`data/languages/Dutch/SIDutch.lex`](https://quoquendo.depau.eu/v1/data/languages/Dutch/SIDutch.lex) | 881 B | `d0ed580a982eabf1b46c0d3eeb6574a3fa6a8392696162135986aa7b13e0227b` |

### English

| File | Size | SHA-256 |
|---|---:|---|
| [`bin/LTTS7English.dll`](https://quoquendo.depau.eu/v1/bin/LTTS7English.dll) | 292 kB | `0379261e43e4c247d2e621dadf920f211fe0bd0bed0db6c4bbece71a6a466086` |
| [`bin/LTTS7Util.dll`](https://quoquendo.depau.eu/v1/bin/LTTS7Util.dll) | 538 kB | `43d6cf2b6d05279146a4cebd8312eaf7c3e6824b3cb7618d76525abbd998597c` |
| [`bin/LoqTTS7.dll`](https://quoquendo.depau.eu/v1/bin/LoqTTS7.dll) | 746 kB | `e8c057e0fbd85c2898a82e99a2fb9680d0ebea4b59badf2ddf654e1a4c4dcb26` |
| [`data/languages/English/American English Automotive Digits.lex`](https://quoquendo.depau.eu/v1/data/languages/English/American%20English%20Automotive%20Digits.lex) | 443 B | `7b4b1eb1e0c8fc651701693ca4b9aa7e9e31345abbd14382483fc994e677cb9a` |
| [`data/languages/English/American English Automotive Pronunciation.lex`](https://quoquendo.depau.eu/v1/data/languages/English/American%20English%20Automotive%20Pronunciation.lex) | 626 B | `8bbd36793a35bf91a49dcf0a986b2dc1fc3288a8df66a6410681dff7969f4879` |
| [`data/languages/English/American English Automotive.lex`](https://quoquendo.depau.eu/v1/data/languages/English/American%20English%20Automotive.lex) | 20 kB | `71288c13984206d02729b336d523ba235561a4f798d057a350f0af52e6354c32` |
| [`data/languages/English/American English USCities.lex`](https://quoquendo.depau.eu/v1/data/languages/English/American%20English%20USCities.lex) | 12 kB | `3bb562c1042ab1d400993a29a04a519b003fda334fb1c5e9f39c970cc01a33d5` |
| [`data/languages/English/Australian English AUCities.lex`](https://quoquendo.depau.eu/v1/data/languages/English/Australian%20English%20AUCities.lex) | 670 kB | `725320283512f63f2e9901c6655f4b59d5758378bdec6784ac885b83ffa7ea23` |
| [`data/languages/English/Australian English Names.lex`](https://quoquendo.depau.eu/v1/data/languages/English/Australian%20English%20Names.lex) | 813 kB | `4039da6f27cf89692c1647b73a6c850da587ce6fb886f41b1cd147bb62e6ce0c` |
| [`data/languages/English/Australian English Proper Names.lex`](https://quoquendo.depau.eu/v1/data/languages/English/Australian%20English%20Proper%20Names.lex) | 94 kB | `ea4f72ae554607c8300cfb83d9fd5dce8847c78eb4bf5e9c65517908f2744b99` |
| [`data/languages/English/British English Address.lex`](https://quoquendo.depau.eu/v1/data/languages/English/British%20English%20Address.lex) | 3 kB | `1dc721b41d32545887704c7ae7407f5ddf74b4ffec27e5968062cee609870d6e` |
| [`data/languages/English/British English Proper Names.lex`](https://quoquendo.depau.eu/v1/data/languages/English/British%20English%20Proper%20Names.lex) | 68 kB | `0286323f25b70417a62c8954e38a477655ff2e0fa488727b59050d32a7c8ccbb` |
| [`data/languages/English/British English USCities.lex`](https://quoquendo.depau.eu/v1/data/languages/English/British%20English%20USCities.lex) | 10 kB | `aeda8604adca16b5c12b008407b48653da91b2b83dd5b42582a7a4eb90033d5a` |
| [`data/languages/English/EnglishAu.dbl`](https://quoquendo.depau.eu/v1/data/languages/English/EnglishAu.dbl) | 156 kB | `9870f3813f2dd696ac822feb94d9be292300ecd6151458a577eb7d3161a6d9e2` |
| [`data/languages/English/EnglishAu.gpr`](https://quoquendo.depau.eu/v1/data/languages/English/EnglishAu.gpr) | 1.2 MB | `0dfba96b3a7d81c6dc81622f876ee9c022439efb5ea4fd504445f809c327afa8` |
| [`data/languages/English/EnglishAu.lde`](https://quoquendo.depau.eu/v1/data/languages/English/EnglishAu.lde) | 455 B | `85160b8a23993edad7ae0b49e88681979fbd7cf4944c1ceea7e8e4d00ee49be8` |
| [`data/languages/English/EnglishAu.lex`](https://quoquendo.depau.eu/v1/data/languages/English/EnglishAu.lex) | 5 kB | `4449c6c728c8bcb0acbd32393ffd8f3801c0f588f723b55fa5c922919790bb18` |
| [`data/languages/English/EnglishGb.dbl`](https://quoquendo.depau.eu/v1/data/languages/English/EnglishGb.dbl) | 150 kB | `78cd65282390cf7f2186bb7b1e993cdf747e8f52d15efc8f28c06165bf13847b` |
| [`data/languages/English/EnglishGb.gpr`](https://quoquendo.depau.eu/v1/data/languages/English/EnglishGb.gpr) | 1.2 MB | `0dfba96b3a7d81c6dc81622f876ee9c022439efb5ea4fd504445f809c327afa8` |
| [`data/languages/English/EnglishGb.lde`](https://quoquendo.depau.eu/v1/data/languages/English/EnglishGb.lde) | 521 B | `08c202456347d483cbe3b63757e779756d0ac45519e65e38db2a4edf6a1bf87a` |
| [`data/languages/English/EnglishGb.lex`](https://quoquendo.depau.eu/v1/data/languages/English/EnglishGb.lex) | 4 kB | `ed57d818316e431f860daf99f259ba67f7eafd69b0a0d829b77c1faf9432713d` |
| [`data/languages/English/EnglishUs.dbl`](https://quoquendo.depau.eu/v1/data/languages/English/EnglishUs.dbl) | 208 kB | `8e69acb497368aea20c39458e7f99d50ea931ae7b9d959758eb8828de4524c2d` |
| [`data/languages/English/EnglishUs.gpr`](https://quoquendo.depau.eu/v1/data/languages/English/EnglishUs.gpr) | 986 kB | `ef631be39f9a10831dfefce44b80f802e7179237684015a6be3eda7abc065ab7` |
| [`data/languages/English/EnglishUs.lde`](https://quoquendo.depau.eu/v1/data/languages/English/EnglishUs.lde) | 552 B | `db878168f667510fd138196ce201bf58f7057062361e578caa1b1a010c32c483` |
| [`data/languages/English/EnglishUs.lex`](https://quoquendo.depau.eu/v1/data/languages/English/EnglishUs.lex) | 4 kB | `c272710cafedd927f00fa4fd88da5fb8bd940d7d0623ad23692b1a4b2d6bcffc` |
| [`data/languages/English/SIEnglishGb.lex`](https://quoquendo.depau.eu/v1/data/languages/English/SIEnglishGb.lex) | 998 B | `f4d2bbde5db1c76d972d44a0bce33838ae1e4401e873c2db3fe300567d2d4b29` |
| [`data/languages/English/SIEnglishUs.lex`](https://quoquendo.depau.eu/v1/data/languages/English/SIEnglishUs.lex) | 945 B | `07b108d3f612974b904416a48fbbb52856ad2fe1909369350bf06d3da4de28e2` |
| [`data/languages/English/SIExtendedEnglishGb.lex`](https://quoquendo.depau.eu/v1/data/languages/English/SIExtendedEnglishGb.lex) | 2 kB | `45192a6d57f9a0774afd7c711de5542b5df16455535a1cf730a2df35ab017c71` |
| [`data/languages/English/SIExtendedEnglishUs.lex`](https://quoquendo.depau.eu/v1/data/languages/English/SIExtendedEnglishUs.lex) | 2 kB | `98959a38a0df8de1273ce7b96d6ae472ec40d850ce5ce93da6f3ba99b14b9989` |
| [`data/languages/English/SMSEnglishAu.lex`](https://quoquendo.depau.eu/v1/data/languages/English/SMSEnglishAu.lex) | 4 kB | `b81101dece20cd3bfdee7a57d2ad2c024072dec390bbf72fc27d78317484caa2` |
| [`data/languages/English/SMSEnglishGb.lex`](https://quoquendo.depau.eu/v1/data/languages/English/SMSEnglishGb.lex) | 4 kB | `b81101dece20cd3bfdee7a57d2ad2c024072dec390bbf72fc27d78317484caa2` |
| [`data/languages/English/SMSEnglishUs.lex`](https://quoquendo.depau.eu/v1/data/languages/English/SMSEnglishUs.lex) | 4 kB | `b81101dece20cd3bfdee7a57d2ad2c024072dec390bbf72fc27d78317484caa2` |

### French

| File | Size | SHA-256 |
|---|---:|---|
| [`bin/LTTS7French.dll`](https://quoquendo.depau.eu/v1/bin/LTTS7French.dll) | 272 kB | `9555f6cffd339e64c8c53441b21bc35eff7d5d7591e2acf78173213753a559bf` |
| [`bin/LTTS7Util.dll`](https://quoquendo.depau.eu/v1/bin/LTTS7Util.dll) | 538 kB | `43d6cf2b6d05279146a4cebd8312eaf7c3e6824b3cb7618d76525abbd998597c` |
| [`bin/LoqTTS7.dll`](https://quoquendo.depau.eu/v1/bin/LoqTTS7.dll) | 746 kB | `e8c057e0fbd85c2898a82e99a2fb9680d0ebea4b59badf2ddf654e1a4c4dcb26` |
| [`data/languages/French/French Automotive Digits.lex`](https://quoquendo.depau.eu/v1/data/languages/French/French%20Automotive%20Digits.lex) | 445 B | `1718b170b9fdecd2e2bec41a047612c6538441abafd96b99fd6428b10c697f90` |
| [`data/languages/French/French Automotive.lex`](https://quoquendo.depau.eu/v1/data/languages/French/French%20Automotive.lex) | 6 kB | `557654f44d7fd3d062f2a724c9302c84712cadf91affefd30db41eec09fb0b4c` |
| [`data/languages/French/French.dbl`](https://quoquendo.depau.eu/v1/data/languages/French/French.dbl) | 141 kB | `bad680c2bd83a519dec337d17c775b4fb5caf9cb0c27f00798cdf6ccc9ca02d0` |
| [`data/languages/French/French.lde`](https://quoquendo.depau.eu/v1/data/languages/French/French.lde) | 361 B | `1359f9931f6adb2bba77a0b96ab749c074810485c944ea63a50b82e015459ef1` |
| [`data/languages/French/French.rex`](https://quoquendo.depau.eu/v1/data/languages/French/French.rex) | 2 kB | `d3cd8a5f0f262bac408244c37a31a5beb213dce4d69c7b976034714849a133a0` |
| [`data/languages/French/FrenchCA.dbl`](https://quoquendo.depau.eu/v1/data/languages/French/FrenchCA.dbl) | 150 kB | `4fa8aeda5c7af2ffe6b90bb76ce73c39ae68241519b187c6b43790d70ff1cbbd` |
| [`data/languages/French/FrenchCA.lde`](https://quoquendo.depau.eu/v1/data/languages/French/FrenchCA.lde) | 275 B | `e50074e764c61a25483268756301e67d5ad373aa4d958fac3639570099538ce7` |
| [`data/languages/French/SIExtendedFrench.lex`](https://quoquendo.depau.eu/v1/data/languages/French/SIExtendedFrench.lex) | 1 kB | `a851f5f9a8d635e44fbac71ad6ee3e4223cfc8974ab4354aa263e9c77646f935` |
| [`data/languages/French/SIFrench.lex`](https://quoquendo.depau.eu/v1/data/languages/French/SIFrench.lex) | 2 kB | `a1c9d8347e6ed2c90c92e1cab0a69766f2bbf0342ea3b1ff5dce58cc5623164a` |
| [`data/languages/French/SMSFrench.lex`](https://quoquendo.depau.eu/v1/data/languages/French/SMSFrench.lex) | 5 kB | `e4ac4733d5491399aebffe5481625cd07ffe63186f7c50818fea7e65cfbd28fe` |

### Galician

| File | Size | SHA-256 |
|---|---:|---|
| [`bin/LTTS7Galician.dll`](https://quoquendo.depau.eu/v1/bin/LTTS7Galician.dll) | 152 kB | `b3c276dbf3418447c61e1ffed97806af6750971f386b07823738959b4bed794f` |
| [`bin/LTTS7Util.dll`](https://quoquendo.depau.eu/v1/bin/LTTS7Util.dll) | 538 kB | `43d6cf2b6d05279146a4cebd8312eaf7c3e6824b3cb7618d76525abbd998597c` |
| [`bin/LoqTTS7.dll`](https://quoquendo.depau.eu/v1/bin/LoqTTS7.dll) | 746 kB | `e8c057e0fbd85c2898a82e99a2fb9680d0ebea4b59badf2ddf654e1a4c4dcb26` |
| [`data/languages/Galician/Galician.dbl`](https://quoquendo.depau.eu/v1/data/languages/Galician/Galician.dbl) | 160 kB | `fbf1a2cdd4f9454ac814e69b8728084bbfa35b653e010731aa6236daa11ca322` |
| [`data/languages/Galician/Galician.lcf`](https://quoquendo.depau.eu/v1/data/languages/Galician/Galician.lcf) | 198 B | `4aa804198c0887cf37bf65b87ec005ef3d71fa90015ee324aecfefbe3c2b6526` |
| [`data/languages/Galician/Galician.lde`](https://quoquendo.depau.eu/v1/data/languages/Galician/Galician.lde) | 309 B | `49ddb560abb87fb41e78337bd33de6b5dcb11b9b9b081902171e34e80776a6b1` |
| [`data/languages/Galician/Galician.rex`](https://quoquendo.depau.eu/v1/data/languages/Galician/Galician.rex) | 2 kB | `182f745d3817addb79be511a89e897eeb1aecb5447bc670e92cbf2bf8a4ee4d6` |

### German

| File | Size | SHA-256 |
|---|---:|---|
| [`bin/LTTS7German.dll`](https://quoquendo.depau.eu/v1/bin/LTTS7German.dll) | 1.5 MB | `4121468a50278e0135dcc8c16944765fcb241806e11ebd2c33a1ee299603ac2f` |
| [`bin/LTTS7Util.dll`](https://quoquendo.depau.eu/v1/bin/LTTS7Util.dll) | 538 kB | `43d6cf2b6d05279146a4cebd8312eaf7c3e6824b3cb7618d76525abbd998597c` |
| [`bin/LoqTTS7.dll`](https://quoquendo.depau.eu/v1/bin/LoqTTS7.dll) | 746 kB | `e8c057e0fbd85c2898a82e99a2fb9680d0ebea4b59badf2ddf654e1a4c4dcb26` |
| [`data/languages/German/German Automotive.lex`](https://quoquendo.depau.eu/v1/data/languages/German/German%20Automotive.lex) | 4 kB | `8c7d75f4ce4b3ec151d52f2f9b971894f2dd9c8163dce24ddf73a2f1d0bd0813` |
| [`data/languages/German/German Cities.lex`](https://quoquendo.depau.eu/v1/data/languages/German/German%20Cities.lex) | 1 kB | `15f3135d29d6c66554b8be4a21f6ff48b94478577c0457c7f850963dff6e861e` |
| [`data/languages/German/German.dbl`](https://quoquendo.depau.eu/v1/data/languages/German/German.dbl) | 301 kB | `6a5d5c5d8400cc3a2976e12fc16740b3bb5a956c0e3e46aece0f5c1eef0d0ed2` |
| [`data/languages/German/German.lde`](https://quoquendo.depau.eu/v1/data/languages/German/German.lde) | 411 B | `9098b22697d83a6388a0af4cd39c805e53ad64dfb8bd9a52e0e6ae92588f4d5e` |
| [`data/languages/German/German.lex`](https://quoquendo.depau.eu/v1/data/languages/German/German.lex) | 6 kB | `df894b04546bc5fbd85fe32d90a7bf673fd15ba45e9ac1850429d6825cfa7568` |
| [`data/languages/German/German.rex`](https://quoquendo.depau.eu/v1/data/languages/German/German.rex) | 4 kB | `b9810999109f0b11251e5484d8d1d9a798470bc8e2b26a6151e685d331802de6` |
| [`data/languages/German/SIGerman.lex`](https://quoquendo.depau.eu/v1/data/languages/German/SIGerman.lex) | 1 kB | `955f35bf946a9d925e65290c744387f87dc88e57adf3011376e1c4283fe65b7a` |
| [`data/languages/German/SMSGerman.lex`](https://quoquendo.depau.eu/v1/data/languages/German/SMSGerman.lex) | 3 kB | `00948e347f80dbb1b9a813f3476ecaa24292f727ddf2269b270d66f31b6a9fd3` |

### Greek

| File | Size | SHA-256 |
|---|---:|---|
| [`bin/LTTS7Greek.dll`](https://quoquendo.depau.eu/v1/bin/LTTS7Greek.dll) | 156 kB | `36f855979d3e5ae414f91aca07565175990eec09481d10346577f809faf16882` |
| [`bin/LTTS7Util.dll`](https://quoquendo.depau.eu/v1/bin/LTTS7Util.dll) | 538 kB | `43d6cf2b6d05279146a4cebd8312eaf7c3e6824b3cb7618d76525abbd998597c` |
| [`bin/LoqTTS7.dll`](https://quoquendo.depau.eu/v1/bin/LoqTTS7.dll) | 746 kB | `e8c057e0fbd85c2898a82e99a2fb9680d0ebea4b59badf2ddf654e1a4c4dcb26` |
| [`data/languages/Greek/Greek.dbl`](https://quoquendo.depau.eu/v1/data/languages/Greek/Greek.dbl) | 60 kB | `0f5fc87fc9feef1d9c43d513efcd53a1f99f6a0f54fde3d1774195a44ef2cf53` |
| [`data/languages/Greek/Greek.lde`](https://quoquendo.depau.eu/v1/data/languages/Greek/Greek.lde) | 311 B | `31e39008ee135cd29f817ed1d2a3db84cc5bef99424be71450b901aec3d9f4aa` |
| [`data/languages/Greek/Greek.lex`](https://quoquendo.depau.eu/v1/data/languages/Greek/Greek.lex) | 5 kB | `2b4bb0e92a955ff3d801942282cfd45d1fa66f83fb2eee29db0d8c9dcbeb4825` |
| [`data/languages/Greek/SIGreek.lex`](https://quoquendo.depau.eu/v1/data/languages/Greek/SIGreek.lex) | 3 kB | `a48c6e41c5f4d12577eb028ed8daac301697fb66e48783bee70afd04c7e1723d` |

### Italian

| File | Size | SHA-256 |
|---|---:|---|
| [`bin/LTTS7Italian.dll`](https://quoquendo.depau.eu/v1/bin/LTTS7Italian.dll) | 590 kB | `283ed68310ea4726a1621b1b74f5c642505bc2e87546080ce893219af42f1430` |
| [`bin/LTTS7Util.dll`](https://quoquendo.depau.eu/v1/bin/LTTS7Util.dll) | 538 kB | `43d6cf2b6d05279146a4cebd8312eaf7c3e6824b3cb7618d76525abbd998597c` |
| [`bin/LoqTTS7.dll`](https://quoquendo.depau.eu/v1/bin/LoqTTS7.dll) | 746 kB | `e8c057e0fbd85c2898a82e99a2fb9680d0ebea4b59badf2ddf654e1a4c4dcb26` |
| [`data/languages/Italian/Italian.lde`](https://quoquendo.depau.eu/v1/data/languages/Italian/Italian.lde) | 310 B | `a333d6a2d45cbf2b4a37c411e2e2c06ab829d73ec7ade46ba61d6faf85e62ebc` |
| [`data/languages/Italian/Italian.lex`](https://quoquendo.depau.eu/v1/data/languages/Italian/Italian.lex) | 4 kB | `8e06fba9ced5dbd94f0067b64f9f0fca257368b2afb248995551e18e10570fce` |
| [`data/languages/Italian/SIItalian.lex`](https://quoquendo.depau.eu/v1/data/languages/Italian/SIItalian.lex) | 1 kB | `7ded3cdb64c6a74c4cc32a89d3d4ff17a47d55381b500d31ef764e182fc2d751` |
| [`data/languages/Italian/SMSItalian.lex`](https://quoquendo.depau.eu/v1/data/languages/Italian/SMSItalian.lex) | 7 kB | `fb7fb35c852caf98a1394cf9f3201d5d738a702f7e755efdb0364c6f87d2aae0` |

### Polish

| File | Size | SHA-256 |
|---|---:|---|
| [`bin/LTTS7Polish.dll`](https://quoquendo.depau.eu/v1/bin/LTTS7Polish.dll) | 279 kB | `23a7fb2e14dda2df9c878360e87207cd869d659f8efb7a230434bfe6d123ce6e` |
| [`bin/LTTS7Util.dll`](https://quoquendo.depau.eu/v1/bin/LTTS7Util.dll) | 538 kB | `43d6cf2b6d05279146a4cebd8312eaf7c3e6824b3cb7618d76525abbd998597c` |
| [`bin/LoqTTS7.dll`](https://quoquendo.depau.eu/v1/bin/LoqTTS7.dll) | 746 kB | `e8c057e0fbd85c2898a82e99a2fb9680d0ebea4b59badf2ddf654e1a4c4dcb26` |
| [`data/languages/Polish/Polish.dbl`](https://quoquendo.depau.eu/v1/data/languages/Polish/Polish.dbl) | 20 kB | `a8af36ff86745f953b76c0d12c0c8c093601eb9d475546dd57e7f246b42ee088` |
| [`data/languages/Polish/Polish.lde`](https://quoquendo.depau.eu/v1/data/languages/Polish/Polish.lde) | 307 B | `31488d002777175af19cb623dd59732581e0e695de566520aa9930189d6ab580` |
| [`data/languages/Polish/Polish.rex`](https://quoquendo.depau.eu/v1/data/languages/Polish/Polish.rex) | 13 kB | `bcd0e683812d9661bd54b8c6ebd16979e8fa67d0c8ede8bf73349e29bd81c550` |
| [`data/languages/Polish/SIPolish.lex`](https://quoquendo.depau.eu/v1/data/languages/Polish/SIPolish.lex) | 2 kB | `046d50ff344346a220b51764e823fe61038cc119e83fc85fcee1e075a5678a54` |

### Portuguese

| File | Size | SHA-256 |
|---|---:|---|
| [`bin/LTTS7Portuguese.dll`](https://quoquendo.depau.eu/v1/bin/LTTS7Portuguese.dll) | 202 kB | `cb6e5e4b89986a7a5a11a2ebd096e580b1f67ac954e49599836f764a4b58667f` |
| [`bin/LTTS7Util.dll`](https://quoquendo.depau.eu/v1/bin/LTTS7Util.dll) | 538 kB | `43d6cf2b6d05279146a4cebd8312eaf7c3e6824b3cb7618d76525abbd998597c` |
| [`bin/LoqTTS7.dll`](https://quoquendo.depau.eu/v1/bin/LoqTTS7.dll) | 746 kB | `e8c057e0fbd85c2898a82e99a2fb9680d0ebea4b59badf2ddf654e1a4c4dcb26` |
| [`data/languages/Portuguese/Portuguese.lex`](https://quoquendo.depau.eu/v1/data/languages/Portuguese/Portuguese.lex) | 4 kB | `c869062ce760fb8ecad8bffe6a6fa5407fe30aec06d150e471a1a3a765090674` |
| [`data/languages/Portuguese/PortugueseBr.dbl`](https://quoquendo.depau.eu/v1/data/languages/Portuguese/PortugueseBr.dbl) | 97 kB | `1fe2527232afe5a487ffcc6c64bcd222bd684fb384d68fe6cb4d362a30134248` |
| [`data/languages/Portuguese/PortugueseBr.lde`](https://quoquendo.depau.eu/v1/data/languages/Portuguese/PortugueseBr.lde) | 362 B | `82e6d8e55e5076b9d38c4bf832146a2c58220a5e7aa4f0f013399d803ef83799` |
| [`data/languages/Portuguese/PortuguesePt.dbl`](https://quoquendo.depau.eu/v1/data/languages/Portuguese/PortuguesePt.dbl) | 177 kB | `d44606b2f2ac9119a7dda9c27a6355c8134b22066e2f5e0fcd0f08f54775952b` |
| [`data/languages/Portuguese/PortuguesePt.lde`](https://quoquendo.depau.eu/v1/data/languages/Portuguese/PortuguesePt.lde) | 363 B | `53727a1a8422ca4c22fa8cf5e50e721c55dcf212f0a4ff8b1edbd315a4f69024` |
| [`data/languages/Portuguese/SIPortuguese.lex`](https://quoquendo.depau.eu/v1/data/languages/Portuguese/SIPortuguese.lex) | 2 kB | `27e0ecab60acf63e66098a7e48ba9c5e3a454fa4fb0e3502767d7168d65cbb61` |

### Russian

| File | Size | SHA-256 |
|---|---:|---|
| [`bin/LTTS7Russian.dll`](https://quoquendo.depau.eu/v1/bin/LTTS7Russian.dll) | 1.2 MB | `fedcda9c25ff78bdf217e71dabd420b009d89a48613068c552d2438370e58b41` |
| [`bin/LTTS7Util.dll`](https://quoquendo.depau.eu/v1/bin/LTTS7Util.dll) | 538 kB | `43d6cf2b6d05279146a4cebd8312eaf7c3e6824b3cb7618d76525abbd998597c` |
| [`bin/LoqTTS7.dll`](https://quoquendo.depau.eu/v1/bin/LoqTTS7.dll) | 746 kB | `e8c057e0fbd85c2898a82e99a2fb9680d0ebea4b59badf2ddf654e1a4c4dcb26` |
| [`data/languages/Russian/Russian.lde`](https://quoquendo.depau.eu/v1/data/languages/Russian/Russian.lde) | 304 B | `17b7a903b85ccb479925596e8be419f34462c154c5acc097d9d06d01fba4fe12` |
| [`data/languages/Russian/Russian.lex`](https://quoquendo.depau.eu/v1/data/languages/Russian/Russian.lex) | 57 kB | `df0dd1b185a9b96ce9ddb2acd546535d16c7bbf324b726f7e0c7d4bd31f05546` |

### Spanish

| File | Size | SHA-256 |
|---|---:|---|
| [`bin/LTTS7Spanish.dll`](https://quoquendo.depau.eu/v1/bin/LTTS7Spanish.dll) | 189 kB | `c81e0e86f6ba817ee6c2ca9703416da5f9c378dc9d4386fd505c8099098387ad` |
| [`bin/LTTS7Util.dll`](https://quoquendo.depau.eu/v1/bin/LTTS7Util.dll) | 538 kB | `43d6cf2b6d05279146a4cebd8312eaf7c3e6824b3cb7618d76525abbd998597c` |
| [`bin/LoqTTS7.dll`](https://quoquendo.depau.eu/v1/bin/LoqTTS7.dll) | 746 kB | `e8c057e0fbd85c2898a82e99a2fb9680d0ebea4b59badf2ddf654e1a4c4dcb26` |
| [`data/languages/Spanish/American Spanish Automotive Ordinals.lex`](https://quoquendo.depau.eu/v1/data/languages/Spanish/American%20Spanish%20Automotive%20Ordinals.lex) | 4 kB | `64c8edae9cd705e45e27a3d07077fb10553e48a4e9dc84f0a8bdad08d56edac1` |
| [`data/languages/Spanish/American Spanish Automotive Pronunciation.lex`](https://quoquendo.depau.eu/v1/data/languages/Spanish/American%20Spanish%20Automotive%20Pronunciation.lex) | 12 kB | `30b911f91f3d22e8ab16f5f999fd4d328d5d8536ca1f6a8ab99ae28f440def56` |
| [`data/languages/Spanish/American Spanish Automotive.lex`](https://quoquendo.depau.eu/v1/data/languages/Spanish/American%20Spanish%20Automotive.lex) | 16 kB | `1a86be1ef17d4aabee42eb8c28d8530dff4438fa4ff308e6e2461cc94b167a1b` |
| [`data/languages/Spanish/SISpanish.lex`](https://quoquendo.depau.eu/v1/data/languages/Spanish/SISpanish.lex) | 2 kB | `381d1a84b7ddfa406340ffdf2aff6ea9d7c150f9cbd20567199fc68a0a646551` |
| [`data/languages/Spanish/Spanish Automotive.lex`](https://quoquendo.depau.eu/v1/data/languages/Spanish/Spanish%20Automotive.lex) | 5 kB | `f42bab65d1bfc25fb71edbb64f2c55188f84577087fbfabb345df0fc0b1168ff` |
| [`data/languages/Spanish/Spanish.lex`](https://quoquendo.depau.eu/v1/data/languages/Spanish/Spanish.lex) | 4 kB | `774c11eab468b4bd460a02680b96a666e5e785a55091cf169798ca870c3cb72f` |
| [`data/languages/Spanish/SpanishAm.dbl`](https://quoquendo.depau.eu/v1/data/languages/Spanish/SpanishAm.dbl) | 255 kB | `25d5d6e3f7c781ae2882146e13f5da2522669e00c627ef4f5d0d2bd96f8d4bf4` |
| [`data/languages/Spanish/SpanishAm.lde`](https://quoquendo.depau.eu/v1/data/languages/Spanish/SpanishAm.lde) | 376 B | `234bac5e50ff2f5a02a45987ef98bdb133268c033bee05dce34265c93de32b1a` |
| [`data/languages/Spanish/SpanishAr.dbl`](https://quoquendo.depau.eu/v1/data/languages/Spanish/SpanishAr.dbl) | 255 kB | `323c6834c30707b1895c36b5e171a284461e1fad6e347e609c6e0ea378ac194f` |
| [`data/languages/Spanish/SpanishAr.lde`](https://quoquendo.depau.eu/v1/data/languages/Spanish/SpanishAr.lde) | 359 B | `17c6e4a313dc6bace34686d92a0675aaa36d218445149049669d6444bbefb896` |
| [`data/languages/Spanish/SpanishCl.dbl`](https://quoquendo.depau.eu/v1/data/languages/Spanish/SpanishCl.dbl) | 255 kB | `33b36a102b444ea972dda5d2eaf032f6cb3c1930eeea184fb53af1f797cacd2e` |
| [`data/languages/Spanish/SpanishCl.lde`](https://quoquendo.depau.eu/v1/data/languages/Spanish/SpanishCl.lde) | 347 B | `37361bc210d50301f7af8866ba198695cca7d6b2af52f6dd7c3e5158e9edd242` |
| [`data/languages/Spanish/SpanishEs.dbl`](https://quoquendo.depau.eu/v1/data/languages/Spanish/SpanishEs.dbl) | 255 kB | `2e1bafa5ca4e55e7f0cfff5867cd6890b2afb3ea3d474a540620ad30c92abda7` |
| [`data/languages/Spanish/SpanishEs.lde`](https://quoquendo.depau.eu/v1/data/languages/Spanish/SpanishEs.lde) | 395 B | `e4721ef931673dab42e1ba38b44961cf2d68ed99c7b6d7160ddd372facf7a538` |
| [`data/languages/Spanish/SpanishMx.dbl`](https://quoquendo.depau.eu/v1/data/languages/Spanish/SpanishMx.dbl) | 255 kB | `779368310da47231faa7ae0be8027fe3518c8cee5c4e7e603b435137c6c09a9a` |
| [`data/languages/Spanish/SpanishMx.lde`](https://quoquendo.depau.eu/v1/data/languages/Spanish/SpanishMx.lde) | 346 B | `646626673652225fd531bbd0d3653c3a1a5bb60d6a51c2cf1add45274ffecc82` |

### Swedish

| File | Size | SHA-256 |
|---|---:|---|
| [`bin/LTTS7Swedish.dll`](https://quoquendo.depau.eu/v1/bin/LTTS7Swedish.dll) | 816 kB | `ed95e5350954ffbeb7d50a473de5211330f9b130a2b77fed1b6528e745729df7` |
| [`bin/LTTS7Util.dll`](https://quoquendo.depau.eu/v1/bin/LTTS7Util.dll) | 538 kB | `43d6cf2b6d05279146a4cebd8312eaf7c3e6824b3cb7618d76525abbd998597c` |
| [`bin/LoqTTS7.dll`](https://quoquendo.depau.eu/v1/bin/LoqTTS7.dll) | 746 kB | `e8c057e0fbd85c2898a82e99a2fb9680d0ebea4b59badf2ddf654e1a4c4dcb26` |
| [`data/languages/Swedish/Swedish.dbl`](https://quoquendo.depau.eu/v1/data/languages/Swedish/Swedish.dbl) | 20 kB | `dc4528b6be90116ab11cbbf051ec37f28bc815593b6292b3d677361f67fc49e0` |
| [`data/languages/Swedish/Swedish.lde`](https://quoquendo.depau.eu/v1/data/languages/Swedish/Swedish.lde) | 330 B | `ad33365ba24fa87a7ed54a866fb45e2f3909d271ff49105f673d04e735e573ca` |
| [`data/languages/Swedish/Swedish.rex`](https://quoquendo.depau.eu/v1/data/languages/Swedish/Swedish.rex) | 2 kB | `04a1ab5d80c6ed70d2aec18159912e5780b6033a87f4b3594243f9fd005f1c9f` |

