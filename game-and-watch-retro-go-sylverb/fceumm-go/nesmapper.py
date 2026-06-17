#!/usr/bin/env python3
import os
import sys
import zlib
import math

#Use the same database as fceumm to always have same mapper
ChecksumDict = {
# CRC32   : ['mapperNum','SubMapper','Mirroring','hasBattery','prgRam','chrRam','region','extra flags']
"b17574f3": [    1,      -1,      -1, 1,    0x70,      -1,      -1, -1], # AD&D Heroes of the Lance
"5de61639": [    1,      -1,      -1, 1,    0x70,    0x07,      -1, -1], # AD&D Hillsfar
"2545214c": [    1,      -1,      -1, 1,    0x70,      -1,      -1, -1], # DW
"3b3f88f0": [    1,      -1,      -1, 1,    0x70,      -1,      -1, -1], # DW
"8c5a784e": [    1,      -1,      -1, 1,    0x70,    0x07,      -1, -1], # DW 2
"506e259d": [    1,      -1,      -1, 1,    0x70,    0x07,      -1, -1], # DW 4
"a86a5318": [    1,      -1,      -1, 1,    0x70,    0x07,      -1, -1], # Dw 3
"45f03d2e": [    1,      -1,      -1, 1,    0x70,      -1,      -1, -1], # Faria
"b8b88130": [    1,      -1,      -1, 1,    0x70,    0x07,      -1, -1], # Final Fantasy (FFE Hack)
"cebd2a31": [    1,      -1,      -1, 1,    0x70,    0x07,      -1, -1], # Final Fantasy
"d29db3c7": [    1,      -1,      -1, 1,    0x70,    0x07,      -1, -1], # Final Fantasy 2
"466efdc2": [    1,      -1,      -1, 1,    0x70,    0x07,      -1, -1], # Final Fantasy J
"eaf7ed72": [    1,      -1,      -1, 1,    0x70,    0x07,      -1, -1], # Legend of Zelda
"3fe272fb": [    1,      -1,      -1, 1,    0x70,    0x07,      -1, -1], # Legend of Zelda
"ba322865": [    1,      -1,      -1, 1,    0x70,      -1,      -1, -1], # Zelda 2
"25952141": [    4,      -1,      -1, 1,      -1,      -1,      -1, -1], # AD&D Pool of Radiance
"1335cb05": [    4,      -1,      -1, 1,      -1,      -1,      -1, -1], # Crystalis
"57e220d0": [    4,      -1,      -1, 1,      -1,      -1,      -1, -1], # Final Fantasy 3
"889129cb": [    4,      -1,      -1, 1,      -1,      -1,      -1, -1], # Startropics
"d054ffb0": [    4,      -1,      -1, 1,      -1,      -1,      -1, -1], # Startropics 2
"b5ff71ab": [   19,      -1,      -1, 1,      -1,      -1,      -1, -1], # Battle Fleet
"0c1792da": [   19,      -1,      -1, 1,      -1,      -1,      -1, -1], # Famista '90
"47c2020b": [   19,      -1,      -1, 1,      -1,      -1,      -1, -1], # Hydlide 3
"bc11e61a": [   19,      -1,      -1, 1,      -1,      -1,      -1, -1], # Kaijuu Monogatari
"ace56f39": [   19,      -1,      -1, 1,      -1,      -1,      -1, -1], # Mindseeker
"e1383deb": [   26,      -1,      -1, 1,      -1,      -1,      -1, -1], # Mouryou Senki Madara
"de9c9c64": [   80,      -1,      -1, 1,      -1,      -1,      -1, -1], # Kyonshiizu 2
"0e1683c5": [   80,      -1,      -1, 1,      -1,      -1,      -1, -1], # Mirai Shinwa Jarvas

"af5d7aa2": [    0,      -1,       0, 0,      -1,      -1,      -1, -1], # Clu Clu Land (W) [o3].nes
"cfb224e6": [  222,      -1,       0, 0,      -1,      -1,      -1, -1], # Dragon Ninja (J) (PRG0) [p1][!].nes
"ad9c63e2": [   70,      -1,       1, 0,      -1,      -1,      -1, -1], # Space Shadow (Japan).nes
"e1526228": [  206,      -1,       1, 0,      -1,      -1,      -1, -1], # Ki no Bouken - The Quest of Ki (Japan).nes
"af5d7aa2": [    0,      -1,       0, 0,      -1,      -1,      -1, -1], # Clu Clu Land (W) [o3].nes
"cfb224e6": [  222,      -1,       0, 0,      -1,      -1,      -1, -1], # Dragon Ninja (J) (PRG0) [p1][!].nes
"fcdaca80": [    0,      -1,       0, 0,      -1,      -1,      -1, -1], # Elevator Action (Japan).nes
"c05a365b": [    0,      -1,       0, 0,      -1,      -1,      -1, -1], # Chou Fuyuu Yousai Exed Exes (Japan).nes
"c4c3949a": [    0,      -1,       0, 0,      -1,      -1,      -1, -1], # Mario Bros. (World).nes
"32fa246f": [    0,      -1,       0, 0,      -1,      -1,      -1, -1], # Tag Team Pro-Wrestling (Japan).nes
"43d30c2f": [    0,      -1,       0, 0,      -1,      -1,      -1, -1], # Ms. Pac-Man (USA) (Tengen) (Unl).nes
"b3c30bea": [    0,      -1,       0, 0,      -1,      -1,      -1, -1], # Xevious (Japan) (En).nes
"e492d45a": [    0,      -1,       0, 0,      -1,      -1,      -1, -1], # Zippy Race (Japan).nes
"24598791": [    0,      -1,       1, 0,      -1,      -1,      -1, -1], # Duck Hunt (World).nes
"49aeb3a6": [    0,      -1,       1, 0,      -1,      -1,      -1, -1], # Excitebike (Japan, USA).nes
"b8535ca3": [    0,      -1,       1, 1,      -1,      -1,      -1, -1], # Mahjong (Japan).nes
"330de468": [    0,      -1,       1, 0,      -1,      -1,      -1, -1], # Obake no Q Tarou - Wanwan Panic (Japan).nes
"e28f2596": [    0,      -1,       1, 0,      -1,      -1,      -1, -1], # Pac-Land (J) [b2].nes
"5112dc21": [    0,      -1,       1, 0,      -1,      -1,      -1, -1], # Wild Gunman (World) (Rev 1).nes
"d8ee7669": [    1,      -1,       8, 0,      -1,      -1,      -1, -1], # Adventures of Rad Gravity, The (USA).nes
"5b837e8d": [    1,      -1,       8, 0,      -1,      -1,      -1, -1], # Alien Syndrome (Japan).nes
"37ba3261": [    1,      -1,       8, 0,      -1,      -1,      -1, -1], # Back to the Future Part II & III (USA).nes
"5b6ca654": [    1,      -1,       8, 0,      -1,      -1,      -1, -1], # Barbie (USA).nes
"61a852ea": [    1,      -1,       8, 1,    0x70,      -1,      -1, -1], # Battle Stadium - Senbatsu Pro Yakyuu (Japan).nes
"f6fa4453": [    1,      -1,       8, 0,      -1,      -1,      -1, -1], # Bigfoot (U) [b4].nes
"391aa1b8": [    1,      -1,       8, 1,    0x70,    0x07,      -1, -1], # Bloody Warriors - Shan-Go no Gyakushuu (Japan).nes
"a5e8d2cd": [    1,      -1,       8, 0,      -1,      -1,      -1, -1], # BreakThru (USA).nes
"3f56a392": [    1,      -1,       8, 0,      -1,    0x07,      -1, -1], # Captain ED (Japan).nes
"078ced30": [    1,      -1,       8, 1,    0x07,      -1,      -1, -1], # Choujin - Ultra Baseball (Japan).nes
"fe364be5": [    1,      -1,       8, 1,    0x70,    0x07,      -1, -1], # Deep Dungeon IV - Kuro no Youjutsushi (Japan).nes
"57c12280": [    1,      -1,       8, 0,      -1,      -1,      -1, -1], # Demon Sword (U) [b1].nes
"d09b74dc": [    1,      -1,       8, 0,      -1,      -1,      -1, -1], # Great Tank (Japan).nes
"e8baa782": [    1,      -1,       8, 1,    0x70,      -1,      -1, -1], # Gunhed - Aratanaru Tatakai (Japan).nes
"970bd9c2": [    1,      -1,       8, 1,    0x70,    0x07,      -1, -1], # Hanjuku Eiyuu (J) [b1].nes
"cd7a2fd7": [    1,      -1,       8, 1,    0x70,    0x07,      -1, -1], # Hanjuku Hero (Japan).nes
"63469396": [    1,      -1,       8, 1,    0x70,    0x07,      -1, -1], # Hokuto no Ken 4 - Shichisei Haken Den - Hokuto Shinken no Kanata e (Japan).nes
"e94d5181": [    1,      -1,       8, 1,    0x70,    0x07,      -1, -1], # Future Wars - Mirai Senshi Lios (Japan).nes
"7156cb4d": [    1,      -1,       8, 0,    0x70,      -1,      -1, -1], # Muppet Adventure - Chaos at the Carnival (USA).nes
"70f67ab7": [    1,      -1,       8, 1,    0x70,      -1,      -1, -1], # Musashi no Bouken (Japan).nes
"958e4bae": [    1,      -1,       8, 0,      -1,      -1,      -1, -1], # Orb-3D (USA).nes
"291bcd7d": [    1,      -1,       8, 0,      -1,    0x07,      -1, -1], # Pachio-kun 2 (Japan).nes
"a9a4ea4c": [    1,      -1,       8, 1,    0x70,      -1,      -1, -1], # Satomi Hakkenden (J) [b2].nes
"cc3544b0": [    1,      -1,       8, 0,      -1,      -1,      -1, -1], # Triathron, The (Japan).nes
"52ab2d17": [    1,      -1,       8, 0,      -1,      -1,      -1, -1], # Toukyou Pachi-Slot Adventure (Japan).nes
"934db14a": [    1,      -1,      -1, 0,      -1,      -1,      -1, -1], # All-Pro Basketball (USA).nes
"f74dfc91": [    1,      -1,      -1, 0,      -1,      -1,      -1, -1], # Win, Lose or Draw (USA).nes
"cfe02ada": [    1,      -1,      -1, 0,      -1,      -1,       1, -1], # Darkman (Europe).nes
"1a71fd06": [    1,      -1,       0, 0,      -1,    0x07,      -1, -1], # Kujaku Ou.nes

# MMC1 games with more than 8K wram
"2225c20f": [    1,      -1,      -1, 1,    0x77,    0x07,      -1, -1], # Genghis Khan
"fb69743a": [    1,      -1,      -1, 1,    0x77,    0x07,      -1, -1], # ""        "" (J)
"4642dda6": [    1,      -1,      -1, 1,    0x77,    0x07,      -1, -1], # Nobunaga's Ambition
"3f7ad415": [    1,      -1,      -1, 1,    0x77,    0x07,      -1, -1], # ""        "" (J) (PRG0)
"2b11e0b0": [    1,      -1,      -1, 1,    0x77,    0x07,      -1, -1], # ""        "" (J) (PRG1)
"c6182024": [    1,      -1,      -1, 1,    0x77,    0x07,      -1, -1], # Romance of the 3 Kingdoms
"abbf7217": [    1,      -1,      -1, 1,    0x77,    0x07,      -1, -1], # ""        "" (J) (PRG0) or Sangokushi 
"ccf35c02": [    1,      -1,      -1, 1,    0x77,    0x07,      -1, -1], # ""        "" (J) (PRG1)
"b8747abf": [    1,      -1,      -1, 1,    0x90,    0x07,      -1, -1], # Best Play Pro Yakyuu Special (J) (PRG0)
"c3de7c69": [    1,      -1,      -1, 1,    0x90,    0x07,      -1, -1], # ""        "" (J) (PRG1)
"c9556b36": [    1,      -1,      -1, 1,    0x90,    0x07,      -1, -1], # Final Fantasy 1+2

"e6a477b2": [    2,      -1,       1, 0,      -1,      -1,      -1, -1], # 3-D WorldRunner (USA).nes
"9ea1dc76": [    2,      -1,       0, 0,      -1,      -1,      -1, -1], # Rainbow Islands (USA).nes
"6d65cac6": [    2,      -1,       0, 0,      -1,      -1,      -1, -1], # Terra Cresta (Japan).nes
"e1b260da": [    2,      -1,       1, 0,      -1,      -1,      -1, -1], # Argos no Senshi - Hachamecha Daishingeki (Japan).nes
"1d0f4d6b": [    2,      -1,       1, 0,      -1,      -1,      -1, -1], # Black Bass, The (USA).nes
"266ce198": [    2,      -1,       1, 0,      -1,      -1,      -1, -1], # City Adventure Touch - Mystery of Triangle (J) [b1].nes
"804f898a": [    2,      -1,       1, 0,      -1,      -1,      -1, -1], # Dragon Unit (Japan).nes
"55773880": [    2,      -1,       1, 0,      -1,      -1,      -1, -1], # Adventures of Gilligan's Island, The (USA).nes
"6e0eb43e": [    2,      -1,       1, 0,      -1,      -1,      -1, -1], # Puss 'n Boots - Pero's Great Adventure (USA).nes
"2bb6a0f8": [    2,      -1,       1, 0,      -1,      -1,      -1, -1], # Sherlock Holmes - Hakushaku Reijou Yuukai Jiken (Japan).nes
"28c11d24": [    2,      -1,       1, 0,      -1,      -1,      -1, -1], # Sukeban Deka 3 (J) [b1].nes
"02863604": [    2,      -1,       1, 0,      -1,      -1,      -1, -1], # Sukeban Deka III (Japan).nes
"419461d0": [    2,      -1,       1, 0,      -1,      -1,      -1, -1], # Super Cars (USA).nes

# CNROM by      -1 has bus conflicts disabled, set to submapper 2 to enable bus conflicts
"ac8dcdea": [    3,       2,       1, 0,      -1,      -1,      -1, -1], # Cybernoid - The Fighting Machine (USA).nes

"2915faf0": [    3,       0,       0, 0,      -1,      -1,      -1, -1], # Incantation (Asia) (Ja) (Unl).nes
"8f154a0d": [    3,       0,       0, 0,      -1,      -1,      -1, -1], # Pu Ke Jing Ling (Asia) (Unl).nes
"b0c871c5": [    3,       0,       1, 0,      -1,      -1,      -1, -1], # Wei Lai Xiao Zi (Joy Van).nes
"b3be2f71": [    3,       0,       0, 0,      -1,      -1,      -1, -1], # Yanshan Chess (Unl).nes
"d04a40e6": [    3,       0,       0, 0,      -1,      -1,      -1, -1], # Bingo 75 (Asia) (Ja) (Unl).nes
"e41b440f": [    3,       0,       0, 0,      -1,      -1,      -1, -1], # Sidewinder (Joy Van).nes
"ebd0644d": [    3,       0,       1, 0,      -1,      -1,      -1, -1], # Master Chu & The Drunkard Hu (Joy Van).nes
"f283cf58": [    3,       0,       0, 0,      -1,      -1,      -1, -1], # Colorful Dragon (Sachen).nes
"2deb12b8": [    3,       0,       1, 0,      -1,      -1,      -1, -1], # Venice Beach Volleyball (Asia) (Ja) (Super Mega) (Unl).nes

"dbf90772": [    3,      -1,       0, 0,      -1,      -1,      -1, -1], # Alpha Mission (USA).nes
"d858033d": [    3,      -1,       0, 0,      -1,      -1,      -1, -1], # ASO - Armored Scrum Object (J).nes
"d858033d": [    3,      -1,       0, 0,      -1,      -1,      -1, -1], # ASO - Armored Scrum Object (J).nes
"637ba508": [    3,      -1,       1, 0,      -1,      -1,       1, -1], # Adan y Eva (Spain) (Gluk Video) (Unl).nes
"9bde3267": [    3,      -1,       1, 0,      -1,      -1,      -1, -1], # Adventures of Dino Riki (USA).nes
"d8eff0df": [    3,      -1,       1, 0,      -1,      -1,      -1, -1], # Gradius (Japan).nes
"1d41cc8c": [    3,      -1,       1, 0,      -1,      -1,      -1, -1], # Gyruss (USA).nes
"cf322bb3": [    3,      -1,       1, 0,      -1,      -1,      -1, -1], # John Elway's Quarterback (USA).nes
"b5d28ea2": [    3,      -1,       1, 0,      -1,      -1,      -1, -1], # Mystery Quest (USA).nes
"02cc3973": [    3,      -1,       1, 0,      -1,      -1,      -1, -1], # Ninja Kid (USA).nes
"bc065fc3": [    3,      -1,       1, 0,      -1,      -1,      -1, -1], # Pipe Dream (U) [b1].nes
"c9ee15a7": [    3,      -1,      -1, 0,      -1,      -1,      -1, -1], # Aladdin III (1995) (Unl) [hM03].nes
"8dedea07": [    3,      -1,       0, 0,      -1,      -1,      -1, -1], # Shui Guo Li (Ch) [a1].nes
"684afccd": [    3,      -1,       1, 0,      -1,      -1,      -1, -1], # Space Hunter (Japan).nes

"97b6cb19": [    4,      -1,      -1, 0,      -1,      -1,      -1, -1], # Aladdin (SuperGame) (Mapper 4) [!].nes
"d97c31b0": [  206,      -1,       1, 0,      -1,      -1,      -1, -1], # Lasalle Ishii no Child's Quest (Japan).nes
"404b2e8b": [    4,      -1,       2, 0,      -1,      -1,      -1, -1], # Rad Racer II (USA).nes
"15141401": [    4,      -1,       8, 0,      -1,      -1,      -1, -1], # Asmik-kun Land (Japan).nes
"4cccd878": [    4,      -1,       8, 0,      -1,      -1,      -1, -1], # Kyatto Ninden Teyandee (J) [b1].nes
"59280bec": [    4,      -1,       8, 0,      -1,      -1,      -1, -1], # Jackie Chan (Japan).nes
"7474ac92": [    4,      -1,       8, 0,      -1,      -1,      -1, -1], # Kabuki - Quantum Fighter (USA).nes
"f2594374": [    4,      -1,       8, 0,      -1,      -1,      -1, -1], # Matendouji (Japan).nes
"5337f73c": [    4,      -1,       8, 1,      -1,      -1,      -1, -1], # Niji no Silk Road (Japan).nes
"9eefb4b4": [    4,      -1,       8, 0,      -1,      -1,      -1, -1], # Pachi-Slot Adventure 2 - Sorotta-kun no Pachi-Slot Tanteidan (Japan).nes
"afe03802": [    4,      -1,       8, 0,      -1,      -1,      -1, -1], # Pachio-kun 3 (Japan) (Rev A).nes
"21a653c7": [    4,      -1,      -1, 0,      -1,      -1,      -1, -1], # Super Sky Kid (VS).nes
"db7f07be": [    4,      -1,      -1, 1,      -1,      -1,      -1, -1], # Toki (USA) (Beta).nes
"8F6CC85A": [    4,      -1,      -1, 1,      -1,      -1,      -1, -1], # (KT-220B) Totally Rad 4-in-1.nes [overdump]
"AB9DE91F": [    4,      -1,      -1, 1,      -1,      -1,      -1, -1], # (KT-220B) Totally Rad 4-in-1.nes, Commonly set to mapper 176

# MMC5
"671f23a8": [    5,      -1,      -1, 0,       0,        0,       1, -1], # Castlevania III - Dracula's Curse (E)
"cd4e7430": [    5,      -1,      -1, 0,       0,        0,      -1, -1], # Castlevania III - Dracula's Curse (KC)
"ed2465be": [    5,      -1,      -1, 0,       0,        0,      -1, -1], # Castlevania III - Dracula's Curse (U)
"0afb395e": [    5,      -1,      -1, 0,       0,        0,      -1, -1], # Gun Sight
"b0480ae9": [    5,      -1,      -1, 0,       0,        0,      -1, -1], # Laser Invasion
"b4735fac": [    5,      -1,      -1, 0,       0,        0,      -1, -1], # Metal Slader Glory
"c4fe19f7": [    5,      -1,      -1, 0,       0,        0,      -1, -1], # Metal Slader Glory (EN translation)
"bb7f829a": [    5,      -1,      -1, 0,       0,        0,      -1, -1], # Uchuu Keibitai SDF
"0ec6c023": [    5,      -1,      -1, 1,    0x70,        0,      -1, -1], # Gemfire
"9cbadc25": [    5,      -1,      -1, 1,    0x70,        0,      -1, -1], # Just Breed
"bc80fb52": [    5,      -1,      -1, 1,    0x70,        0,      -1, -1], # Royal Blood
"d532e98f": [    5,      -1,      -1, 1,    0x70,        0,      -1, -1], # Shin 4 Nin Uchi Mahjong - Yakuman Tengoku
"15fe6d0f": [    5,      -1,      -1, 1,    0x77,        0,      -1, -1], # Bandit Kings of Ancient China
"fe3488d1": [    5,      -1,      -1, 1,    0x77,        0,      -1, -1], # Daikoukai Jidai
"1ced086f": [    5,      -1,      -1, 1,    0x77,        0,      -1, -1], # Ishin no Arashi
"6396b988": [    5,      -1,      -1, 1,    0x77,        0,      -1, -1], # L'Empereur (J)
"9c18762b": [    5,      -1,      -1, 1,    0x77,        0,      -1, -1], # L'Empereur (U)
"eee9a682": [    5,      -1,      -1, 1,    0x77,        0,      -1, -1], # Nobunaga no Yabou - Sengoku Gunyuu Den (J) (PRG0)
"f9b4240f": [    5,      -1,      -1, 1,    0x77,        0,      -1, -1], # Nobunaga no Yabou - Sengoku Gunyuu Den (J) (PRG1)
"8ce478db": [    5,      -1,      -1, 1,    0x77,        0,      -1, -1], # Nobunaga's Ambition 2
"39f2ce4b": [    5,      -1,      -1, 1,    0x77,        0,      -1, -1], # Suikoden - Tenmei no Chikai
"aca15643": [    5,      -1,      -1, 1,    0x77,        0,      -1, -1], # Uncharted Waters
"6f4e4312": [    5,      -1,      -1, 1,    0x90,        0,      -1, -1], # Aoki Ookami to Shiroki Mejika - Genchou Hishi
"f540677b": [    5,      -1,      -1, 1,    0x90,        0,      -1, -1], # Nobunaga no Yabou - Bushou Fuuun Roku
"f011e490": [    5,      -1,      -1, 1,    0x90,        0,      -1, -1], # Romance of The Three Kingdoms II
"184c2124": [    5,      -1,      -1, 1,    0x90,        0,      -1, -1], # Sangokushi II (J) (PRG0)
"ee8e6553": [    5,      -1,      -1, 1,    0x90,        0,      -1, -1], # Sangokushi II (J) (PRG1)

"f518dd58": [    7,      -1,       8, 0,      -1,      -1,      -1, -1], # Captain Skyhawk (USA).nes
"6c4a9735": [    7,      -1,       8, 1,      -1,      -1,       1, -1], # WWF Wrestlemania (Europe).nes
"84382231": [    9,      -1,      -1, 0,      -1,      -1,      -1, -1], # Punch-Out!! (Japan) (Gold Edition).nes
"be939fce": [    9,      -1,      -1, 0,      -1,      -1,      -1, -1], # Punch-Out!! (U) [b1].nes
"7b837fde": [    9,      -1,      -1, 1,      -1,      -1,      -1, -1], # Mike Tyson's Punch-Out!! (PC10) [b1].nes
"345d3a1a": [   11,      -1,       1, 0,      -1,      -1,      -1, -1], # Castle of Deceit (USA) (Unl).nes
"b79f2651": [   11,      -1,       1, 0,      -1,      -1,      -1, -1], # Chiller (USA) (Unl).nes
"5e66eaea": [   13,      -1,       1, 0,      -1,      -1,      -1, -1], # Videomation (USA).nes
"cd373baa": [   14,      -1,      -1, 0,      -1,      -1,      -1, -1], # 武士魂 (8 characters).nes
"bfc7a2e9": [   16,      -1,       8, 0,      -1,      -1,      -1, -1], # Dragon Ball 3 - Gokuu Den (Japan) (Rev 1).nes
"6e68e31a": [   16,      -1,       8, 0,      -1,      -1,      -1, -1], # Dragon Ball 3 - Gokuu Den (Japan).nes
"33b899c9": [   16,      -1,      -1, 0,      -1,      -1,      -1, -1], # Dragon Ball - Daimaou Fukkatsu (Japan).nes
"a262a81f": [   16,      -1,      -1, 0,      -1,      -1,      -1, -1], # Rokudenashi Blues (Japan).nes
"286fcd20": [   21,      -1,      -1, 1,      -1,      -1,      -1, -1], # Ganbare Goemon Gaiden 2 - Tenka no Zaihou (Japan).nes
"335e6339": [   23,      -1,      -1, 0,      -1,      -1,      -1, -1], # kung fu legend (unl)[!].nes
"1a8d767b": [   23,      -1,      -1, 0,      -1,      -1,      -1, -1], # kung fu legend (unl)[!p].nes
"0e263d47": [   23,      -1,      -1, 0,      -1,      -1,      -1, -1], # World Hero (Unl) (TV System Select) [!].nes
"e4a291ce": [   23,      -1,      -1, 0,      -1,      -1,      -1, -1], # World Hero (Unl) [!].nes
"51e9cd33": [   23,      -1,      -1, 0,      -1,      -1,      -1, -1], # World Hero (Unl) [b1].nes
"105dd586": [   27,      -1,      -1, 0,      -1,      -1,      -1, -1], # Mi Hun Che (Ch)(full copyrights)[!].nes
"bc9bb6c1": [   27,      -1,      -1, 0,      -1,      -1,      -1, -1], # Super Car (Ch)(full copyrights)[!].nes
"43753886": [   27,      -1,      -1, 0,      -1,      -1,      -1, -1], # Mi Hun Che (Ch)(replaced copyrights)[p1][!].nes
"5b3de3d1": [   27,      -1,      -1, 0,      -1,      -1,      -1, -1], # --
"511e73f8": [   27,      -1,      -1, 0,      -1,      -1,      -1, -1], # Mi Hun Che (Ch) [p1][b2].nes
"5555fca3": [   32,      -1,       8, 0,      -1,      -1,      -1, -1], # Ai Sensei no Oshiete - Watashi no Hoshi (J) [b1].nes
"283ad224": [   32,      -1,       8, 0,      -1,      -1,      -1, -1], # Ai Sensei no Oshiete - Watashi no Hoshi (Japan).nes
"243a8735": [   32,      -1,  0x10|4, 0,      -1,      -1,      -1, -1], # Major League (Japan).nes
"8a7d0abe": [   33,      -1,       8, 0,      -1,      -1,      -1, -1], # Akira (Japan).nes
"376138d8": [   33,      -1,       8, 0,      -1,      -1,      -1, -1], # Akira (J) [a1].nes
"adf606f6": [   33,      -1,       8, 0,      -1,      -1,      -1, -1], # Bakushou!! Jinsei Gekijou (Japan).nes
"bc7b1d0f": [   33,      -1,       8, 0,      -1,      -1,      -1, -1], # Bakushou!! Jinsei Gekijou 2 (Japan).nes
"7a497ae3": [   33,      -1,       8, 0,      -1,      -1,      -1, -1], # Don Doko Don (Japan).nes
"baca10a9": [   33,      -1,       8, 0,      -1,      -1,      -1, -1], # Golfkko Open (Japan).nes
"f80bdc50": [   33,      -1,       8, 0,      -1,      -1,      -1, -1], # Insector X (Japan).nes
"2a6559a1": [   33,      -1,       8, 0,      -1,      -1,      -1, -1], # Operation Wolf (Japan).nes
"aeb7fce9": [   33,      -1,       8, 0,      -1,      -1,      -1, -1], # Power Blazer (Japan).nes
"d920f9df": [   33,      -1,       8, 0,      -1,      -1,      -1, -1], # Takeshi no Sengoku Fuuunji (Japan).nes
"3cd4b420": [   33,      -1,       8, 0,      -1,      -1,      -1, -1], # Takeshi no Sengoku Fuuunji (Japan) (Beta).nes
"4c7c1af3": [   34,      -1,       0, 0,      -1,      -1,      -1, -1], # Caesars Palace (U) [b1].nes
"932ff06e": [   34,      -1,       0, 0,      -1,      -1,      -1, -1], # Classic Concentration (U) [b1].nes
"f46ef39a": [   37,      -1,      -1, 0,      -1,      -1,       1, -1], # Super Mario Bros. + Tetris + Nintendo World Cup (Europe) (Rev 1).nes
"4686c5dd": [   41,      -1,      -1, 0,      -1,      -1,      -1, -1], # Caltron - 6 in 1 (USA) (Unl).nes
"090c0c17": [   42,      -1,       1, 0,      -1,      -1,      -1, -1], # Ai Senshi Nicol (FDS Conversion) [p1][!].nes
"4df84825": [   42,      -1,       1, 0,      -1,      -1,      -1, -1], # Ai Senshi Nicol (FDS Conversion) [p2][!].nes
"579e5bc5": [   42,      -1,       1, 0,      -1,      -1,      -1, -1], # Ai Senshi Nicol (FDS Conversion) [p3].nes
"c744f205": [   42,      -1,       1, 0,      -1,      -1,      -1, -1], # Ai Senshi Nicol (FDS Conversion) [p3][t1].nes
"71699765": [   42,      -1,      -1, 0,      -1,      -1,      -1, -1], # Love Warrior Nicol.nes
"6bf3f6a3": [   42,      -1,      -1, 0,      -1,      -1,      -1, -1], # Bio Miracle Bokutte Upa (J) (Mario Baby - FDS Conversion).nes
"5ba1c5cf": [   42,      -1,      -1, 0,      -1,      -1,      -1, -1], # Green Beret (FDS Conversion) (Unl).nes
"50ab1ab2": [   42,      -1,      -1, 0,      -1,      -1,      -1, -1], # ?? Green Beret (FDS Conversion, LH09) (Unl) [U][!][t1] (160K PRG
"7ccb12a3": [   43,      -1,      -1, 0,      -1,      -1,      -1, -1], # ?? SMB2j
"6c71feae": [   45,      -1,      -1, 0,      -1,      -1,      -1, -1], # Kunio 8-in-1 [p1].nes
"40c0ad47": [   48,      -1,       8, 0,      -1,      -1,      -1, -1], # Flintstones, The - The Rescue of Dino & Hoppy (Japan).nes
"aebd6549": [   48,      -1,       8, 0,      -1,      -1,      -1, -1], # Bakushou!! Jinsei Gekijou 3 (Japan).nes
"6cdc0cd9": [   48,      -1,       8, 0,      -1,      -1,      -1, -1], # Bubble Bobble 2 (Japan).nes
"99c395f9": [   48,      -1,       8, 0,      -1,      -1,      -1, -1], # Captain Saver (Japan).nes
"a7b0536c": [   48,      -1,       8, 0,      -1,      -1,      -1, -1], # Don Doko Don 2 (Japan).nes
"b17c828a": [   48,      -1,       8, 0,      -1,      -1,      -1, -1], # Don Doko Don 2 (J) [a1].nes
"40c0ad47": [   48,      -1,       8, 0,      -1,      -1,      -1, -1], # Flintstones, The - The Rescue of Dino & Hoppy (J).nes
"1500e835": [   48,      -1,       8, 0,      -1,      -1,      -1, -1], # Jetsons, The - Cogswell's Caper (Japan).nes
"e2c94bc2": [   48,      -1,       8, 0,      -1,      -1,      -1, -1], # Super Bros 8 (Unl) [!].nes
"a912b064": [   51,      -1,       8, 0,      -1,      -1,      -1,   0x800], # 11-in-1 Ball Games [p1][o1].nes (has CHR ROM when it shouldn't)
"2e72a5d9": [   59,      -1,       8, 0,      -1,      -1,      -1, -1], # 11-in-1 (66-in-1, 86-in-1, 63-in-1).nes
"39f514fd": [   59,      -1,       8, 0,      -1,      -1,      -1, -1], # 18 in 1 (118-in-1, 138-in-1, 198-in-1)VTxxxx.nes
"d8b1f465": [   59,      -1,       8, 0,      -1,      -1,      -1, -1], # 26-in-1 (36-in-1, 46-in-1,56-in-1) VT 335.nes
"cf82fae9": [   59,      -1,       8, 0,      -1,      -1,      -1, -1], # 28 in 1 (38-in-1, 48-in-1, 58-in1)VTxxxx.nes
"a7a98698": [   59,      -1,       8, 0,      -1,      -1,      -1, -1], # 28-in-1 (46-in-1, 63-in-1, 118-in-1)VT-5116.nes
"21fd7143": [   59,      -1,       8, 0,      -1,      -1,      -1, -1], # 41-in-1 (5-in-1,71-in-1)  VT345.nes
"49ec88d6": [   59,      -1,       8, 0,      -1,      -1,      -1, -1], # 42-in-1 NT-234 Bad Game Road Fighter.nes
"60306f19": [   59,      -1,       8, 0,      -1,      -1,      -1, -1], # 42-in-1 PCB 3840.nes
"450cd86e": [   59,      -1,       8, 0,      -1,      -1,      -1, -1], # 48-in-1 (62-in-1,73-in-1,88-in-1) VTxxx.nes
"d774e041": [   59,      -1,       8, 0,      -1,      -1,      -1, -1], # 50-in-1 (60-in-1,70-in-1,80-in-1) NT-113.nes
"3c4e94f6": [   59,      -1,       8, 0,      -1,      -1,      -1, -1], # 51-in-1 (61-in-1, 71-in-1, 81-in-1) VT5310.nes
"0422ed44": [   59,      -1,       8, 0,      -1,      -1,      -1, -1], # 7-in-1 (32-in-1,66-in-1,119-in-1) VT15004.nes
"7efc0d2c": [   59,      -1,       8, 0,      -1,      -1,      -1, -1], # 7-in-1 (5000-in-1, 999999999-in-1, 10000000-in-1)NC-07N.nes
"6d92dff1": [   59,      -1,       8, 0,      -1,      -1,      -1, -1], # TN 95-in-1 (6-in-1) [p1].nes
"39ab0fc7": [   64,      -1,       8, 0,      -1,      -1,      -1, -1], # Hard Drivin' (USA) (Proto) (Unl).nes
"b19a55dd": [   64,      -1,       8, 0,      -1,      -1,      -1, -1], # Road Runner (USA) (Unl).nes
"f92be3ec": [   64,      -1,      -1, 0,      -1,      -1,      -1, -1], # Rolling Thunder (USA) (Unl).nes
"d114f544": [   66,      -1,       0, 1,      -1,      -1,      -1, -1], # AV Super Real Pachinko (Japan) (Unl).nes
"e84274c5": [   66,      -1,       1, 0,      -1,      -1,      -1, -1], # Mississippi Satsujin Jiken (J) [h2].nes
"bde3ae9b": [   66,      -1,       1, 0,      -1,      -1,      -1, -1], # Doraemon (Japan).nes
"9552e8df": [   66,      -1,       1, 0,      -1,      -1,      -1, -1], # Dragon Ball - Shen Long no Nazo (Japan).nes
"811f06d9": [   66,      -1,       1, 0,      -1,      -1,      -1, -1], # Dragon Power (USA).nes
"d26efd78": [   66,      -1,       1, 0,      -1,      -1,      -1, -1], # Super Mario Bros. + Duck Hunt (USA).nes
"dd8ed0f7": [   70,      -1,       1, 0,      -1,      -1,      -1, -1], # Kamen Rider Club (Japan).nes
"bba58be5": [   70,      -1,      -1, 0,      -1,      -1,      -1, -1], # Family Trainer 6 - Manhattan Police (Japan).nes
"370ceb65": [   70,      -1,      -1, 0,      -1,      -1,      -1, -1], # Family Trainer 5 - Meiro Daisakusen (Japan).nes
"86be4746": [    2,      -1,      -1, 0,      -1,      -1,      -1, -1], # Dooly Bravo Land (Korea) (Unl).nes
"e62e3382": [   71,      -1,      -1, 0,      -1,      -1,      -1, -1], # MiG 29 - Soviet Fighter (USA) (Unl).nes
"ac7b0742": [   71,      -1,      -1, 0,      -1,      -1,      -1, -1], # Golden KTV (Ch) [!].nes
"054bd3e9": [   74,      -1,      -1, 1,      -1,      -1,      -1, -1], # Di 4 Ci - Ji Qi Ren Dai Zhan (Ch).nes
"496ac8f7": [   74,      -1,      -1, 1,      -1,      -1,      -1, -1], # Ji Jia Zhan Shi (Ch) [b3].nes
"ae854cef": [   74,      -1,      -1, 1,      -1,      -1,      -1, -1], # Jia A Fung Yun (Ch).nes
"ba51ac6f": [   78,      -1,       2, 0,      -1,      -1,      -1, -1], # Holy Diver (Japan).nes
"3d1c3137": [   78,      -1,       8, 0,      -1,      -1,      -1, -1], # Uchuusen Cosmo Carrier (Japan).nes
"a4fbb438": [   79,      -1,       0, 0,      -1,      -1,      -1, -1], # F-15 City War (AVE) (REV1.x) [b1].nes
"d4a76b07": [   79,      -1,       0, 0,      -1,      -1,      -1, -1], # F-15 City War (AVE) (REV1.x) [b2].nes
"8eab381c": [   79,      -1,       1, 0,      -1,      -1,      -1, -1], # Deathbots (USA) (Rev 1) (Unl).nes
"1eb4a920": [   79,      -1,       1, 0,      -1,      -1,      -1, -1], # Double Strike - Aerial Attack Force (USA) (v1.1) (Unl).nes
"3e1271d5": [   79,      -1,       1, 0,      -1,      -1,      -1, -1], # Tiles of Fate (USA) (Unl).nes
"d2699893": [   88,      -1,       0, 0,      -1,      -1,      -1, -1], # Dragon Spirit - Aratanaru Densetsu (Japan).nes
"bb7c5f7a": [   89,      -1,       8, 0,      -1,      -1,      -1, -1], # Tenka no Goikenban - Mito Koumon (J) [f1].nes
"082778e6": [   91,       1,       1, 0,      -1,      -1,      -1, -1], # Super Fighter III.nes
"10119e6b": [   93,      -1,       8, 0,      -1,      -1,      -1, -1], # Fantasy Zone (Japan) (Sunsoft).nes
"2b750bf9": [  101,      -1,       1, 0,      -1,      -1,      -1, -1], # Urusei Yatsura - Lum no Wedding Bell (Japan) (Beta).nes
"0da5e32e": [  101,      -1,       1, 0,      -1,      -1,      -1, -1], # Urusei Yatsura - Lum no Wedding Bell (Japan).nes
"6096f84e": [  104,      -1,       1, 0,      -1,      -1,      -1, -1], # Pegasus 5-in-1 (Golden Five) (Unl).nes
"3d3ff543": [  113,      -1,       0, 0,      -1,      -1,      -1, -1], # Kazama Jun to Asama Yuuko no AV Dragon Mahjong (Japan) (Unl).nes
"68379fdb": [  113,      -1,       1, 1,      -1,      -1,       1, -1], # Pipemania (Australia) (HES) (Unl).nes
"6a03d3f3": [  114,      -1,      -1, 0,      -1,      -1,      -1, -1], # Lion King, The (Unl) (Mapper 114).nes
"0d98db53": [  114,      -1,      -1, 0,      -1,      -1,      -1, -1], # Pocahontas (Unl).nes
"f5676f0b": [  114,      -1,      -1, 0,      -1,      -1,      -1, -1], # Super Donkey Kong (Unl) [b1].nes
"c5e5c5b2": [  115,      -1,      -1, 0,      -1,      -1,      -1, -1], # Bao Qing Tian (Ch).nes
"e40dfb7e": [  116,      -1,      -1, 0,      -1,      -1,      -1, -1], # Somari (SOMARI-P) (NT-616) (Unl) [!].nes
"c9371ebb": [  116,      -1,      -1, 0,      -1,      -1,      -1, -1], # Somari (SOMARI-W) (Unl) [!].nes
"78b657ac": [  118,      -1,      -1, 0,      -1,      -1,      -1, -1], # Armadillo (Japan).nes
"90c773c1": [  118,      -1,      -1, 0,      -1,      -1,      -1, -1], # Goal! Two (USA).nes
"b9b4d9e0": [  118,      -1,      -1, 0,      -1,      -1,      -1, -1], # NES Play Action Football (USA).nes
"07d92c31": [  118,      -1,      -1, 1,      -1,      -1,      -1, -1], # RPG Jinsei Game (Japan).nes
"37b62d04": [  118,      -1,      -1, 1,      -1,      -1,      -1, -1], # Ys III - Wanderers from Ys (Japan).nes
"318e5502": [  121,      -1,      -1, 0,      -1,      -1,      -1, -1], # Sonic 3D Blast 6 (Unl).nes
"ddcfb058": [  121,      -1,      -1, 0,      -1,      -1,      -1, -1], # Street Fighter Zero 2 '97 (Unl) [!].nes
"d2674b0a": [  132,      -1,       0, 0,      -1,      -1,      -1, -1], # Qi Wang - Chinese Chess (Asia) (Ja) (Unl).nes
"5aefbc94": [  133,      -1,      -1, 0,      -1,      -1,       1, -1], # Jovial Race (Asia) (Ja) (PAL) (Unl).nes
"B550B627": [  136,      -1,      -1, 0,      -1,      -1,      -1, -1], # Incantation (Dip Bin) (Joy Van).nes
"c2df0a00": [  140,      -1,       1, 0,      -1,      -1,      -1, -1], # Bio Senshi Dan - Increaser Tono Tatakai (J) [hM66][b3].nes
"e46b1c5d": [  140,      -1,       1, 0,      -1,      -1,      -1, -1], # Mississippi Satsujin Jiken (Japan).nes
"3293afea": [  140,      -1,       1, 0,      -1,      -1,      -1, -1], # Mississippi Satsujin Jiken (Japan) (Rev A).nes
"6bc65d7e": [  140,      -1,       1, 0,      -1,      -1,      -1, -1], # Youkai Club (Japan).nes
"5caa3e61": [  144,      -1,       1, 0,      -1,      -1,      -1, -1], # Death Race (USA) (Unl).nes
"48239b42": [  146,      -1,      -1, 0,      -1,      -1,      -1, -1], # Mahjang Companion (Asia) (Ja) (Hacker) (Unl).nes
"b6a727fa": [  146,      -1,      -1, 0,      -1,      -1,      -1, -1], # Papillon Gals (Japan) (Unl).nes
"a62b79e1": [  146,      -1,      -1, 0,      -1,      -1,      -1, -1], # Sidewinder (HES) [o1].nes
"cc868d4e": [  149,      -1,      -1, 0,      -1,      -1,      -1, -1], # Taiwan Mahjong 16 (Sachen) [a1][!].nes
"29582ca1": [  150,      -1,      -1, 0,      -1,      -1,      -1, -1], # Mei Nu Quan (Honey Peach) (Sachen) [!].nes
"40dbf7a2": [  150,      -1,      -1, 0,      -1,      -1,       1, -1], # Olympic IQ (Asia) (Ja) (PAL) (Unl).nes
"73fb55ac": [  150,      -1,      -1, 0,      -1,      -1,      -1, -1], # Lightgun Game 2 in 1 - Cosmocop + Cyber Monster (Asia) (Ja) (Unl).nes
"ddcbda16": [  150,      -1,      -1, 0,      -1,      -1,      -1, -1], # Lightgun Game 2 in 1 - Tough Cop + Super Tough Cop (Asia) (Ja) (Unl).nes
"47918d84": [  150,      -1,      -1, 0,      -1,      -1,       1, -1], # Auto-Upturn (Asia) (Ja) (PAL) (Unl).nes
"471173e7": [  150,      -1,      -1, 0,      -1,      -1,       1, -1], # Chinese Checkers (Asia) (Ja) (PAL) (Unl).nes
"2394ae1c": [  150,      -1,      -1, 0,      -1,      -1,       1, -1], # Happy Pairs (Asia) (Ja) (PAL) (Unl).nes
"cab40a6c": [  150,      -1,      -1, 0,      -1,      -1,       1, -1], # Magic Cube (Asia) (Ja) (PAL) (Unl).nes
"be17e27b": [  150,      -1,      -1, 0,      -1,      -1,      -1, -1], # Poker III (Asia) (Ja) (Alt 2) (Unl).nes
"34ddf806": [  150,      -1,      -1, 0,      -1,      -1,      -1, -1], # Strategist (Asia) (Ja) (NTSC) (Unl).nes
"c06facfc": [  150,      -1,      -1, 0,      -1,      -1,       1, -1], # Strategist (Asia) (Ja) (PAL) (Unl).nes
"a95a915a": [  150,      -1,      -1, 0,      -1,      -1,      -1, -1], # Tasac (Asia) (Ja) (Unl).nes
"0f141525": [  152,      -1,       8, 0,      -1,      -1,      -1, -1], # Arkanoid II (Japan).nes
"bda8f8e4": [  152,      -1,       8, 0,      -1,      -1,      -1, -1], # Gegege no Kitarou 2 - Youkai Gundan no Chousen (Japan).nes
"b1a94b82": [  152,      -1,       8, 0,      -1,      -1,      -1, -1], # Pocket Zaurus - Juu Ouken no Nazo (Japan).nes
"026c5fca": [  152,      -1,       8, 0,      -1,      -1,      -1, -1], # Saint Seiya - Ougon Densetsu (Japan).nes
"3f15d20d": [  153,      -1,       8, 1,      -1,      -1,      -1, -1], # Famicom Jump II - Saikyou no 7 Nin (Japan).nes
"d1691028": [  154,      -1,       8, 0,      -1,      -1,      -1, -1], # Devil Man (Japan).nes
"cfd4a281": [  155,      -1,       8, 1,      -1,      -1,      -1, -1], # Money Game, The (Japan).nes
"2f27cdef": [  155,      -1,       8, 0,      -1,      -1,      -1, -1], # Tatakae!! Rahmen Man - Sakuretsu Choujin 102 Gei (J) [b1].nes
"c1719664": [  155,      -1,       8, 0,      -1,      -1,      -1, -1], # Tatakae!! Rahmen Man - Sakuretsu Choujin 102 Gei (Japan).nes
"ccc03440": [  156,      -1,      -1, 0,      -1,      -1,      -1, -1], # Buzz & Waldog (USA) (Proto) (Unl).nes
"983d8175": [  157,      -1,       8, 0,      -1,      -1,      -1, -1], # Datach - Battle Rush - Build Up Robot Tournament (Japan).nes
"894efdbc": [  157,      -1,       8, 0,      -1,      -1,      -1, -1], # Datach - Crayon Shin-chan - Ora to Poi Poi (Japan).nes
"19e81461": [  157,      -1,       8, 0,      -1,      -1,      -1, -1], # Datach - Dragon Ball Z - Gekitou Tenkaichi Budoukai (Japan).nes
"be06853f": [  157,      -1,       8, 0,      -1,      -1,      -1, -1], # Datach - J.League Super Top Players (Japan).nes
"0be0a328": [  157,      -1,       8, 0,      -1,      -1,      -1, -1], # Datach - SD Gundam - Gundam Wars (Japan).nes
"5b457641": [  157,      -1,       8, 0,      -1,      -1,      -1, -1], # Datach - Ultraman Club - Supokon Fight! (Japan).nes
"f51a7f46": [  157,      -1,       8, 0,      -1,      -1,      -1, -1], # Datach - Yu Yu Hakusho - Bakutou Ankoku Bujutsukai (Japan).nes
"cbf4366f": [  158,      -1,       8, 0,      -1,      -1,      -1, -1], # Alien Syndrome (USA) (Unl).nes
"e170404c": [  159,      -1,      -1, 1,    0x10,      -1,      -1, -1], # SD Gundam Gaiden - Knight Gundam Monogatari (Japan).nes
"276ac722": [  159,      -1,      -1, 1,    0x10,      -1,      -1, -1], # SD Gundam Gaiden - Knight Gundam Monogatari (Japan) (Rev 1).nes
"0cf42e69": [  159,      -1,      -1, 1,    0x10,      -1,      -1, -1], # Magical Taruruuto-kun - Fantastic World!! (Japan).nes
"dcb972ce": [  159,      -1,      -1, 1,    0x10,      -1,      -1, -1], # Magical Taruruuto-kun - Fantastic World!! (Japan) (Rev 1).nes
"b7f28915": [  159,      -1,      -1, 1,    0x10,      -1,      -1, -1], # Magical Taruruuto-kun 2 - Mahou Daibouken (Japan).nes
"183859d2": [  159,      -1,      -1, 1,    0x10,      -1,      -1, -1], # Dragon Ball Z - Kyoushuu! Saiya Jin (Japan).nes
"58152b42": [  160,      -1,       1, 0,      -1,      -1,      -1, -1], # Pipe V (Asia) (Ja) (Unl).nes

# Mapper 162: Waixing FS304
"08FBF3F0": [  162,       0,       0, 1,    0x70,    0x07,       3, -1], # Mummy - 神鬼传奇
"F5D34C8E": [  162,       0,       0, 1,    0x70,    0x07,       3, -1], # Zelda 传说: 三神之力
"EE3A1CA8": [  162,       0,       0, 1,    0x70,    0x07,       3, -1], # 法老王 - Pharaoh
"CF4ADAAD": [  162,       0,       0, 1,    0x70,    0x07,       3, -1], # 火焰纹章 - 圣战的系谱
"B2045E9C": [  162,       0,       0, 1,    0x70,    0x07,       3, -1], # 聖火徽章 III
"1C098942": [  162,       0,       0, 1,    0x70,    0x07,       3, -1], # 西游记后传
# Mapper 162: Nanjing games that use an FS304 feature as a protection check
"8589652D": [  162,       0,       1, 1,    0x70,    0x07,       3, -1], # 农场小精灵
"99FE9AB5": [  162,       0,       1, 1,    0x70,    0x07,       3, -1], # 时空斗士 - Pegasus Senya
"82F204AE": [  162,       0,       1, 1,    0x70,    0x07,       3, -1], # 梁山英雄
	
# Mapper 163: Nanjing, running with $5300=$04
"C9ABA7F0": [  163,       0,       1, 1,    0x70,    0x07,       3, -1], # Chrono Trigger - 时空之轮
"143B4D30": [  163,       0,       1, 1,    0x70,    0x07,       3, -1], # Final Fantasy VII - 最终幻想 7
"609458B6": [  163,       0,       1, 1,    0x70,    0x07,       3, -1], # Final Fantasy VII - 最终幻想 7, English translation
"3CC55A44": [  163,       0,       1, 1,    0x70,    0x07,       3, -1], # The Legend of Zelda - 塞尔达传说: 神奇的帽子
"4752BD5E": [  163,       0,       1, 1,    0x70,    0x07,       3, -1], # 七國大戰
"1121C0D1": [  163,       0,       1, 1,    0x70,    0x07,       3, -1], # 三国志 之 傲视天地
"8E4294A9": [  163,       0,       1, 1,    0x70,    0x07,       3, -1], # 三国志: 曹操传
"F52468E7": [  163,       0,       1, 1,    0x70,    0x07,       3, -1], # 三国无双: 猛将传
"EF7BA485": [  163,       0,       1, 1,    0x70,    0x07,       3, -1], # 仙剑奇侠
"20E1CF44": [  163,       0,       1, 1,    0x70,    0x07,       3, -1], # 仙界精灵
"9688AEEA": [  163,       0,       1, 1,    0x70,    0x07,       3, -1], # 倚天传说
"04DFE0D4": [  163,       0,       1, 1,    0x70,    0x07,       3, -1], # 倚天剑传奇
"6CB6D619": [  163,       0,       1, 1,    0x70,    0x07,       3, -1], # 口袋宝石 - 银
"8C73A47B": [  163,       0,       1, 1,    0x70,    0x07,       3, -1], # 哪吒传奇
"E40DA18F": [  163,       0,       1, 1,    0x70,    0x07,       3, -1], # 圣剑传说
"C07CD2CE": [  163,       0,       1, 1,    0x70,    0x07,       3, -1], # 圣斗士星矢: 天马之幻想
"9237C200": [  163,       0,       1, 1,    0x70,    0x07,       3, -1], # 大话西游
"4FB02A43": [  163,       0,       1, 1,    0x70,    0x07,       3, -1], # 天龙八部
"52D7FE18": [  163,       0,       1, 1,    0x70,    0x07,       3, -1], # 太空幻想
"A01CA587": [  163,       0,       1, 1,    0x70,    0x07,       3, -1], # 幻世录
"89F4ACD1": [  163,       0,       1, 1,    0x70,    0x07,       3, -1], # 幻想传说
"65C63CC3": [  163,       0,       1, 1,    0x70,    0x07,       3, -1], # 数码战队 2
"2C01DE06": [  163,       0,       1, 1,    0x70,    0x07,       3, -1], # 数码暴龙
"B614AAA2": [  163,       0,       1, 1,    0x70,    0x07,       3, -1], # 暗黑破坏神 - Diablo
"80A18CDC": [  163,       0,       1, 1,    0x70,    0x07,       3, -1], # 机器人大战
"33DB45BA": [  163,       0,       1, 1,    0x70,    0x07,       3, -1], # 核心危机
"1B74A022": [  163,       0,       1, 1,    0x70,    0x07,       3, -1], # 梦幻沙漏
"695A7A70": [  163,       0,       1, 1,    0x70,    0x07,       3, -1], # 楚汉风云
"D0807FD2": [  163,       0,       1, 1,    0x70,    0x07,       3, -1], # 水浒 III
"524AF6E8": [  163,       0,       1, 1,    0x70,    0x07,       3, -1], # 水浒神兽 (南晶)
"2802E40F": [  163,       0,       1, 1,    0x70,    0x07,       3, -1], # 汉刘邦
"AC491507": [  163,       0,       1, 1,    0x70,    0x07,       3, -1], # 游戏王
"31C1BF98": [  163,       0,       1, 1,    0x70,    0x07,       3, -1], # 牧场物语
"191F7D5E": [  163,       0,       1, 1,    0x70,    0x07,       3, -1], # 真 Samurai Spirits - 武士道列传: 侍魂
"1FE67BB3": [  163,       0,       1, 1,    0x70,    0x07,       3, -1], # 真 Samurai Spirits - 魂之利刃
"4973B16B": [  163,       0,       1, 1,    0x70,    0x07,       3, -1], # 石器时代
"09C7AED3": [  163,       0,       1, 1,    0x70,    0x07,       3, -1], # 葫芦金刚
"4E3EDF88": [  163,       0,       1, 1,    0x70,    0x07,       3, -1], # 超级机器人大战 A
"74C1EDC7": [  163,       0,       1, 1,    0x70,    0x07,       3, -1], # 轩辕剑外传 之 天之痕
"723C6345": [  163,       0,       1, 1,    0x70,    0x07,       3, -1], # 金庸群侠传: 书剑江山
"DA47B05A": [  163,       0,       1, 1,    0x70,    0x07,       3, -1], # 隋唐英雄
"056F2B8E": [  163,       0,       1, 1,    0x70,    0x07,       3, -1], # 雷电皇: Pikachu传说
"04166E96": [  163,       0,       1, 1,    0x70,    0x07,       3, -1], # 魔幻世界
"85FA53E1": [  163,       0,       1, 1,    0x70,    0x07,       3, -1], # 魔界塔士
"2121DAB2": [  163,       0,       1, 1,    0x70,    0x07,       3, -1], # 魔界霸主
"E3EF9739": [  163,       0,       1, 1,    0x70,    0x07,       3, -1], # 黄金太阳	
# Mapper 163: Nanjing, running with $5300=$07
"6F94C5E5": [  163,       0,       1, 1,    0x70,    0x07,       3, -1], # Final Fantasy IV - 最终幻想 4: 光与暗 水晶纷争
"D6CBB05D": [  163,       0,       1, 1,    0x70,    0x07,       3, -1], # Naruto RPG - 火影忍者
"D7A4CBA5": [  163,       0,       1, 1,    0x70,    0x07,       3, -1], # 七龙珠大冒险 - Dragon Ball
"DF27B96C": [  163,       0,       1, 1,    0x70,    0x07,       3, -1], # 三国大乱斗: 战国 - Orochi
"8C60CECF": [  163,       0,       1, 1,    0x70,    0x07,       3, -1], # 三国志: 吕布传
"44AC9C8E": [  163,       0,       1, 1,    0x70,    0x07,       3, -1], # 三国群侠传
"A3193C51": [  163,       0,       1, 1,    0x70,    0x07,       3, -1], # 倚天屠龙记
"98266D3A": [  163,       0,       1, 1,    0x70,    0x07,       3, -1], # 傲视三国志
"A026AE52": [  163,       0,       1, 1,    0x70,    0x07,       3, -1], # 口袋宝石 - 红
"B6F72A18": [  163,       0,       1, 1,    0x70,    0x07,       3, -1], # 口袋宝石 - 蓝
"5108AB7F": [  163,       0,       1, 1,    0x70,    0x07,       3, -1], # 口袋宝石 - 金
"8EB1B4CF": [  163,       0,       1, 1,    0x70,    0x07,       3, -1], # 口袋水晶
"A66756AD": [  163,       0,       1, 1,    0x70,    0x07,       3, -1], # 口袋钻石
"3A613060": [  163,       0,       1, 1,    0x70,    0x07,       3, -1], # 吞食天地 VI: 刘备传
"B35BE92F": [  163,       0,       1, 1,    0x70,    0x07,       3, -1], # 圣斗士星矢: 北欧突击篇
"15E50ECD": [  163,       0,       1, 1,    0x70,    0x07,       3, -1], # 宠物高达战记
"BBAB3A61": [  163,       0,       1, 1,    0x70,    0x07,       3, -1], # 拳皇R-1: 最强格斗王
"2DA3A49C": [  163,       0,       1, 1,    0x70,    0x07,       3, -1], # 时空斗士 - Pegasus Senya, turns out to be wrong after all, and the mapper 162 one is the correct one
"222A136A": [  163,       0,       1, 1,    0x70,    0x07,       3, -1], # 武侠天地
"EBC6E2E2": [  163,       0,       1, 1,    0x70,    0x07,       3, -1], # 毁灭之神
"E08AB52E": [  163,       0,       1, 1,    0x70,    0x07,       3, -1], # 魔兽世界: 恶魔猎人
# Mapper 162: Nanjing, running with $5300=$07, but dumped without taking the D0/D1 bit swap into account.
#    Such dumps need not be invalidated however, as they can run as mapper 162 when properly emulated.
"9D8AA034": [  162,       0,       1, 1,    0x70,    0x07,       3, -1], # Final Fantasy IV - 最终幻想4: 光与暗 水晶纷争
"5E66E6C4": [  162,       0,       1, 1,    0x70,    0x07,       3, -1], # Naruto RPG - 火影忍者
"9DE10A91": [  162,       0,       1, 1,    0x70,    0x07,       3, -1], # 七龙珠大冒险 - Dragon Ball
"FC209609": [  162,       0,       1, 1,    0x70,    0x07,       3, -1], # 三国大乱斗: 战国 - Orochi
"696D98E3": [  162,       0,       1, 1,    0x70,    0x07,       3, -1], # 三国志: 吕布传
"9F197F2B": [  162,       0,       1, 1,    0x70,    0x07,       3, -1], # 三国群侠传
"975F64E2": [  162,       0,       1, 1,    0x70,    0x07,       3, -1], # 倚天屠龙记
"915C5179": [  162,       0,       1, 1,    0x70,    0x07,       3, -1], # 傲视三国志
"852BDB36": [  162,       0,       1, 1,    0x70,    0x07,       3, -1], # 口袋宝石 - 红
"B41CF445": [  162,       0,       1, 1,    0x70,    0x07,       3, -1], # 口袋宝石 - 蓝
"7829C3A9": [  162,       0,       1, 1,    0x70,    0x07,       3, -1], # 口袋宝石 - 金
"BC383C09": [  162,       0,       1, 1,    0x70,    0x07,       3, -1], # 口袋水晶
"A9C4712A": [  162,       0,       1, 1,    0x70,    0x07,       3, -1], # 口袋钻石
"C2B02B71": [  162,       0,       1, 1,    0x70,    0x07,       3, -1], # 吞食天地 VI: 刘备传
"054444A0": [  162,       0,       1, 1,    0x70,    0x07,       3, -1], # 圣斗士星矢: 北欧突击篇
"9BA518BA": [  162,       0,       1, 1,    0x70,    0x07,       3, -1], # 宠物高达战记
"4CE082F8": [  162,       0,       1, 1,    0x70,    0x07,       3, -1], # 拳皇R-1: 最强格斗王
"57414FB6": [  162,       0,       1, 1,    0x70,    0x07,       3, -1], # 武侠天地
"979239DE": [  162,       0,       1, 1,    0x70,    0x07,       3, -1], # 毁灭之神
"EFF96E8A": [  162,       0,       1, 1,    0x70,    0x07,       3, -1], # 魔兽世界: 恶魔猎人
	
# Mapper 164: Dongda PEC-9588 and Yancheng cy2000-3 PCB
"FE31765B": [  164,       0,       1, 0,    0x07,    0x07,       3, -1], # Pyramid 金字塔 PEC-9588 家庭电脑
"0878A7EE": [  164,       0,       0, 1,    0x35,    0x07,       3, -1], # Dark Seed - 黑暗之蛊
"56A0D271": [  164,       0,       0, 1,    0x35,    0x07,       3, -1], # Final Fantasy 太空戰士 V (rev0)
"CB1EF911": [  164,       0,       0, 1,    0x35,    0x07,       3, -1], # Final Fantasy 太空戰士 V (rev1)
"8209BA79": [  164,       0,       0, 1,    0x35,    0x07,       3, -1], # 櫻桃小丸子
"BC7562A6": [  164,       0,       0, 1,    0x35,    0x07,       3, -1], # 口袋精靈: 金
"65F1DB91": [  164,       0,       0, 1,    0x35,    0x07,       3, -1], # 大話西游 [restored, no good dump known]
"0A244228": [  164,       0,       0, 1,    0x35,    0x07,       3, -1], # 岳飛傳 [restored, no good dump known]

# Mapper 558: Waixing FS??? and Yancheng YC-03-09 PCB
"E65A8C08": [  558,       0,       0, 1,    0x30,    0x07,       3, -1], # 大話三國
"228559A7": [  558,       0,       0, 1,    0x30,    0x07,       3, -1], # 口袋精靈: 水晶
"8B41D49C": [  558,       0,       0, 1,    0x70,    0x07,       3, -1], # Pet Evolve - 宠物进化史
"5622EC51": [  558,       0,       1, 1,    0x70,    0x07,       3, -1], # 三国志 II 代
"E7AEB114": [  558,       0,       0, 1,    0x70,    0x07,       3, -1], # 口袋妖怪: 鑽石版
"8EB4BB51": [  558,       0,       0, 1,    0x70,    0x07,       3, -1], # 口袋精靈: 紅
"48244391": [  558,       0,       0, 1,    0x70,    0x07,       3, -1], # 宠物: 红
"01A24301": [  558,       0,       0, 1,    0x70,    0x07,       3, -1], # 数码宝贝
"88CB68A7": [  558,       0,       0, 1,    0x70,    0x07,       3, -1], # 数码暴龙 4: 水晶版
"081CAAFF": [  558,       0,       0, 1,    0x70,    0x07,       3, -1], # 盟军敢死队 - Commandos

# Mapper 176
"ffde0de5": [  176,      -1,      -1, 1,      -1,      -1,      -1, -1], # 梦幻之星4[简体](修正)一战一级.nes
"E0ED68B1": [  176,       0,      -1, 0,      -1,      -1,      -1, -1], # (AA-6109) 4-in-1.nes
"C447B9FE": [  176,       0,      -1, 0,      -1,      -1,      -1, -1], # (BRC-4) Super 4-in-1.nes
"922D16FD": [  176,       0,      -1, 0,      -1,      -1,      -1, -1], # (FK-9087) Super Game 3-in-1.nes
"C18A7BCB": [  176,       0,      -1, 0,      -1,      -1,      -1, -1], # (KT-443B) 4-in-1.nes
"43BDB0FF": [  176,       0,      -1, 0,      -1,      -1,      -1, -1], # (KT-8095) Super Game 4-in-1.nes
"719CCE0A": [  176,       0,      -1, 0,      -1,      -1,      -1, -1], # (KT-8394) 4-in-1.nes
"12DDA422": [  176,       0,      -1, 0,      -1,      -1,      -1, -1], # (KT-8404) Super Game 4-in-1.nes
"AB09C88B": [  176,       0,      -1, 0,      -1,      -1,      -1, -1], # (KT-8405) 4-in-1.nes
"A2E2031C": [  176,       0,      -1, 0,      -1,      -1,      -1, -1], # (KT-8406) 4-in-1.nes
"6063F4D5": [  176,       0,      -1, 0,      -1,      -1,      -1, -1], # (KT-A) Super Game.nes
"D2F5F51D": [  176,       0,      -1, 0,      -1,      -1,      -1, -1], # (KY-1008) 10-in-1.nes
"BF31508D": [  176,       0,      -1, 0,      -1,      -1,      -1, -1], # (KY-1102) Super 11-in-1.nes
"2B882971": [  176,       0,      -1, 0,      -1,      -1,      -1, -1], # (KY-1501) 999999-in-1.nes
"981AE6BB": [  176,       0,      -1, 0,      -1,      -1,      -1, -1], # (KY-1901) 19-in-1.nes
"5D2129AC": [  176,       0,      -1, 0,      -1,      -1,      -1, -1], # (KY-2001) 20-in-1.nes
"9AD9A8E9": [  176,       0,      -1, 0,      -1,      -1,      -1, -1], # (KY-6006) 6-, 4-in-1.nes
"521AF87B": [  176,       0,      -1, 0,      -1,      -1,      -1, -1], # (KY-6009) 6-in-1 (FK23C).nes
"AA05C592": [  176,       0,      -1, 0,      -1,      -1,      -1, -1], # (KY-6011) Super 6-in-1.nes
"205CF073": [  176,       0,      -1, 0,      -1,      -1,      -1, -1], # (KY-8002) 8-in-1 Supergame.nes
"D408F0BC": [  176,       0,      -1, 0,      -1,      -1,      -1, -1], # (KY-9005) 9-in-1 (FK23C).nes
"205D476E": [  176,       0,      -1, 0,      -1,      -1,      -1, -1], # (KY-9006) 9-in-1 Super Game.nes
"0B2CF73F": [  176,       0,      -1, 0,      -1,      -1,      -1, -1], # (MK-035) 3-in-1.nes
"2D1C667A": [  176,       0,      -1, 0,      -1,      -1,      -1, -1], # (MK-036) 3-in-1.nes
"25D7CB92": [  176,       0,      -1, 0,      -1,      -1,      -1, -1], # (MK-038) 4-in-1.nes
"8EFF9139": [  176,       0,      -1, 0,      -1,      -1,      -1, -1], # (MK-042) Super Game 3-in-1.nes
"B55103AD": [  176,       0,      -1, 0,      -1,      -1,      -1, -1], # (MK-042) Super Game 3-in-1 [game name correction]
"E83E5726": [  176,       0,      -1, 0,      -1,      -1,      -1, -1], # (MK-064) Super Game 4-in-1.nes
"BEEB0B07": [  176,       0,      -1, 0,      -1,      -1,      -1, -1], # (MK-064) Super Game 4-in-1 [game name correction].nes
"BBFBECBC": [  176,       0,      -1, 0,      -1,      -1,      -1, -1], # (SB-04) 4-in-1.nes
"2A1D5933": [  176,       0,      -1, 0,      -1,    0x07,      -1, -1], # (SC-03) Super 24-in-1.nes
"07564AC6": [  176,       0,      -1, 0,      -1,    0x07,      -1, -1], # (SD-01) 絡克家族大集合 - Rockman 16-in-1.nes
"D7D2123C": [  176,       0,      -1, 0,      -1,      -1,      -1, -1], # (VT-087) 4-in-1.nes
"8CB6D32C": [  176,       0,      -1, 0,      -1,      -1,      -1, -1], # (VT-089) 4-in-1.nes
"23994975": [  176,       0,      -1, 0,      -1,      -1,      -1, -1], # (VT-089) 4-in-1 [Russian].nes
"93196E95": [  176,       0,      -1, 0,      -1,      -1,      -1, -1], # (VT-208) 4-in-1.nes
"ECFE604A": [  176,       0,      -1, 0,      -1,      -1,      -1, -1], # (YH-2001) 2-in-1 Mortal-Kombat.nes
"A232AA4E": [  176,       0,      -1, 0,      -1,      -1,      -1, -1], # (YH-3128) Super Game 3-in-1.nes
"F565C023": [  176,       0,      -1, 0,      -1,      -1,      -1, -1], # (YH-3132) Super Game 3-in-1.nes
"01131224": [  176,       0,      -1, 0,      -1,      -1,      -1, -1], # (YH-3133) Super Game 3-in-1.nes
"7A76F2B9": [  176,       0,      -1, 0,      -1,      -1,      -1, -1], # (YH-3135) Super Game 3-in-1.nes
"FCACB02A": [  176,       0,      -1, 0,      -1,      -1,      -1, -1], # (YH-346) 3-in-1.nes
"5ADDD942": [  176,       0,      -1, 0,      -1,      -1,      -1, -1], # (YH-360) Super Game 3-in-1.nes
"940933DC": [  176,       0,      -1, 0,      -1,      -1,      -1, -1], # (YH-363) 3-in-1.nes
"C06C9AF7": [  176,       0,      -1, 0,      -1,      -1,      -1, -1], # (YH-370) Super Game 3-in-1.nes
"728CA4AF": [  176,       0,      -1, 0,      -1,      -1,      -1, -1], # (YH-401) 4-in-1.nes
"F019BFEF": [  176,       0,      -1, 0,      -1,      -1,      -1, -1], # (YH-415) Super Game 4-in-1
"4D25A3A6": [  176,       0,      -1, 0,      -1,      -1,      -1, -1], # (YH-415) Super Game 4-in-1 [game name correction]
"5F96184B": [  176,       0,      -1, 0,      -1,      -1,      -1, -1], # (YH-4102) 4-in-1.nes [bad CHR]
"C4D1D2F8": [  176,       0,      -1, 0,      -1,      -1,      -1, -1], # (YH-4102) 4-in-1.nes
"6D580074": [  176,       0,      -1, 0,      -1,      -1,      -1, -1], # (YH-4118) 4-in-1.nes
"FA5B1D26": [  176,       0,      -1, 0,      -1,      -1,      -1, -1], # (YH-4122) 4-in-1 [b].nes
"4410BD8D": [  176,       0,      -1, 0,      -1,      -1,      -1, -1], # (YH-4126) Super Game 4-in-1.nes
"C3166E11": [  176,       0,      -1, 0,      -1,      -1,      -1, -1], # (YH-4146) 4-in-1.nes
"BE05120A": [  176,       0,      -1, 0,      -1,      -1,      -1, -1], # (YH-417) Super Game 4-in-1.nes
"43BDB0FF": [  176,       0,      -1, 0,      -1,      -1,      -1, -1], # (YH-4219) Super Game 4-in-1.nes
"F371BCF2": [  176,       0,      -1, 0,      -1,      -1,      -1, -1], # (YH-4228) Super Game 4-in-1.nes
"FA1CB05C": [  176,       0,      -1, 0,      -1,      -1,      -1, -1], # (YH-4237) 4-in-1.nes
"44F46BBC": [  176,       0,      -1, 0,      -1,      -1,      -1, -1], # (YH-4239) New 4-in-1 Supergame.nes
"0A486DD7": [  176,       0,      -1, 0,      -1,      -1,      -1, -1], # (YH-4242) Super Game 4-in-1.nes
"634036DB": [  176,       0,      -1, 0,      -1,      -1,      -1, -1], # (YH-4246) Super Game 4-in-1.nes
"D7A8AFCA": [  176,       0,      -1, 0,      -1,      -1,      -1, -1], # (YH-4247) Super Game 4-in-1.nes
"6AB68F4F": [  176,       0,      -1, 0,      -1,      -1,      -1, -1], # (YH-4248) Super Game 4-in-1.nes
"1EF30CC8": [  176,       0,      -1, 0,      -1,      -1,      -1, -1], # (YH-4253) 4-in-1.nes
"4D18054C": [  176,       0,      -1, 0,      -1,      -1,      -1, -1], # (YH-437) Super Game 4-in-1.nes
"18DD93BC": [  176,       0,      -1, 0,      -1,      -1,      -1, -1], # (YH-437) Super Game 4-in-1 [game name correction].nes
"63D43F22": [  176,       0,      -1, 0,      -1,      -1,      -1, -1], # (YH-451) 4-in-1.nes
"BF31508D": [  176,       0,      -1, 0,      -1,      -1,      -1, -1], # (YH-469) Super 4-in-1.nes
"981AE6BB": [  176,       0,      -1, 0,      -1,      -1,      -1, -1], # (YH-480) 4-in-1.nes
"5D2129AC": [  176,       0,      -1, 0,      -1,      -1,      -1, -1], # (YH-481) 4-in-1.nes
"5D061E04": [  176,       0,      -1, 0,    0x07,      -1,      -1, -1], # (YH-602) 6-in-1.nes
"D2A4A9C6": [  176,       0,      -1, 0,    0x07,      -1,      -1, -1], # (YH-602) Super Game 6-in-1 [game name correction, font change].nes
"35974F2C": [  176,       0,      -1, 0,    0x07,      -1,      -1, -1], # (YH-602) Super Game 6-in-1 [font change].nes
"5B17FB27": [  176,       0,      -1, 0,    0x07,      -1,      -1, -1], # (YH-801) Super Game 8-in-1.nes
"0881169E": [  176,       0,      -1, 0,    0x07,      -1,      -1, -1], # (YH-801) Super Game 8-in-1 [font change].nes
"CD55A865": [  176,       0,      -1, 0,      -1,      -1,      -1, -1], # (YH-8023) 4-in-1.nes
"92B52357": [  176,       0,      -1, 0,      -1,      -1,      -1, -1], # (YH-8028) 4-in-1.nes
"DE94B7FD": [  176,       0,      -1, 0,      -1,      -1,      -1, -1], # (YH-8029) 4-in-1.nes	
"D56F27AE": [  176,       0,      -1, 0,      -1,      -1,      -1, -1], # (YH-8046) 4-in-1.nes
"0E8766DC": [  176,       0,      -1, 0,      -1,      -1,      -1, -1], # (YH-8049) 4-in-1.nes
"05E43745": [  176,       0,      -1, 0,    0x07,      -1,      -1, -1], # (YH-904) Super Game 9-in-1.nes
"31CADEF3": [  176,       0,      -1, 0,    0x07,      -1,      -1, -1], # (YH-904) Super Game 9-in-1 [font change].nes
"04398A9F": [  176,       0,      -1, 0,      -1,      -1,      -1, -1], # (卡聖 NT-945) 2-in-1.nes
"9BDF2424": [  176,       0,      -1, 0,      -1,      -1,      -1, -1], # 12-in-1 Console TV Game Cartridge.nes
"629FBEEC": [  176,       0,      -1, 0,      -1,      -1,      -1, -1], # 125-in-1.nes
"6EDED153": [  176,       0,      -1, 0,    0x07,      -1,      -1, -1], # 16-in-1 Battle Hymn of the Republic.nes
"68EDFDEA": [  176,       0,      -1, 0,      -1,      -1,      -1, -1], # 3-in-1 (Mortal Kombat MK5).nes
"8E994BCD": [  176,       0,      -1, 0,      -1,      -1,      -1, -1], # 3-in-1 (The Lion King 5).nes
"104D84DF": [  176,       0,      -1, 0,      -1,      -1,      -1, -1], # 3in1 (ES-Q800C) (Unl).nes
"DC904F4C": [  176,       0,      -1, 0,      -1,      -1,      -1, -1], # 3in1 (ES-Q800C)(FSS).nes
"CF7C6AE0": [  176,       0,      -1, 0,      -1,      -1,      -1, -1], # 3in1 (ES-Q800C)(TRS).nes
"4E5F123A": [  176,       0,      -1, 0,      -1,      -1,      -1, -1], # 4-in-1 (Digital Adventure).nes
"7DB2D1F5": [  176,       0,      -1, 0,      -1,      -1,      -1, -1], # 6-in-1 (Spiderman 2, Aladdin).nes
"75DFF1A4": [  176,       0,      -1, 0,      -1,    0x07,      -1, -1], # 6-in-1 (Super Rockman).nes
"ACCC9036": [  176,       0,      -1, 0,      -1,      -1,      -1, -1], # 9-in-1 (Pokemon Yellow).nes
"674D6CDE": [  176,       0,      -1, 0,      -1,      -1,      -1, -1], # Intellivision X2 15-in-1 [missing CHR data].nes
"0F907E7F": [  176,       0,      -1, 0,      -1,    0x07,      -1, -1], # Rockman 6-in-1 (rev0).nes
"E6D869ED": [  176,       0,      -1, 0,      -1,    0x07,      -1, -1], # Rockman 6-in-1 (rev0)[missing CHR data].nes
"88E0C48C": [  176,       0,      -1, 0,      -1,    0x07,      -1, -1], # Rockman 6-in-1 (rev1).nes
"EECF01BF": [  176,       0,      -1, 0,      -1,    0x07,      -1, -1], # Rockman 6-in-1 (rev3).nes
"C043A8DF": [  176,       0,      -1, 0,      -1,      -1,      -1, -1], # 小博士教育软件系列꞉ 数学 小狀元.nes
"2CF5DB05": [  176,       0,      -1, 0,      -1,      -1,      -1, -1], # 小博士教育软件系列꞉ 智力 小狀元.nes
"02C41438": [  176,       0,      -1, 1,    0x70,    0x07,      -1, -1], # 星河战士.nes
"A39C9A6B": [  176,       0,      -1, 0,      -1,      -1,      -1, -1], # (KY-1203) Super Game 12-in-1/(YH-467) Super Game 4-in-1
"0F05C0E6": [  176,       0,      -1, 0,    0x07,      -1,      -1, -1], # (YH-701) Super Game 4-in-1
"80F1E11E": [  176,       0,      -1, 0,    0x07,      -1,      -1, -1], # (YH-701) Super Game 4-in-1 [game name correction, font change]
"C5C30EFA": [  176,       0,      -1, 0,      -1,      -1,      -1, -1], # (YH-8011) Super Game 4-in-1
"8858D3F7": [  176,       0,      -1, 0,      -1,      -1,      -1, -1], # (YH-8011) Super Game 4-in-1 [game name correction]
"18BEB276": [  176,       0,      -1, 0,      -1,      -1,      -1, -1], # (YH-8024) Super Game 4-in-1
"E3A0B9E5": [  176,       0,      -1, 0,      -1,      -1,      -1, -1], # (YH-8027) Super Game 4-in-1
"5A0E3E69": [  176,       0,      -1, 0,      -1,      -1,      -1, -1], # (YH-8033) Super Game 4-in-1
"21D4484A": [  176,       0,      -1, 0,      -1,      -1,      -1, -1], # (YH-8042) Super Game 4-in-1
"F011AFD6": [    4,       0,      -1, 0,    0x70,    0x07,      -1, -1], # Rockman 4- Minus Infinity [Infinite Life]. Basically oversize MMC3 that fails to enable WRAM before accessing it.
"576D9589": [  176,       1,      -1, 0,      -1,      -1,      -1, -1], # (BS-0210A PCB) Super Mario 4-in-1.nes
"01B3EDD2": [  176,       1,      -1, 0,      -1,      -1,      -1, -1], # (BS-6002) Super Game 16-in-1.nes
"291F5318": [  176,       1,      -1, 0,      -1,      -1,      -1, -1], # (BS-6008) 210-in-1.nes
"B12CCB95": [  176,       1,      -1, 0,      -1,      -1,      -1, -1], # (BS-6028) 180-in-1.nes
"D5281CF3": [  176,       1,      -1, 0,      -1,      -1,      -1, -1], # (BS-8002) New 4-in-1.nes
"1F6AC22E": [  176,       1,      -1, 0,      -1,      -1,      -1, -1], # (BS-8005) Super Game 3-in-1.nes
"B48D6F26": [  176,       1,      -1, 0,      -1,      -1,      -1, -1], # (BS-8008) Super Game 4-in-1.nes
"BDDA85CE": [  176,       1,      -1, 0,      -1,      -1,      -1, -1], # (BS-8013) Super Game 4-in-1.nes
"85C2CC88": [  176,       1,      -1, 0,      -1,      -1,      -1, -1], # (BS-8029) Super Game 4-in-1.nes
"4F2CCD03": [  176,       1,      -1, 0,      -1,      -1,      -1, -1], # (BS-8029) Super Game 4-in-1 [overdump]
"40182FF6": [  176,       1,      -1, 0,      -1,      -1,      -1, -1], # (BS-8045) Super Game 4-in-1.nes
"F4CE452C": [  176,       1,      -1, 0,      -1,      -1,      -1, -1], # (BS-8062) Super Game 4-in-1.nes
"409601A5": [  176,       1,      -1, 0,      -1,      -1,      -1, -1], # (BS-8088) 4-in-1.nes
"C8AB31FD": [  176,       1,      -1, 0,      -1,      -1,      -1, -1], # (BS-8103) Super Game 4-in-1.nes
"87308F9D": [  176,       1,      -1, 0,      -1,      -1,      -1, -1], # (BS-8105) Super Game 4-in-1.nes
"95ACA7A7": [  176,       1,      -1, 0,      -1,      -1,      -1, -1], # (FK-012) 80-in-1.nes
"E650EC91": [  176,       1,      -1, 0,      -1,      -1,      -1, -1], # (FK-014) 128-in-1.nes
"7642F6B6": [  176,       1,      -1, 0,      -1,      -1,      -1, -1], # (FK-021) 180-in-1.nes
"24762CE8": [  176,       1,      -1, 0,      -1,      -1,      -1, -1], # (FK-022) 178-in-1.nes
"761CF0C0": [  176,       1,      -1, 0,      -1,      -1,      -1, -1], # (FK-026) 210-in-1.nes
"165102DB": [  176,       1,      -1, 0,      -1,      -1,      -1, -1], # (FK-033) 52-in-1.nes
"6614D4C0": [  176,       1,      -1, 0,      -1,      -1,      -1, -1], # (FK-037) Super Game 16-in-1.nes
"B3277B6C": [  176,       1,      -1, 0,      -1,      -1,      -1, -1], # (FK-3004) 6-in-1.nes
"3907578B": [  176,       1,      -1, 0,      -1,      -1,      -1, -1], # (FK-8008) 4-in-1.nes
"A391549D": [  176,       1,      -1, 0,      -1,      -1,      -1, -1], # (FK-8021) 4-in-1.nes
"36C27AE8": [  176,       1,      -1, 0,      -1,      -1,      -1, -1], # (FK-8021) 4-in-1.nes [PRG banks missing]
"06D13D9E": [  176,       1,      -1, 0,      -1,      -1,      -1, -1], # (FK-8026) 4-in-1.nes
"2EBD5FD6": [  176,       1,      -1, 0,      -1,      -1,      -1, -1], # (FK-8033) 4-in-1.nes
"23E4906A": [  176,       1,      -1, 0,      -1,      -1,      -1, -1], # (FK-8043) 4-in-1.nes
"10155A92": [  176,       1,      -1, 0,      -1,      -1,      -1, -1], # (FK-8045) 4-in-1.nes
"F66944EE": [  176,       1,      -1, 0,      -1,      -1,      -1, -1], # (FK-8050) 4-in-1.nes
"8BAEEDC0": [  176,       1,      -1, 0,      -1,      -1,      -1, -1], # (FK-8052) 4-in-1.nes
"39307391": [  176,       1,      -1, 0,      -1,      -1,      -1, -1], # (FK-8056) 4-in-1.nes
"72CEAB1E": [  176,       1,      -1, 0,      -1,      -1,      -1, -1], # (FK-8078) 4-in-1.nes
"05A1F101": [  176,       1,      -1, 0,      -1,      -1,      -1, -1], # (FK-8078) 4-in-1 [bad CHR].nes
"07D3F6CB": [  176,       1,      -1, 0,      -1,      -1,      -1, -1], # (FK-8079) 4-in-1.nes
"4E34CC0A": [  176,       1,      -1, 0,      -1,      -1,      -1, -1], # (FK-8161) Super Game 12-in-1.nes
"20379331": [  176,       1,      -1, 0,      -1,      -1,      -1, -1], # (FK-8244) 7-in-1.nes
"622E9E35": [  176,       1,      -1, 0,      -1,      -1,      -1, -1], # (FK-xxx) 126-in-1.nes
"666E736D": [  176,       1,      -1, 0,      -1,      -1,      -1, -1], # (FK-xxx) Super 15-in-1.nes
"19195C7F": [  176,       1,      -1, 0,      -1,      -1,      -1, -1], # (FK-xxx) Super 35-in-1.nes
"6F775C1A": [  176,       1,      -1, 0,      -1,      -1,      -1, -1], # (FK-xxx) Super 8-in-1.nes
"17D43AF9": [  176,       1,      -1, 0,      -1,      -1,      -1, -1], # (K-5003) 5-in-1.nes
"FE384E4B": [  176,       1,      -1, 0,      -1,      -1,      -1, -1], # (KB-0306N PCB) Super Game 4-in-1 #1.nes
"425F5325": [  176,       1,      -1, 0,      -1,      -1,      -1, -1], # (KB-0306N PCB) Super Game 4-in-1 #2.nes
"D206A6DF": [  176,       1,      -1, 0,      -1,      -1,      -1, -1], # (KB-4004) Super 4-in-1.nes
"F350556E": [  176,       1,      -1, 0,      -1,      -1,      -1, -1], # (KB-4009) Super 4-in-1.nes
"14BD2D1C": [  176,       1,      -1, 0,      -1,      -1,      -1, -1], # (KB-4016) Super 4-in-1 [fix Legend of Kage].nes
"BB9258BE": [  176,       1,      -1, 0,      -1,      -1,      -1, -1], # (KB-4016) Super 4-in-1.nes
"6343E6A6": [  176,       1,      -1, 0,      -1,      -1,      -1, -1], # (KD-1512 PCB) 128-in-1.nes
"91A8CD7E": [  176,       1,      -1, 0,      -1,      -1,      -1, -1], # (KD-1512 PCB) Super 20-in-1.nes
"E31220BE": [  176,       1,      -1, 0,      -1,      -1,      -1, -1], # (KD-6020) Super Game 131-in-1.nes
"B623C3D0": [  176,       1,      -1, 0,      -1,      -1,      -1, -1], # (KD-6024) Super Game 168-in-1.nes
"30FFB076": [  176,       1,      -1, 0,      -1,      -1,      -1, -1], # (KD-6033) 7-in-1.nes
"F0C581B3": [  176,       1,      -1, 0,      -1,      -1,      -1, -1], # (KG-6009) Super Game 58-in-1.nes
"576EC760": [  176,       1,      -1, 0,      -1,      -1,      -1, -1], # (KT-3445) Super 4-in-1.nes
"BF015F20": [  176,       1,      -1, 0,      -1,      -1,      -1, -1], # (KT-3445A-1) Super 4-in-1.nes
"0163CA53": [  176,       1,      -1, 0,      -1,      -1,      -1, -1], # (KT-3445A-B) 4-in-1.nes
"3DF39CE4": [  176,       1,      -1, 0,      -1,      -1,      -1, -1], # (KT-4403) Super 3-in-1.nes
"B8FED144": [  176,       1,      -1, 0,      -1,      -1,      -1, -1], # (KT-8017) Super Game 4-in-1.nes
"10E7B6ED": [  176,       1,      -1, 0,      -1,      -1,      -1, -1], # (KT-8040) Super Game 4-in-1.nes
"38E63310": [  176,       1,      -1, 0,      -1,      -1,      -1, -1], # (KT-8109) Super Game 4-in-1.nes
"C6D97331": [  176,       1,      -1, 0,      -1,      -1,      -1, -1], # (BS-0210A PCB) 4-in-1.nes
"9367D1F4": [  176,       1,      -1, 0,      -1,      -1,      -1, -1], # (BS-0210A PCB) 4-in-1 [Russian].nes
"E6617BF1": [  176,       1,      -1, 0,      -1,      -1,      -1, -1], # (SJ-0027) 180-in-1.nes
"9206B787": [  176,       1,      -1, 0,      -1,      -1,      -1, -1], # (YE-C011) 60-in-1.nes
"32CDAD83": [  176,       1,      -1, 0,      -1,      -1,      -1, -1], # (晶太 JY-222) 1998 97格鬥天王 激鬥篇 6-in-1.nes
"3C894AD1": [  176,       0,      -1, 0,      -1,      -1,      -1, -1], # (晶太 JY-224) 1998 97格鬥天王 激鬥篇 7-in-1.nes
"78CE996D": [  176,       1,      -1, 0,    0x07,      -1,      -1, -1], # (晶太 JY-225) 1998 97格鬥天王 激戰篇 6-in-1.nes
"B5D25A20": [  176,       1,      -1, 0,      -1,      -1,      -1, -1], # (BS-0306M PCB) Super 4-in-1.nes
"8C48BDBA": [  176,       1,      -1, 0,      -1,      -1,      -1, -1], # (BS-0306M PCB) Super 4-in-1 #2.nes
"C51FA465": [  176,       1,      -1, 0,      -1,      -1,      -1, -1], # (KD-6032) 180-in-1.nes
"A22DE99D": [  176,       1,      -1, 0,      -1,      -1,      -1, -1], # (KD-6026) Super Game 210-in-1
"E8BD5AC3": [  176,       1,      -1, 0,      -1,      -1,      -1, -1], # (BS-6017) Super Game 28-in-1
"37478F0C": [  176,       1,      -1, 0,      -1,      -1,      -1, -1], # (BS-8009) Super 4-in-1
"98C59170": [  176,       1,      -1, 0,      -1,      -1,      -1, -1], # (BS-8014) Super 4-in-1
"7CA43C89": [  176,       1,      -1, 0,      -1,      -1,      -1, -1], # (BS-8117) Super Game 4-in-1
"678DE5AA": [  176,       2,      -1, 0,      -1,    0x0C,       3, -1], # (外星) 120-in-1.nes
"6C979BAC": [  176,       2,      -1, 0,      -1,    0x0B,       3, -1], # (奔升) 10-in-1 Omake Game.nes
"E79F157E": [  176,       2,      -1, 0,      -1,    0x0B,       3, -1], # (福州 Coolboy) 245-in-1 Real Game.nes
"3AE35EC1": [  176,       2,      -1, 0,      -1,    0x0B,       3, -1], # (福州 Coolboy) 400-in-1 Real Game.nes
"D14617D7": [  176,       2,      -1, 0,      -1,    0x0B,       3, -1], # (福州 Coolboy) 合金装备 150-in-1 Real Game.nes
"37290B20": [  176,       2,      -1, 1,    0x90,    0x07,       3, -1], # Age of Empires - 帝国时代 (970493) [protection removed].nes
"5EE2EF97": [  176,       2,      -1, 1,    0x90,    0x07,       3, -1], # Age of Empires - 帝国时代 (970493).nes
"5077CAC1": [  176,       2,      -1, 1,    0x90,    0x07,       3, -1], # EverQuest - 八宝奇珠.nes
"F354D847": [  176,       2,      -1, 1,    0x90,    0x07,       3, -1], # Grandia - 格蘭蒂亞傳說 (2006SR04308).nes
"D51DD22C": [  176,       2,      -1, 1,    0x90,    0x07,       3, -1], # SD 高达外传 - 骑士高达故事 3꞉ 传说之骑士团.nes
"F29C8186": [  176,       2,      -1, 1,    0x90,    0x07,       3, -1], # Shanghai Tycoon - 上海大亨 (960313).nes
"F2398802": [  176,       2,      -1, 1,    0x90,    0x07,       3, -1], # 七龙珠 2꞉ 激战弗利萨!!.nes
"53B62838": [  176,       2,      -1, 1,    0x90,    0x07,       3, -1], # 七龙珠 2꞉ 电光石火.nes
"ED481B7C": [  176,       2,      -1, 1,    0x90,    0x07,       3, -1], # 七龙珠 Z 外传꞉ 赛亚人灭绝计划 (rev1).nes
"C768098B": [  176,       2,      -1, 1,    0x90,    0x07,       3, -1], # 三侠五义꞉ 御猫傳奇 (2006SR04307).nes
"B511C04B": [  176,       2,      -1, 1,    0x90,    0x07,       3, -1], # 三侠五义꞉ 御猫傳奇 (2006SR04307)[protection removed].nes
"44C20420": [  176,       2,      -1, 1,    0x90,    0x07,       3, -1], # 三国志 II - 中文加强版.nes
"8FFC1864": [  176,       2,      -1, 1,    0x90,    0x07,       3, -1], # 三国志 II꞉ 覇王の大陸.nes
"8F6AB5AC": [  176,       2,      -1, 1,    0x90,    0x07,       3, -1], # 三国忠烈传 - The Story of Three Kingdoms (FS005).nes
"94782FBD": [  176,       2,      -1, 1,    0x90,    0x07,       3, -1], # 三國志꞉ 雄霸天下 (980100026).nes
"99051CB5": [  176,       2,      -1, 1,    0x90,    0x07,       3, -1], # 三國志꞉ 雄霸天下 (980337).nes
"3A1CFE21": [  176,       2,      -1, 1,    0x90,    0x07,       3, -1], # 上古神殿.nes
"D6EA31C0": [  176,       2,      -1, 1,    0x90,      -1,       3, -1], # 东周列国志 (2006SR04301).nes
"03346083": [  176,       2,      -1, 1,    0x90,      -1,       3, -1], # 东周列国志 (2006SR04301)[protection removed].nes
"33443508": [  176,       2,      -1, 1,    0x90,      -1,       3, -1], # 争霸世纪 (960268).nes
"7696573A": [  176,       2,      -1, 1,    0x90,    0x07,       3, -1], # 亚特鲁战记.nes
"FE383376": [  176,       2,      -1, 1,    0x90,    0x07,       3, -1], # 仙剑神曲 - Space General [protection removed].nes
"97F3D7C1": [  176,       2,      -1, 1,    0x90,    0x07,       3, -1], # 仙剑神曲 - Space General.nes
"8264EA52": [  176,       2,      -1, 1,    0x90,      -1,       3, -1], # 侠客情꞉ 荆轲刺秦王 (FS005).nes
"027FD794": [  176,       2,      -1, 1,    0x90,      -1,       3, -1], # 剑舞者 - Sword Dancer.nes
"095D8678": [  176,       2,      -1, 1,    0x90,    0x07,       3, -1], # 双月传.nes
"D5F7AAEF": [  176,       2,      -1, 1,    0x90,    0x07,       3, -1], # 神风剑.nes
"377FDB36": [  176,       2,      -1, 1,    0x90,    0x07,       3, -1], # 口袋精靈꞉ 金.nes
"85DD49B6": [  176,       2,      -1, 1,    0x90,    0x07,       3, -1], # 口袋精靈꞉ 金 [protection removed].nes
"3F6124C3": [  176,       2,      -1, 1,    0x90,    0x07,       3, -1], # 哆啦A梦 - 超时空历险 [a].nes
"69A3CA5C": [  176,       2,      -1, 1,    0x90,    0x07,       3, -1], # 哆啦A梦 - 超时空历险.nes
"548D72FF": [  176,       2,      -1, 1,    0x90,    0x07,       3, -1], # 哥伦布 冒险记 - 黄金中文版.nes
"EEBEE0C8": [  176,       2,      -1, 1,    0x90,    0x07,       3, -1], # 哥伦布传 - 黄金中文版 (FS006).nes
"3C9DF646": [  176,       2,      -1, 1,    0x90,    0x07,       3, -1], # 圣斗士.nes
"52A5F554": [  176,       2,      -1, 1,    0x90,      -1,       3, -1], # 基督山恩仇记 - Le Comte de Monte Cristo (970260).nes
"35F8BD75": [  176,       2,      -1, 1,    0x90,      -1,       3, -1], # 基督山恩仇记 - Le Comte de Monte Cristo (970260)[protection removed].nes
"EE49F509": [  176,       2,      -1, 1,    0x90,    0x07,       3, -1], # 大盗伍佑卫门之天下宝藏.nes
"C04D330D": [  176,       2,      -1, 1,    0x90,      -1,       3, -1], # 夺宝小英豪꞉ 光明與黑暗傳説 (960270).nes
"848F2D69": [  176,       2,      -1, 1,    0x90,    0x07,       3, -1], # 女神的救赎.nes
"2C3D4EF0": [  176,       2,      -1, 1,    0x90,    0x07,       3, -1], # 宠物꞉ 翡翠 (2004SR05368).nes
"09FC02C7": [  176,       2,      -1, 1,    0x90,    0x07,       3, -1], # 宠物꞉ 翡翠 (2004SR05368)[protection removed].nes
"913B95F7": [  176,       2,      -1, 1,    0x90,    0x07,       3, -1], # 富甲三国 [protection removed].nes
"0F73D488": [  176,       2,      -1, 1,    0x90,    0x07,       3, -1], # 富甲三国.nes
"8947AB85": [  176,       2,      -1, 1,    0x90,    0x07,       3, -1], # 少年游侠꞉ 光明之神 - Young Chivalry (FS005).nes
"EE66C5E8": [  176,       2,      -1, 1,    0x90,      -1,       3, -1], # 帝国风暴 - Napoleon's War (980340) [protection removed].nes
"351DD533": [  176,       2,      -1, 1,    0x90,      -1,       3, -1], # 帝国风暴 - Napoleon's War (980340).nes
"EABBB630": [  176,       2,      -1, 1,    0x90,    0x07,       3, -1], # 怪物制造者 1.nes
"F1D40F5B": [  176,       2,      -1, 1,    0x90,    0x07,       3, -1], # 怪物制造者 2.nes
"2661109F": [  176,       2,      -1, 1,    0x90,      -1,       3, -1], # 戰國無雙 [protection removed].nes
"CC6E548C": [  176,       2,      -1, 1,    0x90,      -1,       3, -1], # 戰國無雙.nes
"BA29435A": [  176,       2,      -1, 1,    0x90,      -1,       3, -1], # 戰國群雄傳 (970498).nes
"5A88B5B0": [  176,       2,      -1, 1,    0x90,    0x07,       3, -1], # 数码暴龙 4꞉ 水晶版 (2004SR01259).nes
"A46353D1": [  176,       2,      -1, 1,    0x90,      -1,       3, -1], # 杨家将 - Yang's Troops (980186).nes
"C155128F": [  176,       2,      -1, 1,    0x90,      -1,       3, -1], # 梁山英雄传 [protection removed].nes
"AD223177": [  176,       2,      -1, 1,    0x90,      -1,       3, -1], # 梁山英雄传.nes
"BF6E95F5": [  176,       2,      -1, 1,    0x90,    0x07,       3, -1], # 梦境之王 - Dream Master.nes
"416C07A1": [  176,       2,      -1, 1,    0x90,    0x07,       3, -1], # 梦幻之星 IV.nes
"852CE16B": [  176,       2,      -1, 1,    0x90,      -1,       3, -1], # 楚漢爭霸 - The War Between Chu & Han (980005).nes
"E0DD8D77": [  176,       2,      -1, 1,    0x90,      -1,       3, -1], # 楚留香꞉ 香帥傳奇之血海飘零 (FS005).nes
"2E4F3051": [  176,       2,      -1, 1,    0x90,      -1,       3, -1], # 楚留香꞉ 香帥傳奇之血海飘零 (FS005)[protection removed].nes
"34AC5AE9": [  176,       2,      -1, 1,    0x90,      -1,       3, -1], # 水滸傳 - Marsh Outlaws (960415).nes
"1923A8C5": [  176,       2,      -1, 1,    0x90,    0x07,       3, -1], # 水滸神獸 [protection removed].nes
"6B4CAC80": [  176,       2,      -1, 1,    0x90,    0x07,       3, -1], # 水滸神獸.nes
"C9D968AF": [  176,       2,      -1, 1,    0x90,    0x07,       3, -1], # 混沌世界.nes
"7F3DBF1B": [  176,       2,      -1, 1,    0x90,    0x07,       1, -1], # 混沌世界 [VirtuaNES PAL].nes
"B616885C": [  176,       2,      -1, 1,    0x90,    0x07,       1, -1], # 混沌世界 [VirtuaNES   1, SRAM bank switch removed].nes
"F031E7CD": [  176,       2,      -1, 1,    0x90,    0x07,       3, -1], # 激战弗利萨之孙悟饭.nes
"D871D3E6": [  176,       2,      -1, 1,    0x90,    0x07,       3, -1], # 激战弗利萨之孙悟饭 [bad CHR].nes
"C35E9AA8": [  176,       2,      -1, 1,    0x90,    0x07,       3, -1], # 激战弗利萨之孙悟饭 [Traditional Chinese].nes
"BFF7C60C": [  176,       2,      -1, 1,    0x90,    0x07,       3, -1], # 激斗圣战士.nes
"8CAAFF73": [  176,       2,      -1, 1,    0x90,    0x07,       3, -1], # 爆笑三國 - Jacks in Tri-Empire (970261)(FS005).nes
"97B82F53": [  176,       2,      -1, 1,    0x90,    0x07,       3, -1], # 爆笑三國 - Jacks in Tri-Empire (970261)(FS005)[protection removed].nes
"95569A86": [  176,       2,      -1, 1,    0x90,    0x07,       3, -1], # 甲A - China Soccer League for Division A (980333).nes
"977D22C3": [  176,       2,      -1, 1,    0x90,    0x07,       3, -1], # 破釜沉舟 [protection removed].nes
"A2DC64FF": [  176,       2,      -1, 1,    0x90,    0x07,       3, -1], # 破釜沉舟.nes
"356A16E9": [  176,       2,      -1, 1,    0x90,      -1,       3, -1], # 絶代英雄 - Unrivalled Hero [protection removed].nes
"DA7D586D": [  176,       2,      -1, 1,    0x90,      -1,       3, -1], # 絶代英雄 - Unrivalled Hero.nes
"50D5F94B": [  176,       2,      -1, 1,    0x90,    0x07,       3, -1], # 英烈群侠传 (980332).nes
"F5C16B14": [  176,       2,      -1, 1,    0x90,    0x07,       3, -1], # 荣耀之骑士团 [protection removed].nes
"FB2563D1": [  176,       2,      -1, 1,    0x90,    0x07,       3, -1], # 荣耀之骑士团.nes
"F9863ADF": [  176,       2,      -1, 1,    0x90,    0x07,       3, -1], # 西楚霸王 (980100461)(FS005).nes
"3FF36623": [  176,       2,      -1, 1,    0x90,    0x07,       3, -1], # 西楚霸王 (990031).nes
"F1D803F3": [  176,       2,      -1, 1,    0x90,    0x07,       3, -1], # 西楚霸王 (990031)[protection].nes
"49F22159": [  176,       2,      -1, 1,    0x90,    0x07,       3, -1], # 超级 大富翁 - Super Rich (FS005).nes
"2DDA2835": [  176,       2,      -1, 1,    0x90,      -1,       3, -1], # 超级 大战略 [protection removed].nes
"7DCE29CB": [  176,       2,      -1, 1,    0x90,      -1,       3, -1], # 超级 大战略.nes
"0DBFF515": [  176,       2,      -1, 1,    0x90,    0x07,       3, -1], # 银白色沙加.nes
"38EFFD3E": [  176,       2,      -1, 1,    0x90,      -1,       3, -1], # 隋唐演义 (980185).nes
"CC56BCFE": [  176,       2,      -1, 1,    0x90,    0x07,       3, -1], # 風雲 - Traitor Legend (970100684)(FS005).nes
"FD883527": [  176,       2,      -1, 1,    0x90,    0x07,       3, -1], # 風雲 - Traitor Legend (980334).nes
"BC4AC7FE": [  176,       2,      -1, 1,    0x90,    0x07,       3, -1], # 高达骑士物语 2꞉ 光之骑士.nes
"B8FCD425": [  176,       2,      -1, 1,    0x90,    0x07,       3, -1], # 高达骑士物语.nes
"F17E49D5": [  176,       2,      -1, 1,    0x90,    0x07,       3, -1], # 鬼魅战记.nes
"4BCDB970": [  176,       2,      -1, 1,    0x90,    0x07,       3, -1], # 鬼魅战记.nes
"7D9C7206": [  176,       2,      -1, 1,    0x90,      -1,       3, -1], # 魔域英雄传 - Hero on Devil Lands (FS005).nes
"B16D4268": [  176,       2,      -1, 1,    0x90,      -1,       3, -1], # 魔法门之英雄无敌 (980584).nes
"71DAF1A6": [  176,       2,      -1, 1,    0x90,      -1,       3, -1], # 魔法门之英雄无敌 (980584)[protection removed].nes
"3532A114": [  176,       2,      -1, 1,    0x90,      -1,       3, -1], # 魔神法师 - Demogorgon Monk (FS005).nes
"E8EAFBC1": [  176,       2,      -1, 1,    0x90,    0x07,       3, -1], # 龙之谷 [protection removed].nes
"27DBC407": [  176,       2,      -1, 1,    0x90,    0x07,       3, -1], # 龙之谷.nes
"EBA09ADA": [  176,       2,      -1, 1,    0x90,    0x07,       3, -1], # 龙域天下.nes
"62DDE924": [  176,       2,      -1, 1,    0x90,    0x07,       3, -1], # 龙珠Z3꞉ 人造人列传.nes
"42161530": [  176,       2,      -1, 1,    0x90,    0x07,       3, -1], # 龙魂 [protection removed].nes
"8F8FC9A7": [  176,       2,      -1, 1,    0x90,    0x07,       3, -1], # 龙魂.nes
"B6984DAD": [  176,       3,      -1, 1,    0x07,    0x0B,      -1, -1], # Super Mario 160-in-1 Funny Time.nes
"AD82BBEA": [  176,       4,      -1, 1,    0x07,    0x07,      -1, -1], # GameStar Smart Genius Deluxe.nes

# Nanjing games that are sometimes found erroneously set to mapper 163 even though they use other mappers
"3CD15707": [  178,       0,       1, 1,    0x70,    0x07,       3, -1], # [NJ027] Fang Shi Yu (C)
"2779BB41": [  534,       0,       1, 0,      -1,    0x07,       3, -1], # [NJ064] Shu Du (Sudoku) (C)
"1DE558A1": [  178,       0,       1, 1,    0x70,    0x07,       3, -1], # [NJ085] Shan Shan De Hong Xing (C)
"F814EC57": [  178,       0,       1, 1,    0x70,    0x07,       3, -1], # [NJ090] Yong Zhe Chuan Shuo (C)
"53A1F436": [  178,       0,       1, 1,    0x70,    0x07,       3, -1], # [NJ091] Xian Jian Wen Qing (C)

# Non-Mapper 176 files that are sometimes found erroneously set to mapper 176
"60AC647F": [  260,       0,      -1, 0,      -1,      -1,      -1, -1], # (YH-4222) Super Game 4-in-1.nes

"558c0dc3": [  178,      -1,      -1, 0,      -1,      -1,      -1, -1], # Super 2-in-1 (Soccer Game & Crazy Dance) (Unl) [!].nes
"c68363f6": [  180,      -1,       0, 0,      -1,      -1,      -1, -1], # Crazy Climber (Japan).nes
"0f05ff0a": [  181,      -1,      -1, 0,      -1,      -1,      -1, -1], # Seicross (Japan) (Rev 1).nes
"96ce586e": [  189,      -1,       8, 0,      -1,      -1,      -1, -1], # Street Fighter II - The World Warrior (Unl) [!].nes
"0e76e4c1": [  190,      -1,       1, 0,      -1,      -1,      -1, -1], # Magic Kid Googoo (Korea) (Unl).nes
"555a555e": [  191,      -1,      -1, 1,      -1,      -1,      -1, -1], # Sugoro Quest - Dice no Senshitachi (Ch).nes
"2cc381f6": [  191,      -1,      -1, 1,      -1,      -1,      -1, -1], # Sugoro Quest - Dice no Senshitachi (Ch) [o1].nes
"a145fae6": [  192,      -1,      -1, 1,      -1,      -1,      -1, -1], # Young Chivalry (Ch) [b3].nes
"a9115bc1": [  192,      -1,      -1, 1,      -1,      -1,      -1, -1], # Young Chivalry (Ch) [!].nes
"4c7bbb0e": [  192,      -1,      -1, 1,      -1,      -1,      -1, -1], # Young Chivalry (Ch) [b2].nes
"98c1cd4b": [  192,      -1,      -1, 1,      -1,      -1,      -1, -1], # Ying Lie Qun Xia Zhuan (Asia) (Unl).nes
"ee810d55": [  192,      -1,      -1, 1,      -1,      -1,      -1, -1], # You Ling Xing Dong (Asia) (Unl).nes
"442f1a29": [  192,      -1,      -1, 1,      -1,      -1,      -1, -1], # Young Chivalry (Ch) [b1].nes
"637134e8": [  193,      -1,       0, 0,      -1,      -1,      -1, -1], # Fighting Hero (Asia) (Ja) (Unl).nes
"a925226c": [  194,      -1,      -1, 0,      -1,      -1,      -1, -1], # Dai-2-Ji - Super Robot Taisen (Ch) [b1].nes
"33c5df92": [  195,      -1,      -1, 0,      -1,      -1,      -1, -1], # Captain Tsubasa Vol. II - Super Striker (Ch) [a1].nes
"1bc0be6c": [  195,      -1,      -1, 0,      -1,      -1,      -1, -1], # Captain Tsubasa Vol. II - Super Striker (Ch) [a3].nes
"d5224fde": [  195,      -1,      -1, 1,      -1,      -1,      -1, -1], # God Slayer - Haruka Tenkuu no Sonata (Ch).nes
"fdec419f": [  196,      -1,      -1, 0,      -1,      -1,      -1, -1], # Street Fighter VI 16 Peoples (Unl) [!].nes
"700705f4": [  198,      -1,      -1, 0,      -1,      -1,      -1, -1], # ----
"9a2cf02c": [  198,      -1,      -1, 0,      -1,      -1,      -1, -1], # ----
"d8b401a7": [  198,      -1,      -1, 0,      -1,      -1,      -1, -1], # ----
"28192599": [  198,      -1,      -1, 0,      -1,      -1,      -1, -1], # ----
"19b9e732": [  198,      -1,      -1, 1,      -1,      -1,      -1, -1], # Tenchi wo Kurau II - Shokatsu Koumei Den (J) (PRG0) [T-Chi][b6].nes
"dd431ba7": [  198,      -1,      -1, 1,      -1,      -1,      -1, -1], # Tenchi wo kurau 2 (c)
"05658ded": [  201,      15,      -1, 0,      -1,      -1,      -1, -1], # 21-in-1 (CF-043) (2006-V) (Unl) [p1].nes
"276237b3": [  206,      -1,       0, 0,      -1,      -1,      -1, -1], # Karnov (Japan) (Rev 1).nes
"a5e6baf9": [  206,      -1,       1, 0,      -1,      -1,      -1, -1], # Dragon Slayer IV - Drasle Family (Japan).nes
"4f2f1846": [  206,      -1,       1, 0,      -1,      -1,      -1, -1], # Famista '89 - Kaimaku Ban!! (Japan).nes
"22d6d5bd": [  206,      -1,       1, 0,      -1,      -1,      -1, -1], # Jikuu Yuuden - Debias (Japan).nes
"9d21fe96": [  206,      -1,       1, 0,      -1,      -1,      -1, -1], # Lupin Sansei - Pandora no Isan (Japan).nes
"ae321339": [  206,      -1,       1, 0,      -1,      -1,      -1, -1], # Pro Yakyuu - Family Stadium '88 (Japan).nes
"96dfc776": [  206,      -1,       1, 0,      -1,      -1,      -1, -1], # R.B.I. Baseball 2 (USA) (Unl).nes
"fd63e7ac": [  206,      -1,       1, 0,      -1,      -1,      -1, -1], # R.B.I. Baseball 3 (USA) (Unl).nes
"2a01f9d1": [  206,      -1,       1, 0,      -1,      -1,      -1, -1], # Wagyan Land (Japan).nes
"7678f1d5": [  207,      -1,       8, 0,      -1,      -1,      -1, -1], # Fudou Myouou Den (Japan).nes
"07eb2c12": [  208,      -1,      -1, 0,      -1,      -1,      -1, -1], # FC25-台湾格斗游戏+快打传说.nes
"dd8ced31": [  209,      -1,      -1, 0,      -1,      -1,      -1, -1], # Power Rangers III (Unl) [!].nes
"063b1151": [  209,      -1,      -1, 0,      -1,      -1,      -1, -1], # Power Rangers IV (Unl) [!].nes
"dd4d9a62": [  209,      -1,      -1, 0,      -1,      -1,      -1, -1], # Shin Samurai Spirits 2 - Haoumaru Jigoku Hen (Ch).nes
"0c47946d": [  210,       1,       1, 0,      -1,      -1,      -1, -1], # Chibi Maruko-chan - Uki Uki Shopping (Japan).nes
"c247cc80": [  210,       1,       1, 1,    0x50,      -1,      -1, -1], # Family Circuit '91 (Japan) (En).nes
"808606f0": [  210,       1,       1, 0,      -1,      -1,      -1, -1], # Famista '91 (Japan).nes
"6ec51de5": [  210,       2,       0, 0,      -1,      -1,      -1, -1], # Famista '92 (Japan).nes
"adffd64f": [  210,       2,       0, 0,      -1,      -1,      -1, -1], # Famista '93 (Japan).nes
"429103c9": [  210,       2,       0, 0,      -1,      -1,      -1, -1], # Famista '94 (Japan).nes
"81b7f1a8": [  210,       1,       1, 0,      -1,      -1,      -1, -1], # Heisei Tensai Bakabon (Japan).nes
"2447e03b": [  210,       2,       0, 0,      -1,      -1,      -1, -1], # Top Striker (Japan).nes
"1dc0f740": [  210,       2,       0, 0,      -1,      -1,      -1, -1], # Wagyan Land 2 (Japan).nes
"d323b806": [  210,       2,       0, 0,      -1,      -1,      -1, -1], # Wagyan Land 3 (Japan).nes
"bd523011": [  210,       2,       0, 0,      -1,      -1,      -1, -1], # Namco Prism Zone - Dream Master (Japan).nes
"5daae69a": [  211,      -1,      -1, 0,      -1,      -1,      -1, -1], # Aladdin - Return of Jaffar, The (Unl) [!].nes
"1ec1dfeb": [  217,      -1,      -1, 0,      -1,      -1,      -1, -1], # 255-in-1 (Mapper 204) [p1].nes
"046d70cc": [  217,      -1,      -1, 0,      -1,      -1,      -1, -1], # ?? 500-in-1 (Anim Splash, Alt Mapper)[p1][!]
"12f86a4d": [  217,      -1,      -1, 0,      -1,      -1,      -1, -1], # 500-in-1.nes
"d09f778d": [  217,      -1,      -1, 0,      -1,      -1,      -1, -1], # ?? 9999999-in-1 (Static Splash, Alt Mapper)[p1][!]
"62ef6c79": [  232,      -1,       8, 0,      -1,      -1,      -1, -1], # Quattro Sports (Camerica) (Aladdin) [b1].nes
"2705eaeb": [  234,      -1,      -1, 0,      -1,      -1,      -1, -1], # Maxi 15 (USA) (Unl).nes
"80cbcacb": [  235,      -1,      -1, 0,      -1,      -1,      -1, -1], # 100-in-1 (Unl).nes
"6175b9a0": [  235,      -1,      -1, 0,      -1,      -1,      -1, -1], # 150_in_1_199x-ASp.nes
"745a6791": [  235,       2,      -1, 0,      -1,      -1,      -1, -1], # 210-in-1 and Contra 4-in-1 (212-in-1,212 Hong Kong,Reset Based)(Unl).nes
"df81364d": [  235,      -1,      -1, 0,      -1,      -1,      -1, -1], # 260-in-1 [p1][!].nes
"a38f2f1d": [  235,      -1,      -1, 0,      -1,    0x07,      -1, -1], # 1500-in-1.nes
"6f12afc5": [  235,      -1,      -1, 0,      -1,      -1,      -1, -1], # ?? Golden Game 150-in-1
"2537b3e6": [  241,      -1,      -1, 0,      -1,      -1,      -1, -1], # Dance Xtreme - Prima (Unl).nes
"11611e89": [  241,      -1,      -1, 0,      -1,      -1,      -1, -1], # Darkseed (Unl) [p1][b1].nes
"81a37827": [  241,      -1,      -1, 0,      -1,      -1,      -1, -1], # Darkseed (Unl) [p1].nes
"fb2b6b10": [  241,      -1,      -1, 1,      -1,      -1,      -1, -1], # Fan Kong Jing Ying (Asia) (Unl).nes
"b5e83c9a": [  241,      -1,      -1, 1,      -1,      -1,      -1, -1], # Xing Ji Zheng Ba (Asia) (Unl).nes
"368c19a8": [  241,      -1,      -1, 0,      -1,      -1,      -1, -1], # LIKO Study Cartridge 3-in-1 (Unl) [!].nes
"54d98b79": [  241,      -1,      -1, 1,      -1,      -1,      -1, -1], # Titanic 1912 (Unl).nes
"c2730c30": [   34,      -1,       0, 0,      -1,      -1,      -1, -1], # Deadly Towers (USA).nes
"a21e675c": [   34,      -1,       0, 0,      -1,      -1,      -1, -1], # Mashou (Japan).nes
"6bea1235": [  245,      -1,      -1, 1,      -1,      -1,      -1, -1], # Yong Zhe Dou E Long - Dragon Quest VI (Ch) [a1].nes
"345ee51a": [  245,      -1,      -1, 1,      -1,      -1,      -1, -1], # Yong Zhe Dou E Long - Dragon Quest VII (Ch).nes
"57514c6c": [  245,      -1,      -1, 1,      -1,      -1,      -1, -1], # Yong Zhe Dou E Long - Dragon Quest VI (Ch).nes
"db9d71b7": [  114,      -1,      -1, 0,      -1,      -1,      -1, -1], # Super Donkey Kong (Unl) [o1].nes
	
# KT-008 PCB. These require mapper 224 for having an outer bank register at $5xxx, but are usually found set to mapper 4.
"61fc4d20": [  224,      -1,       0, 1,    0x70,    0x07,       3, -1], # (KT-1062) 口袋怪兽꞉ 水晶版.nes
"aa666c19": [  224,      -1,       0, 1,    0x70,    0x07,       3, -1], # Ys Origin: Hugo.nes
"b0d011d3": [  224,      -1,       0, 1,    0x70,    0x07,       3, -1], # Ys Origin: Yunica.nes
"16143319": [  224,      -1,       0, 1,    0x70,    0x07,       3, -1], # Ys VI: 纳比斯汀的方舟.nes
"e05fc21f": [  224,      -1,       0, 1,    0x70,    0x07,       3, -1], # Ys: 菲尔盖纳之誓约.nes
"f8b58b59": [  224,      -1,       0, 1,    0x70,    0x07,       3, -1], # 三国志 - 蜀魏争霸.nes
"cb1bab3d": [  224,      -1,       0, 1,    0x70,    0x07,       3, -1], # 三国志꞉ 蜀汉风云.nes
"ddc122ed": [  224,      -1,       0, 1,    0x70,    0x07,       3, -1], # 亡灵崛起.nes
"4d2811c7": [  224,      -1,       0, 1,    0x70,    0x07,       3, -1], # 伏魔英雄传.nes
"24750e5d": [  224,      -1,       0, 1,    0x70,    0x07,       3, -1], # 傲视天地.nes
"c658b6a8": [  224,      -1,       0, 1,    0x70,    0x07,       3, -1], # 刀剑英雄传.nes
"ea831217": [  224,      -1,       0, 1,    0x70,    0x07,       3, -1], # 剑侠情缘.nes
"92ebad5b": [  224,      -1,       0, 1,    0x70,    0x07,       3, -1], # 勇者斗恶龙 III꞉ 罪恶渊源.nes
"48210324": [  224,      -1,       0, 1,    0x70,    0x07,       3, -1], # 勇者斗恶龙꞉ 勇者的试炼.nes
"3439d140": [  224,      -1,       0, 1,    0x70,    0x07,       3, -1], # 勇者斗恶龙꞉ 天空的新娘.nes
"9a0a531a": [  224,      -1,       0, 1,    0x70,    0x07,       3, -1], # 勇者斗恶龙꞉ 天空篇.nes
"b5fdb3cb": [  224,      -1,       0, 1,    0x70,    0x07,       3, -1], # 勇者黑暗世界 - 混沌世界.nes
"4f427110": [  224,      -1,       0, 1,    0x70,    0x07,       3, -1], # 口袋怪兽꞉ 珍珠版.nes
"a46d6f4c": [  224,      -1,       0, 1,    0x70,    0x07,       3, -1], # 口袋怪兽꞉ 琥珀版.nes
"e001de16": [  224,      -1,       0, 1,    0x70,    0x07,       3, -1], # 口袋怪兽꞉ 白金版.nes
"5464d7f8": [  224,      -1,       0, 1,    0x70,    0x07,       3, -1], # 口袋怪兽꞉ 翡翠版.nes
"4d735cb1": [  224,      -1,       0, 1,    0x70,    0x07,       3, -1], # 口袋怪兽꞉ 钻石版.nes
"5d04547c": [  224,      -1,       0, 1,    0x70,    0x07,       3, -1], # 失落的神器.nes
"cb524b42": [  224,      -1,       0, 1,    0x70,    0x07,       3, -1], # 征战天下.nes
"5f362198": [  224,      -1,       0, 1,    0x70,    0x07,       3, -1], # 战神世界.nes
"36de88e7": [  224,      -1,       0, 1,    0x70,    0x07,       3, -1], # 新魔界.nes
"f8e9c9cf": [  224,      -1,       0, 1,    0x70,    0x07,       3, -1], # 无双乱舞.nes
"a4c39535": [  224,      -1,       0, 1,    0x70,    0x07,       3, -1], # 神魔大陆.nes
"76bbe916": [  224,      -1,       0, 1,    0x70,    0x07,       3, -1], # 落日征战.nes
"9b518d54": [  224,      -1,       0, 1,    0x70,    0x07,       3, -1], # 轩辕剑꞉ 云的彼端.nes
"91396b3f": [  224,      -1,       0, 1,    0x70,    0x07,       3, -1], # 轩辕剑꞉ 天之痕.nes
"aa621fa0": [  224,      -1,       0, 1,    0x70,    0x07,       3, -1], # 轩辕剑꞉ 枫之舞.nes
"48d1f54a": [  224,      -1,       0, 1,    0x70,    0x07,       3, -1], # 轩辕剑꞉ 王者归来.nes
	
"bdbe3c96": [  238,      -1,       1, 0,      -1,      -1,      -1, -1], # Contra Fighter (Unl).nes
"cb53c523": [   11,      -1,       1, 0,      -1,      -1,      -1, -1], # King Neptune's Adventure (USA) (Unl).nes
"6e149729": [  189,      -1,      -1, 0,      -1,      -1,      -1, -1], # Master Fighter II (Unl) [a1].nes
"60bfeb0c": [   90,      -1,      -1, 0,      -1,      -1,      -1, -1], # Mortal Kombat 2 (Unl) [!].nes
"247cc73d": [  150,      -1,      -1, 0,      -1,      -1,      -1, -1], # Poker II (Asia) (Ja) (Unl).nes
"1f1326d4": [  121,      -1,      -1, 0,      -1,      -1,      -1, -1], # Super Sonic 5 (1997) (Unl) [!].nes
"99748230": [  215,      -1,      -1, 0,      -1,      -1,      -1, -1], # EarthWorm Jim 2 (SuperGame).nes
"37876ac7": [  215,      -1,      -1, 0,      -1,      -1,      -1, -1], # Golden Card 6-in-1 (Unl) [!].nes
"1a3320a3": [  215,      -1,      -1, 0,      -1,      -1,      -1, -1], # Mortal Kombat 3 (SuperGame).nes
"ec70f8d8": [  258,      -1,      -1, 0,      -1,      -1,      -1, -1], # 1997 留念: 港-京 - Super Hang-On (protected version, CHR doubled)
"224989d9": [  258,      -1,      -1, 0,      -1,      -1,      -1, -1], # 1997 留念: 港-京 - Super Hang-On (protected version)
"80eb1839": [  114,       1,      -1, 0,      -1,      -1,      -1, -1], # Boogerman (Sugar Softec) (Unl) [!].nes
"071e4ee8": [  114,      -1,      -1, 0,      -1,      -1,      -1, -1], # ?? m114,submapper 1 test rom
"fe3e03a1": [  197,      -1,      -1, 0,      -1,      -1,      -1, -1], # Mortal Kombat III Special (YY-030) (Ch) [!].nes
"9151d311": [  197,      -1,      -1, 0,      -1,      -1,      -1, -1], # Mortal Kombat III 28 Peoples (NT-328) (Ch) [!].nes
"272709b9": [  237,      -1,      -1, 0,      -1,      -1,      -1, -1], # Teletubbies Y2K (420-in-1).nes
"2e27e0af": [  227,      -1,      -1, 1,    0x70,    0x07,       3, -1], # Bio Hazard.nes
"0e7e9309": [  189,      -1,      -1, 0,      -1,      -1,      -1, -1], # Street Fighter II - The World Warrior (Unl) [a1].nes
"a3ac0095": [  189,      -1,      -1, 0,      -1,      -1,      -1, -1], # Street Fighter II - The World Warrior (Unl) [a2].nes
"eced5899": [  121,      -1,      -1, 0,      -1,      -1,      -1, -1], # Ultimate Mortal Kombat 4 (Unl) [!].nes
"19c1ed51": [  150,      -1,      -1, 0,      -1,      -1,      -1, -1], # Poker III (Asia) (Ja) (Unl).nes
"282745c5": [  141,      -1,      -1, 0,      -1,      -1,      -1, -1], # Q Boy (Asia) (Ja) (Unl).nes
"4b9ecfb2": [   21,      -1,      -1, 0,      -1,      -1,      -1, -1], # Wai Wai World 2 - SOS!! Paseri Jou (Japan) (Virtual Console).nes
"33751782": [    4,      -1,      -1, 1,      -1,      -1,      -1, -1], # Zoda's Revenge - StarTropics II (USA, Europe) (Virtual Console).nes
"9bbf3e5d": [   15,      -1,      -1, 0,      -1,      -1,      -1, -1], # 168-in-1 [p1][!].nes

# TXC / Sachen / JoyVan

# This cart does not work anymore with latest mapper implementation.
# This will be run using alternate mapper using override in m132
"2a5f4c5a": [  132,      -1,      -1, 0,      -1,      -1,      -1, -1], # Zhan Guo Si Chuan Sheng (C&E) (Unl).nes

"0acfc3cd": [  132,      -1,      -1, 0,      -1,      -1,      -1, -1], # Mahjong Block (MGC-008) (Unl) [!].nes

# ines mappers that uses iNes 2.0 numbers

# CRC32, mapperNum, SubMapper, Mirroring, hasBattery, prgRam, chrRam, region, extra flags
"f6bd8e31": [  281,      -1,       0, 0,      -1,      -1,      -1, -1], # 1997 Super HIK 4-in-1 (JY-052) [p1][!].nes
# dumps are assigned 281, but its 293 on nesdev
"06256C80": [  293,      -1,      -1, 0,      -1,    0x07,      -1, -1], # Super 12-in-1 NewStar (UNL)
"5aa23a15": [  361,      -1,       0, 0,      -1,      -1,      -1, -1], # 4-in-1 (OK-411)[p1][!].nes
"f6b9d088": [  366,      -1,       0, 0,      -1,      -1,      -1, -1], # 4-in-1 (K-3131GS, GN-45) [p1][!].nes
"503566b2": [  366,      -1,       0, 0,      -1,      -1,      -1, -1], # 4-in-1 (K-3131SS, GN-45) [p1][!].nes
"db2d2d88": [  369,      -1,      -1, 0,    0x07,      -1,      -1, -1], # Super Mario Bros. Party.nes
"87f83ea2": [  380,      -1,      -1, 0,      -1,    0x07,      -1, -1], # 42 to 80,000
"c4b94bd5": [  389,      -1,      -1, 0,      -1,      -1,      -1, -1], # Caltron - 9 in 1 (USA) (Proto) (Unl).nes
"2F497313": [  401,      -1,      -1, 0,      -1,      -1,      -1, -1], # Super 19-in-1 (VIP19)

# ines mappers that uses unif boards

"0073dbd8": [  260,      -1,      -1, 0,      -1,      -1,      -1, -1], # 2-in-1 - Mortal Kombat V Turbo 30 + Super Aladdin (Unl) [p1][!].nes
"4dc6107d": [  260,      -1,      -1, 0,      -1,      -1,      -1, -1], # 2-in-1 - Boogerman + Flintstones, The (Unl) [p1][!].nes
"b72b2cf4": [  260,      -1,      -1, 0,      -1,      -1,      -1, -1], # 2-in-1 - Aladdin + Lion King, The (Unl) [p1][!].nes
"5638ba59": [  260,      -1,      -1, 0,      -1,      -1,      -1, -1], # Mortal Kombat Trilogy - 8 People (M1274) (Ch) [!].nes
"a1dc16c0": [  262,      -1,      -1, 0,      -1,      -1,      -1, -1], # Street Heroes (Asia) (Ja) (Unl).nes
"1df10182": [  263,      -1,      -1, 0,      -1,      -1,      -1, -1], # Boogerman II (Rex-Soft) [!].nes
"f956fcea": [  521,      -1,      -1, 0,      -1,    0x70,      -1, -1], # Korean Igo (Korea) (Unl).nes
"2eed2e34": [  289,      -1,      -1, 0,      -1,      -1,      -1, -1], # 76-in-1 [p1][a1].nes 2048 PRG, 0 CHR
}

