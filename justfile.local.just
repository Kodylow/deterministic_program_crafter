cleanlocal:
    rm -rf ./work_dir

run-local cargo-cookie:
    cargo run -- --instructions "simple http server with post endpoints for basic math" --cargo-cookie {{ cargo-cookie }} --work-dir ./work_dir

rl cargo-cookie:
    cargo run -- --instructions "simple http server with a post endpoint for summing two numbers" --cargo-cookie {{ cargo-cookie }} --work-dir ./work_dir
