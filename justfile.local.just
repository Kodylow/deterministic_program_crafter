cleanlocal:
    rm -rf ./work_dir

run-local cargo-cookie:
    cargo run -- --instructions "simple http server with a post endpoint for finding the sum of two numbers" --cargo-cookie {{ cargo-cookie }} --work-dir ./work_dir