def analyzeRom(nesFile,fileName):
    mapper = -1
    PRGRamSize = 0
    PRGRamSaveSize = 0
    CHRRamSize = 0
    CHRRamSaveSize = 0
    ines2header = 0
    hasBattery = 0
    nesBytes = nesFile.read(32) # read 32 bytes for FDS files with headers
    try :
        idString = nesBytes[0:3].decode("utf-8",errors='ignore')
        if idString == "NES" and nesBytes[3] == 0x1a:
            nesFile.seek(16) # Seek to offset 16 for correct CRC32 value
            hash = 0
            while True:
                s = nesFile.read(65536)
                if not s:
                    break
                hash = zlib.crc32(s, hash)
                hashString = "%08x" % (hash & 0xFFFFFFFF)
            if ChecksumDict.get(hashString) != None:
                mapper = ChecksumDict.get(hashString)[0]
            if mapper == -1 :
                # Get mapper for iNes header
                ines2header = nesBytes[7] & 0xc == 8
                if ines2header: # NES 2.0 header
                    mapper = ((nesBytes[8] & 0x0f) << 8) + (nesBytes[7] & 0xf0) + ((nesBytes[6] & 0xf0) >> 4)
                else :
                    mapper = (nesBytes[7] & 0xf0) + ((nesBytes[6] & 0xf0) >> 4)
            # Get PRGRamSize and PRGRamSaveSize if NES 2.0 header
            if ines2header: # NES 2.0 header
                ROMType = nesBytes[7]
                hasBattery = ROMType & 2 == 2
                PRGRam = nesBytes[10]
                if PRGRam & 0x0F != 0:
                    PRGRamSize = 64 << ((PRGRam >> 0) & 0x0F)
                if PRGRam & 0xF0 != 0:
                    PRGRamSaveSize = 64 << ((PRGRam >> 4) & 0x0F)
                CHRRam = nesBytes[11]
                if CHRRam & 0x0F != 0:
                    CHRRamSize = 64 << ((CHRRam >> 0) & 0x0F)
                if CHRRam & 0xF0 != 0:
                    CHRRamSaveSize = 64 << ((CHRRam >> 4) & 0x0F)
            else:
                if ChecksumDict.get(hashString) != None:
                    ines2header = 1 # force ines2 header as we have prgRam info in database
                    hasBattery = ChecksumDict.get(hashString)[3]
                    PRGRam = ChecksumDict.get(hashString)[4]
                    if PRGRam & 0x0F != 0:
                        PRGRamSize = 64 << ((PRGRam >> 0) & 0x0F)
                    if PRGRam & 0xF0 != 0:
                        PRGRamSaveSize = 64 << ((PRGRam >> 4) & 0x0F)
                    CHRRam = ChecksumDict.get(hashString)[5]
                    if CHRRam & 0x0F != 0:
                        CHRRamSize = 64 << ((CHRRam >> 0) & 0x0F)
                    if CHRRam & 0xF0 != 0:
                        CHRRamSaveSize = 64 << ((CHRRam >> 4) & 0x0F)
        if mapper == -1: #check if it's a nsf file
            idString = nesBytes[0:4].decode("utf-8",errors='ignore')
            if idString == "NESM" and nesBytes[4] == 0x1a:
                mapper = -3
        if mapper == -1: #check if it's a FDS file
            idString = nesBytes[1:15].decode("utf-8")
            if idString == "*NINTENDO-HVC*":
                mapper = -2
            else: # check if it's a FDS file with header
                idString = nesBytes[17:31].decode("utf-8")
                if idString == "*NINTENDO-HVC*":
                    mapper = -2
    except Exception as e:
        mapper = -1
    return mapper,ines2header,PRGRamSize,PRGRamSaveSize,CHRRamSize,CHRRamSaveSize,hasBattery

