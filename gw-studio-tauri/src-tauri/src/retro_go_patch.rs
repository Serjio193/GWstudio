use std::fs;
use std::path::PathBuf;

use crate::toolchain::{locate_git_bin_dir, locate_python_exe, to_bash_path_for};

pub(crate) fn patch_retro_go_workspace_for_windows(workspace_dir: &PathBuf, coverflow_enabled: bool) -> Result<(), String> {
    let makefile_common = workspace_dir.join("Makefile.common");
    let original = fs::read_to_string(&makefile_common)
        .map_err(|error| format!("failed to read {}: {error}", makefile_common.display()))?;
    let git_bin = locate_git_bin_dir().ok_or_else(|| "Git bash/sh not found".to_string())?;
    let python_exe = locate_python_exe();
    let python3_exe_path = to_bash_path_for(&python_exe, &git_bin);
    let python3_make_command = format!(
        "\"{}\" -c \"import sys,runpy; from pathlib import Path; script=sys.argv.pop(1); sys.path.insert(0,str(Path(script).resolve().parent)); sys.path.insert(0,'.'); runpy.run_path(script, run_name='__main__')\"",
        python3_exe_path
    );
    let normalized = original
        .replace("\r\n", "\n")
        .replace("PYTHON3 ?= /usr/bin/env python3", &format!("PYTHON3 ?= {python3_make_command}"))
        .replace(
            "\t$(V)wget -q $(SDK_URL)/$(SDK_VERSION)/$@ -P $(dir $@)",
            "\t$(V)mkdir -p $(dir $@) && curl -L --silent --fail $(SDK_URL)/$(SDK_VERSION)/$@ -o $@",
        )
        .replace("\t$(V)./scripts/", "\t$(V)bash ./scripts/")
        .replace("\t$(V)/bin/sh -c true", "\t$(V)sh -c true");
    let mut patched_lines = Vec::new();
    patched_lines.push("SHELL := bash.exe".to_string());
    let source_prereq_patch = r#"# GW Studio patch: keep same-named emulator sources from colliding through global vpath.
define gw_studio_c_obj_rule
$$(BUILD_DIR)/$(1)/$(notdir $(patsubst %.c,%.o,$(2))): $(2) Makefile.common Makefile $$(SDK_HEADERS) $$(BUILD_DIR)/config.h | $$(BUILD_DIR)
	$$(V)$$(ECHO) [ CC $(3) ] $$(notdir $$<)
	$$(V)$$(CC) -c $$(CFLAGS) $$($(4)) $(5) -Wa,-a,-ad,-alms=$$(BUILD_DIR)/$(1)/$$(notdir $$(<:.c=.lst)) $$< -o $$@

endef
$(eval $(foreach obj,$(NES_C_SOURCES),$(call gw_studio_c_obj_rule,nes,$(obj),nes,NES_C_INCLUDES,)))
$(eval $(foreach obj,$(NES_FCEU_C_SOURCES),$(call gw_studio_c_obj_rule,nes_fceu,$(obj),nes-fceu,NES_FCEU_C_INCLUDES,-Wno-sequence-point -Wno-parentheses)))
$(eval $(foreach obj,$(GNUBOY_C_SOURCES),$(call gw_studio_c_obj_rule,gnuboy,$(obj),gb,GNUBOY_C_INCLUDES,)))
$(eval $(foreach obj,$(SMSPLUSGX_C_SOURCES),$(call gw_studio_c_obj_rule,smsplusgx,$(obj),sms,SMSPLUSGX_C_INCLUDES,)))
$(eval $(foreach obj,$(PCE_C_SOURCES),$(call gw_studio_c_obj_rule,pce,$(obj),pce,PCE_C_INCLUDES,)))
$(eval $(foreach obj,$(GW_C_SOURCES),$(call gw_studio_c_obj_rule,gw,$(obj),gw,GW_C_INCLUDES,)))
$(eval $(foreach obj,$(MSX_C_SOURCES),$(call gw_studio_c_obj_rule,msx,$(obj),msx,MSX_C_INCLUDES,)))
$(eval $(foreach obj,$(WSV_C_SOURCES),$(call gw_studio_c_obj_rule,wsv,$(obj),wsv,WSV_C_INCLUDES,)))
$(eval $(foreach obj,$(MD_C_SOURCES),$(call gw_studio_c_obj_rule,md,$(obj),md,MD_C_INCLUDES,)))
$(eval $(foreach obj,$(A7800_C_SOURCES),$(call gw_studio_c_obj_rule,a7800,$(obj),a7800,A7800_C_INCLUDES,)))
$(eval $(foreach obj,$(TAMA_C_SOURCES),$(call gw_studio_c_obj_rule,tama,$(obj),tama,TAMA_C_INCLUDES,)))
"#;

    let mut lines = normalized.lines().peekable();
    while let Some(line) = lines.next() {
        if line == "# generate all object prerequisite rules" {
            patched_lines.extend(source_prereq_patch.lines().map(|line| line.to_string()));
        }
        if line == "$(BUILD_DIR):" {
            patched_lines.push(line.to_string());
            while let Some(next_line) = lines.peek() {
                if next_line.starts_with('\t') || next_line.trim().is_empty() {
                    lines.next();
                    continue;
                }
                break;
            }
            patched_lines.push("\t$(V)mkdir -p $@/core $@/nes $@/nes_fceu $@/gnuboy $@/smsplusgx $@/pce $@/gw $@/msx $@/wsv $@/md $@/a7800 $@/tama".to_string());
            continue;
        }
        patched_lines.push(line.to_string());
    }

    fs::write(&makefile_common, patched_lines.join("\n"))
        .map_err(|error| format!("failed to write {}: {error}", makefile_common.display()))?;

    if coverflow_enabled {
        let rg_main_c = workspace_dir.join("Core").join("Src").join("retro-go").join("rg_main.c");
        let rg_main_original = fs::read_to_string(&rg_main_c)
            .map_err(|error| format!("failed to read {}: {error}", rg_main_c.display()))?;
        let rg_main_patched = rg_main_original
            .replace("\r\n", "\n")
            .replace("    // gui.show_cover = odroid_settings_int32_get(KEY_SHOW_COVER, 1);", "    gui.show_cover = 1;");
        if rg_main_patched != rg_main_original {
            fs::write(&rg_main_c, rg_main_patched)
                .map_err(|error| format!("failed to write {}: {error}", rg_main_c.display()))?;
        }
    }

    let rg_emulators_c = workspace_dir.join("Core").join("Src").join("retro-go").join("rg_emulators.c");
    if rg_emulators_c.exists() {
        let rg_emulators_original = fs::read_to_string(&rg_emulators_c)
            .map_err(|error| format!("failed to read {}: {error}", rg_emulators_c.display()))?;
        let rg_emulators_patched = rg_emulators_original
            .replace("\r\n", "\n")
            .replace("#include \"main_amstrad.h\"\n", "");
        if rg_emulators_patched != rg_emulators_original {
            fs::write(&rg_emulators_c, rg_emulators_patched)
                .map_err(|error| format!("failed to write {}: {error}", rg_emulators_c.display()))?;
        }
    }

    let extflash_size_script = workspace_dir.join("scripts").join("extflash_size.sh");
let extflash_size_script_patched = r#"#!/bin/bash
# Usage: ./extflash_size.sh app.elf

export LC_ALL=C

if [[ "${GCC_PATH}" != "" ]]; then
	DEFAULT_OBJDUMP=${GCC_PATH}/arm-none-eabi-objdump
else
	DEFAULT_OBJDUMP=arm-none-eabi-objdump
fi

OBJDUMP=${OBJDUMP:-$DEFAULT_OBJDUMP}

elf_file=$1

function get_symbol {
	name=$1
	size=$("$OBJDUMP" -t "$elf_file" | awk -v n="$name" '$NF == n {print toupper($1); exit}')
	if [[ -z "$size" ]]; then
		echo "Missing symbol: $name" >&2
		exit 1
	fi
	printf "%d\n" "$((16#$size))"
}

function get_section_length {
	name=$1
	start=$(get_symbol "__${name}_start__")
	end=$(get_symbol "__${name}_end__")
	echo $(( end - start ))
}

function print_usage {
	symbol=$1
	length_symbol=$2
	usage=$(get_section_length $symbol)
	usagemb=$(printf "%.3f" "$(( (usage * 1000000) / 1024 / 1024 ))e-6")
	length=$(get_symbol $length_symbol)
	lengthmb=$(printf "%.3f" "$(( (length * 1000000) / 1024 / 1024 ))e-6")
	free=$(( length - usage ))
	freemb=$(printf "%.3f" "$(( (free * 1000000) / 1024 / 1024 ))e-6")
	echo -e ""
	echo -e "External flash usage"
	printf  "    Capacity: %12d Bytes (%7.3f MB)\n" $length $lengthmb
	printf  "    Usage:    %12d Bytes (%7.3f MB)\n" $usage $usagemb
	printf  "    Free:     %12d Bytes (%7.3f MB)\n" $free $freemb
	echo -e ""
}

print_usage extflash __EXTFLASH_LENGTH__
"#;
    fs::write(&extflash_size_script, extflash_size_script_patched)
        .map_err(|error| format!("failed to write {}: {error}", extflash_size_script.display()))?;
    Ok(())
}
