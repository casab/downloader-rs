#
# .zshrc.override.sh
#

# persistent zsh history
HISTFILE=~/.zsh_history
PROMPT_COMMAND="history -a; $PROMPT_COMMAND"

# alias ls="exa --icons -l"

# set some env vars
source /entrypoint

# restore default shell options
set +o errexit
set +o pipefail
set +o nounset

# start ssh-agent
# https://code.visualstudio.com/docs/remote/troubleshooting
eval "$(ssh-agent -s)"