def getSaveSize(mapper,fileSize,ines2header,PRGRamSize,PRGRamSaveSize,CHRRamSize,CHRRamSaveSize,hasBattery):
    size = 0
    #TODO : find reliable way to know exact save size
    if mapper == -3: # NSF
        size = 0
    elif mapper == -2: # FDS
        size = 46926
        sides = math.ceil(fileSize/65500)
        size+=sides*65508
    elif mapper == -1: # Unknown file format
        size = 0
    elif mapper == 0:
        size = 13726
    elif mapper == 1:
        workRAM = 8
        if ines2header:
            workRAM = (PRGRamSize + PRGRamSaveSize) / 1024
            if workRAM > 0 and workRAM < 8:
                workRAM = 8
            if workRAM > 32:
                workRAM = 32
        size = 13780 + workRAM * 1024
    elif mapper == 2:
        size = 21944
    elif mapper == 3:
        size = 13744
    elif mapper == 4:
        size = 13805
    elif mapper == 5:
        WRAMSize = 64
        if ines2header:
            WRAMSize = (PRGRamSize + PRGRamSaveSize) / 1024
            if WRAMSize and WRAMSize < 8:
                WRAMSize = 8
            elif WRAMSize > 64:
                WRAMSize = 64
        if WRAMSize == 0:
            size = 7133
        else:
            size = 7133 + 8 + 1024 * WRAMSize
    elif mapper == 6:
        size = 13796
    elif mapper == 7:
        size = 13744
    elif mapper == 8:
        size = 13744
    elif mapper == 9:
        size = 5574
    elif mapper == 10:
        size = 13744
    elif mapper == 11:
        size = 5544
    elif mapper == 12:
        size = 13815
    elif mapper == 13:
        size = 21936
    elif mapper == 14:
        size = 5666
    elif mapper == 15:
        size = 21945
    elif mapper == 16:
        size = 6035
    elif mapper == 17:
        size = 21996
    elif mapper == 18:
        size = 13796
    elif mapper == 19:
        size = 14151
    #20 Unsupported
    elif mapper == 21:
        size = 13907
    elif mapper == 22:
        size = 5707
    elif mapper == 23:
        size = 13907
    elif mapper == 24:
        size = 5760
    elif mapper == 25:
        size = 13907
    elif mapper == 26:
        size = 13960
    elif mapper == 27:
        size = 5536
    elif mapper == 28:
        size = 38347
    elif mapper == 29:
        size = 46520
    elif mapper == 30:
        size = 40376
    elif mapper == 32:
        size = 13763
    elif mapper == 33:
        size = 5580
    elif mapper == 34:
        size = 21937
    elif mapper == 35:
        if ines2header:
            WRAMSize = PRGRamSize + PRGRamSaveSize
        else:
            WRAMSize = 8192
        size = 5714 + WRAMSize
    elif mapper == 36:
        size = 13798
    elif mapper == 37:
        size = 13814
    elif mapper == 38:
        size = 5544
    #39 Unsupported
    elif mapper == 40:
        size = 5568
    elif mapper == 41:
        size = 5568
    elif mapper == 42:
        size = 5553
    #43 ?
    elif mapper == 44:
        size = 13814
    elif mapper == 45:
        size = 13814
    elif mapper == 46:
        size = 5544
    elif mapper == 47:
        size = 13814
    elif mapper == 48:
        size = 5580
    elif mapper == 49:
        size = 5614
    #50 ?
    elif mapper == 51:
        size = 13744
    elif mapper == 52:
        size = 13815
    elif mapper == 53:
        size = 13744
    #54 Unsupported
    #55 Unsupported
    #56 ?
    elif mapper == 57:
        size = 5545
    elif mapper == 58:
        size = 13736
    elif mapper == 59:
        size = 5545
    elif mapper == 60:
        size = 5535
    elif mapper == 61:
        size = 13736
    elif mapper == 62:
        size = 5545
    #63 ?
    #64 ?
    #65 ?
    #66 ?
    #67 ?
    #68 ?
    #69 ?
    #70 ?
    elif mapper == 71:
        size = 13744
    #72 ?
    #73 ?
    #74 ?
    #75 ?
    #76 ?
    #77 ?
    #78 ?
    #79 ?
    #80 ?
    #81 Unsupported
    #82 ?
    elif mapper == 83:
        size = 13805
    #84 Unsupported
    elif mapper == 85:
        size = 23984
    #86 ?
    #87 ?
    #88 ?
    #89 ?
    elif mapper == 90:
        if ines2header:
            WRAMSize = PRGRamSize + PRGRamSaveSize
        else:
            WRAMSize = 0
        size = 5714 + WRAMSize
    #91 ?
    #92 ?
    #93 ?
    #94 ?
    #95 ?
    #96 ?
    #97 ?
    #98 Unsupported
    #99 ?
    #100 Unsupported
    #101 ?
    #102 Unsupported
    #103 ?
    elif mapper == 104:
        size = 21936
    elif mapper == 105:
        size = 21972
    #106 ?
    #107 ?
    #108 ?
    #109 Unsupported
    #110 Unsupported
    #111 ?
    #112 ?
    elif mapper == 113:
        size = 5544
    #114 ?
    #115 ?
    #116 ?
    #117 ?
    #118 ?
    #119 ?
    #120 ?
    #121 ?
    #122 Unsupported
    #123 ?
    #124 Unsupported
    #125 ?
    #126 ?
    #127 Unsupported
    #128 Unsupported
    #129 Unsupported
    #130 Unsupported
    #131 Unsupported
    #132 ?
    #133 ?
    elif mapper == 134:
        WRAMSize = 8
        if ines2header:
            WRAMSize = (PRGRamSize + PRGRamSaveSize) / 1024
        if WRAMSize == 0:
            size = 5626
        else:
            size = 5626 + 8 + 1024 * WRAMSize
    #135 Unsupported
    #136 ?
    #137 ?
    #138 ?
    #139 ?
    #140 ?
    elif mapper == 141:
        size = 5551
    #142 ?
    #143 ?
    #144 ?
    #145 ?
    #146 ?
    #147 ?
    #148 ?
    #149 ?
    elif mapper == 150:
        size = 5551
    #151 ?
    #152 ?
    #153 ?
    #154 ?
    elif mapper == 155: # To check (MMC1)
        size = 7133 + 8 + 1024 * 8
    #156 ?
    #157 ?
    #158 ?
    #159 ?
    #160 ?
    #161 Unsupported
    #162 ? (Only unif file format ?)
    elif mapper == 163:
        if ines2header:
            WRAMSize = (PRGRamSize + PRGRamSaveSize)
        else:
            WRAMSize = 8*1024
        size = 13746 + WRAMSize
    #164 ?
    #165 ?
    #166 ?
    #167 ?
    #168 ?
    #169 Unsupported
    #170 ?
    elif mapper == 171: # To check (MMC1)
        size = 7133
    #172 ?
    #173 ?
    #174 Unsupported
    elif mapper == 175:
        size = 5544
    #176 Unsupported : Allocating too much RAM for the G&W
    #177 ?
    #178 ?
    #179 Unsupported
    #180 ?
    #181 ?
    #182 Unsupported
    #183 ?
    #184 ?
    #185 ?
    #186 ?
    #187 ?
    #188 ?
    #189 ?
    #190 ?
    #191 ?
    #192 ?
    #193 ?
    #194 ?
    #195 ?
    #196 ?
    #197 ?
    #198 ?
    #199 ?
    elif mapper == 200:
        size = 5536
    elif mapper == 201:
        size = 5536
    elif mapper == 202:
        size = 5536
    elif mapper == 203:
        size = 5544
    elif mapper == 204:
        size = 5536
    elif mapper == 205:
        size = 13814
    #206 ?
    #207 ?
    #208 ?
    elif mapper == 209:
        if ines2header:
            WRAMSize = PRGRamSize + PRGRamSaveSize
        else:
            WRAMSize = 0
        size = 5714 + WRAMSize
    #210 ?
    elif mapper == 211:
        if ines2header:
            WRAMSize = PRGRamSize + PRGRamSaveSize
        else:
            WRAMSize = 0
        size = 5714 + WRAMSize
    elif mapper == 212:
        size = 5536
    elif mapper == 213:
        size = 5536
    elif mapper == 214:
        size = 5536
    elif mapper == 215:
        size = 5617
    #216 ?
    #217 ?
    #218 ?
    #219 ?
    #220 Unsupported
    elif mapper == 221:
        size = 13746
    #222 ?
    #223 Unsupported
    #224 ?
    elif mapper == 225:
        size = 5574
    elif mapper == 226:
        size = 13745
    elif mapper == 227:
        size = 21936
    #228 ?
    elif mapper == 229:
        size = 5536
    elif mapper == 230:
        size = 13744
    elif mapper == 231:
        size = 13736
    elif mapper == 232:
        size = 13744
    elif mapper == 233:
        size = 13744
    elif mapper == 234:
        size = 5544
    elif mapper == 235:
        size = 13763
    #236 ?
    #237 ?
    #238 ?
    #239 Unsupported
    #240 ?
    elif mapper == 241:
        size = 21944
    #242 ?
    #243 ?
    #244 ?
    #245 ?
    #246 ?
    #247 Unsupported
    #248 Unsupported
    #249 ?
    #250 ?
    #251 Unsupported
    #252 ?
    #253 ?
    #254 ?
    #255 ?
    # NES 2.0 mapper
    #256 ?
    #258 ?
    #...
    #268 Unsupported : Allocating too much RAM for the G&W
    #...
    elif mapper == 281:
        if ines2header:
            WRAMSize = PRGRamSize + PRGRamSaveSize
        else:
            WRAMSize = 0
        size = 5714 + WRAMSize
    elif mapper == 282:
        if ines2header:
            WRAMSize = PRGRamSize + PRGRamSaveSize
        else:
            WRAMSize = 0
        size = 5714 + WRAMSize
    #...
    elif mapper == 295:
        if ines2header:
            WRAMSize = PRGRamSize + PRGRamSaveSize
        else:
            WRAMSize = 0
        size = 5714 + WRAMSize
    #...
    elif mapper == 297: # To check (MMC1)
        size = 7133
    #...
    elif mapper == 358:
        if ines2header:
            WRAMSize = PRGRamSize + PRGRamSaveSize
        else:
            WRAMSize = 0
        size = 5714 + WRAMSize
    #...
    elif mapper == 374: # To check (MMC1)
        size = 7133
    #...
    elif mapper == 386 or mapper == 387 or mapper == 388 :
        if ines2header:
            WRAMSize = PRGRamSize + PRGRamSaveSize
        else:
            WRAMSize = 0
        size = 5714 + WRAMSize
    #...
    elif mapper == 394:
        if ines2header:
            WRAMSize = PRGRamSize + PRGRamSaveSize
        else:
            WRAMSize = 0
        size = 5714 + WRAMSize
    #...
    elif mapper == 397:
        if ines2header:
            WRAMSize = PRGRamSize + PRGRamSaveSize
        else:
            WRAMSize = 0
        size = 5714 + WRAMSize
    #...
    #404 ? (MMC1)
    #...
    elif mapper == 421:
        if ines2header:
            WRAMSize = PRGRamSize + PRGRamSaveSize
        else:
            WRAMSize = 0
        size = 5714 + WRAMSize
    #...
    #543 ? (MMC1)
    #...
    #550 ? (MMC1)
    else:
        size = 24*1024 # 24KB by default
    if size == 0:
        return 0
    return size + 4 # +4 to include G&W specific data

