fix:
    cargo clippy --fix --allow-dirty --allow-staged --all-targets -- -D warnings

fmt:
    just fix
    cargo fmt --all

test:
    cargo test

e2e:
    cargo test --test real_e2e -- --ignored --nocapture
