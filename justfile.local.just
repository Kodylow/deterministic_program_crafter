clean-work-dir:
    rm -rf ./work_dir

run-local cargo-cookie:
    cargo run -- --instructions "simple http server with post endpoints that do basic math" --cargo-cookie {{ cargo-cookie }} --work-dir ./work_dir