n = len(sys.argv)

if n < 3: print("Usage :\nnesmapper.py [mapper|savesize] file.nes\n"); sys.exit(0)

nesFileName = sys.argv[2]
nesFile = open(nesFileName, 'rb')
# Open file and find its size
mapper,ines2header,PRGRamSize,PRGRamSaveSize,CHRRamSize,CHRRamSaveSize,hasBattery = analyzeRom(nesFile,nesFileName)
nesFile.close()
if sys.argv[1] == "savesize":
    #get save state size
    save = getSaveSize(mapper,os.stat(nesFileName).st_size,ines2header,PRGRamSize,PRGRamSaveSize,CHRRamSize,CHRRamSaveSize,hasBattery)
    save = math.ceil(save/4096)*4096 # round to upper 4kB size (flash block size)
    print(str(save))
elif sys.argv[1] == "mapper":
    print(str(mapper))

    if mapper >= 0:
        addMapper = 1
        mappersFile = "./build/mappers.h"
        # Add mapper #define in file if needed
        mapperString = "{:03d}".format(mapper)
        if os.path.exists(mappersFile):
            mappers = open(mappersFile, 'r')
            if mapperString in mappers.read():
                addMapper = 0
            mappers.close
        if addMapper:
            mappers = open(mappersFile, 'a+')
            mappers.write("#define NES_MAPPER_"+mapperString+"\n")
            mappers.close
    elif mapper == -3: #NSF
        addMapper = 1
        mappersFile = "./build/mappers.h"
        # Add mapper #define in file if needed
        mapperString = "NES_MAPPER_NSF"
        if os.path.exists(mappersFile):
            mappers = open(mappersFile, 'r')
            if mapperString in mappers.read():
                addMapper = 0
            mappers.close
        if addMapper:
            mappers = open(mappersFile, 'a+')
            mappers.write("#define NES_MAPPER_NSF\n")
            mappers.close

else:
    print("unknown command\nUsage :\nnesmapper.py [mapper|savesize] file.nes\n")
sys.exit(0)
