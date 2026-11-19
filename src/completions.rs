//! Shell completion scripts. They call back into `r __complete <words...>`
//! so task names always reflect the files in the current directory.

pub const SHELLS: [&str; 3] = ["bash", "zsh", "fish"];

pub fn script(shell: &str) -> Option<&'static str> {
    match shell {
        "bash" => Some(BASH),
        "zsh" => Some(ZSH),
        "fish" => Some(FISH),
        _ => None,
    }
}

const BASH: &str = r#"# bash completion for r
# Load with: eval "$(r completions bash)"
_r() {
    # Split the line ourselves: bash treats ':' as a word break, but task
    # names like `build:prod` are common.
    local line="${COMP_LINE:0:COMP_POINT}"
    local cur="${line##*[[:space:]]}"
    local -a words
    read -ra words <<< "$line"
    if [[ -n "$cur" ]]; then
        unset 'words[${#words[@]}-1]'
    fi

    local IFS=$'\n'
    local -a candidates
    candidates=($(r __complete "${words[@]:1}" 2>/dev/null | cut -f1))
    COMPREPLY=($(compgen -W "${candidates[*]}" -- "$cur"))

    # Bash replaces only the part after the last ':', so strip the rest.
    if [[ "$cur" == *:* && "$COMP_WORDBREAKS" == *:* ]]; then
        local prefix="${cur%"${cur##*:}"}"
        local i
        for i in "${!COMPREPLY[@]}"; do
            COMPREPLY[$i]="${COMPREPLY[$i]#"$prefix"}"
        done
    fi
}
complete -o default -F _r r
"#;

const ZSH: &str = r#"#compdef r
# zsh completion for r
# Load with: eval "$(r completions zsh)"
# or save as _r in a directory of your $fpath.
_r() {
    local -a lines candidates
    local line
    lines=("${(@f)$(r __complete "${(@)words[2,CURRENT-1]}" 2>/dev/null)}")
    for line in $lines; do
        [[ -z "$line" ]] && continue
        local name="${line%%$'\t'*}" desc=""
        [[ "$line" == *$'\t'* ]] && desc="${line#*$'\t'}"
        candidates+=("${name//:/\\:}:${desc}")
    done
    if (( ${#candidates} )); then
        _describe -t tasks 'task' candidates
    else
        _files
    fi
}
if [[ "${funcstack[1]}" == "_r" ]]; then
    _r "$@"
else
    compdef _r r
fi
"#;

const FISH: &str = r#"# fish completion for r
# Load with: r completions fish | source
# or save as ~/.config/fish/completions/r.fish
function __r_complete
    set -l words (commandline -opc)
    r __complete $words[2..-1] 2>/dev/null
end
complete -c r -a '(__r_complete)'
"#;
