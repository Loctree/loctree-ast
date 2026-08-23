#!/bin/sh

# Clear only the repository-local variables documented by Git. Callers must
# capture their repository root before invoking this function.
loctree_clear_local_git_env() {
  loctree_git_local_vars="$(git rev-parse --local-env-vars)" || return 1

  for loctree_git_local_var in $loctree_git_local_vars; do
    unset "$loctree_git_local_var"
  done

  unset loctree_git_local_var loctree_git_local_vars
}
