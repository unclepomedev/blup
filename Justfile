fix:
    cargo clippy --fix --allow-dirty --allow-staged --all-targets -- -D warnings

fmt:
    just fix
    cargo fmt --all
