{#
    NOTE: this is a jinja template, rendered in completion_script.rs
#}
# functions to avoid clobbering any other completions for cargo
function __find_fish_builtin_completiondir \
    -d "Try to find the fish completions datadir in a thorough manner.
    Upon failure, prints nothing (i.e., not even an empty string)."

    if command -q pkg-config;
            and set -l result (pkg-config fish --variable datadir)/fish/completions;
            and test -d $result
        echo $result
    else if set -q __fish_data_dir
        echo $__fish_data_dir/fish/completions
    else
        return 1
    end
end

function __load_lazy_completions \
    -a cmd \
    -d "attempt to manually activate completions for the provided command"

    # Make extra, super-duper sure this script does not try to source itself
    # (and exit immediately if it does)
    if test "$_CARGO_PG_COMPLETION_ACTIVATING" = "$cmd"
        return 1
    else
        set -gx _CARGO_PG_COMPLETION_ACTIVATING "$cmd"
    end

    # Try to source any existing autocompletion script for this command
    for dirpath in $fish_complete_path (__find_fish_builtin_completiondir)
        set -l script "$dirpath/$cmd.fish"
        if test -f $script;
                and not test $script -ef (status --current-file)
            source $script
            break
        end
    end

    set -eg _CARGO_PG_COMPLETION_ACTIVATING
end

__load_lazy_completions "{{cmd}}"

{{clap_completion_script}}

# clean up
functions -e __load_lazy_completions
functions -e __fish_builtin_completiondir
